use crate::error::Result;
use crate::graph::{Edge, EdgeType, Graph, Node, NodeType};
use std::collections::HashMap;
use walkdir::WalkDir;

/// Terraform HCL parser for infrastructure dependency discovery.
///
/// Uses `hcl-rs`, a real HCL2 parser, instead of the regex-based extraction
/// this previously did -- regex cannot correctly handle multi-line blocks,
/// nested blocks, or comments containing lookalike resource declarations,
/// all of which are common in real Terraform. Resource *types* are no
/// longer limited to 3 hardcoded AWS types either: any `resource "type"
/// "name" { ... }` block is discovered.
///
/// Dependency edges are derived from real resource references found in each
/// resource's own attribute values (Terraform's own implicit-dependency
/// mechanism -- e.g. `db_endpoint = aws_db_instance.production.endpoint` or
/// `"postgres://${aws_db_instance.production.endpoint}"` both reference
/// `aws_db_instance.production`), not from a hardcoded example dependency
/// list. A resource genuinely referencing another resource id is the only
/// thing that produces an edge here.
pub struct TerraformDiscoverer {
    pub root_dir: String,
}

/// A resource block's classification, its raw HCL body (kept around so
/// dependency edges can be derived from it after every resource has been
/// registered in `node_map` -- a resource can reference one declared
/// earlier *or* later in file/directory scan order).
struct ParsedResource {
    address: String, // "{type}.{name}"
    node_type: NodeType,
    criticality: f32,
    discovered_via: &'static str,
    body: hcl::Body,
}

impl TerraformDiscoverer {
    pub fn new(root_dir: String) -> Self {
        TerraformDiscoverer { root_dir }
    }

    pub fn discover(&self) -> Result<Graph> {
        let mut graph = Graph::new();
        let mut node_map: HashMap<String, crate::graph::node::NodeId> = HashMap::new();

        let resources = self.collect_resources()?;

        for resource in &resources {
            let node = Node::new(
                resource_short_name(&resource.address),
                resource.node_type.clone(),
            )
            .with_criticality(resource.criticality)
            .add_discovered_via(resource.discovered_via);
            let node_id = graph.add_node(node)?;
            node_map.insert(resource.address.clone(), node_id);
        }

        self.add_dependency_edges(&mut graph, &node_map, &resources)?;

        Ok(graph)
    }

    fn collect_resources(&self) -> Result<Vec<ParsedResource>> {
        let mut resources = Vec::new();

        for entry in WalkDir::new(&self.root_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "tf"))
        {
            let content = std::fs::read_to_string(entry.path()).map_err(|e| {
                crate::error::Error::parse_error(format!(
                    "Failed to read {}: {}",
                    entry.path().display(),
                    e
                ))
            })?;

            let body: hcl::Body = hcl::from_str(&content).map_err(|e| {
                crate::error::Error::parse_error(format!(
                    "Failed to parse HCL in {}: {}",
                    entry.path().display(),
                    e
                ))
            })?;

            for structure in body.into_iter() {
                let hcl::Structure::Block(block) = structure else {
                    continue;
                };
                if block.identifier() != "resource" {
                    continue;
                }
                let [resource_type, resource_name] = block.labels() else {
                    continue;
                };
                let (node_type, criticality, discovered_via) =
                    classify_resource_type(resource_type.as_str());

                resources.push(ParsedResource {
                    address: format!("{}.{}", resource_type.as_str(), resource_name.as_str()),
                    node_type,
                    criticality,
                    discovered_via,
                    body: block.body().clone(),
                });
            }
        }

        Ok(resources)
    }

    /// Scan every resource's own attribute values for references to other
    /// known resource addresses (`node_map`'s keys), adding a `DependsOn`
    /// edge for each real reference found. Self-references are skipped.
    fn add_dependency_edges(
        &self,
        graph: &mut Graph,
        node_map: &HashMap<String, crate::graph::node::NodeId>,
        resources: &[ParsedResource],
    ) -> Result<()> {
        for resource in resources {
            let Some(&source_id) = node_map.get(&resource.address) else {
                continue;
            };

            for referenced_address in references_in_body(&resource.body, node_map.keys()) {
                if referenced_address == resource.address {
                    continue; // a resource cannot depend on itself
                }
                let Some(&target_id) = node_map.get(&referenced_address) else {
                    continue;
                };

                let edge = Edge::new(EdgeType::DependsOn, source_id, target_id)
                    .with_criticality(true)
                    .add_discovered_via("terraform-hcl-reference");
                graph.add_edge(edge)?;
            }
        }

        Ok(())
    }
}

/// The short display name for a resource address ("aws_db_instance.production"
/// -> "production") -- the type prefix is already captured via `discovered_via`
/// and the node type, so the node's own name stays the human-meaningful part.
fn resource_short_name(address: &str) -> &str {
    address.split('.').nth(1).unwrap_or(address)
}

/// Classify a Terraform resource type string into a graph `NodeType`, a
/// default criticality, and a `discovered_via` tag. Falls back to `Service`
/// for unrecognized types rather than silently dropping the resource --
/// every resource block found becomes a node.
fn classify_resource_type(resource_type: &str) -> (NodeType, f32, &'static str) {
    let t = resource_type;
    if t.contains("db_instance") || t.contains("rds") || t.contains("dynamodb_table") {
        (NodeType::Database, 0.95, "terraform-database")
    } else if t.contains("elasticache") || t.contains("redis") || t.contains("memcache") {
        (NodeType::Cache, 0.8, "terraform-cache")
    } else if t.contains("sqs")
        || t.contains("sns")
        || t.contains("kinesis")
        || t.contains("mq_broker")
    {
        (NodeType::Queue, 0.7, "terraform-queue")
    } else if t.contains("s3_bucket") || t.contains("_table") {
        (NodeType::Table, 0.6, "terraform-storage")
    } else if t.contains("ecs_service")
        || t.contains("lambda_function")
        || t.contains("ec2_instance")
        || t.contains("eks_")
        || t.contains("ecs_task")
        || t.contains("fargate")
        || t.contains("api_gateway")
        || t.contains("lb")
        || t.contains("elb")
    {
        (NodeType::Service, 0.7, "terraform-service")
    } else {
        (NodeType::Service, 0.5, "terraform-resource")
    }
}

/// Find every `known_address` (from `candidates`) referenced anywhere in
/// `body`'s attribute values -- both bare traversal expressions
/// (`aws_db_instance.production.endpoint`, valid directly as a value) and
/// interpolated strings (`"${aws_db_instance.production.endpoint}"`).
/// Recurses into nested blocks (e.g. a resource's `lifecycle`/`provisioner`
/// sub-blocks) since references there are just as real.
fn references_in_body<'a>(
    body: &hcl::Body,
    candidates: impl Iterator<Item = &'a String> + Clone,
) -> Vec<String> {
    let mut found = Vec::new();
    for structure in body.iter() {
        match structure {
            hcl::Structure::Attribute(attr) => {
                collect_references_in_expr(&attr.expr, candidates.clone(), &mut found);
            }
            hcl::Structure::Block(nested) => {
                found.extend(references_in_body(nested.body(), candidates.clone()));
            }
        }
    }
    found.sort();
    found.dedup();
    found
}

/// A resource address followed by a `TraversalOperator` chain looks like
/// `aws_db_instance.production.endpoint` -- the reference is the first two
/// path segments (`aws_db_instance.production`); everything after that is
/// an attribute access *on* the referenced resource, not part of its
/// address. This matches the same address format `known_addresses` keys
/// (and `TerraformDiscoverer::add_dependency_edges` builds node_map from).
fn collect_references_in_expr<'a>(
    expr: &hcl::Expression,
    candidates: impl Iterator<Item = &'a String> + Clone,
    out: &mut Vec<String>,
) {
    // hcl-rs's Display impl round-trips an Expression back to HCL source,
    // including the literal "${...}" form for template interpolations --
    // so a single text scan over that (rather than hand-walking every
    // Expression variant, several of which can themselves contain
    // interpolated templates) reliably catches both bare traversals and
    // interpolated references without missing either form.
    let text = expr.to_string();
    for candidate in candidates {
        if text_references_address(&text, candidate) {
            out.push(candidate.clone());
        }
    }
}

/// True if `text` contains `address` (e.g. "aws_db_instance.production") as
/// a real reference -- i.e. not immediately preceded/followed by another
/// identifier character, so "aws_db_instance.production_replica" doesn't
/// false-positive as a reference to "aws_db_instance.production".
fn text_references_address(text: &str, address: &str) -> bool {
    let mut search_start = 0;
    while let Some(rel_idx) = text[search_start..].find(address) {
        let idx = search_start + rel_idx;
        let before_ok = idx == 0
            || !text.as_bytes()[idx - 1].is_ascii_alphanumeric()
                && text.as_bytes()[idx - 1] != b'_';
        let after_idx = idx + address.len();
        let after_ok = after_idx >= text.len()
            || !(text.as_bytes()[after_idx].is_ascii_alphanumeric()
                || text.as_bytes()[after_idx] == b'_');
        if before_ok && after_ok {
            return true;
        }
        search_start = idx + 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_tf(dir: &TempDir, filename: &str, content: &str) {
        fs::write(dir.path().join(filename), content).unwrap();
    }

    #[test]
    fn test_terraform_discovery_on_empty_dir_is_empty_not_an_error() {
        let dir = TempDir::new().unwrap();
        let discoverer = TerraformDiscoverer::new(dir.path().to_string_lossy().to_string());
        let graph = discoverer.discover().unwrap();
        assert_eq!(graph.node_count(), 0);
    }

    #[test]
    fn test_discovers_any_resource_type_not_just_the_three_hardcoded_ones() {
        let dir = TempDir::new().unwrap();
        write_tf(
            &dir,
            "main.tf",
            r#"
resource "aws_s3_bucket" "artifacts" {
  bucket = "my-artifacts"
}

resource "aws_lambda_function" "processor" {
  function_name = "processor"
}
"#,
        );

        let discoverer = TerraformDiscoverer::new(dir.path().to_string_lossy().to_string());
        let graph = discoverer.discover().unwrap();

        assert_eq!(graph.node_count(), 2);
        assert!(graph.nodes().any(|n| n.name == "artifacts"));
        assert!(graph.nodes().any(|n| n.name == "processor"));
    }

    #[test]
    fn test_real_reference_produces_a_real_edge_bare_traversal() {
        let dir = TempDir::new().unwrap();
        write_tf(
            &dir,
            "main.tf",
            r#"
resource "aws_db_instance" "production" {
  engine = "postgres"
}

resource "aws_ecs_service" "auth_service" {
  name = "auth"
  environment {
    db_endpoint = aws_db_instance.production.endpoint
  }
}
"#,
        );

        let discoverer = TerraformDiscoverer::new(dir.path().to_string_lossy().to_string());
        let graph = discoverer.discover().unwrap();

        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);
        let auth = graph.nodes().find(|n| n.name == "auth_service").unwrap();
        let db = graph.nodes().find(|n| n.name == "production").unwrap();
        assert!(graph.has_path(&auth.id, &db.id).unwrap());
    }

    #[test]
    fn test_real_reference_produces_a_real_edge_interpolated_string() {
        let dir = TempDir::new().unwrap();
        write_tf(
            &dir,
            "main.tf",
            r#"
resource "aws_db_instance" "production" {
  engine = "postgres"
}

resource "aws_ecs_service" "payment_service" {
  name = "payment"
  environment {
    connection_string = "postgres://${aws_db_instance.production.endpoint}/payments"
  }
}
"#,
        );

        let discoverer = TerraformDiscoverer::new(dir.path().to_string_lossy().to_string());
        let graph = discoverer.discover().unwrap();

        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn test_similar_but_distinct_resource_name_is_not_a_false_positive() {
        let dir = TempDir::new().unwrap();
        write_tf(
            &dir,
            "main.tf",
            r#"
resource "aws_db_instance" "production" {
  engine = "postgres"
}

resource "aws_db_instance" "production_replica" {
  engine = "postgres"
}

resource "aws_ecs_service" "auth_service" {
  name = "auth"
  environment {
    db_endpoint = aws_db_instance.production_replica.endpoint
  }
}
"#,
        );

        let discoverer = TerraformDiscoverer::new(dir.path().to_string_lossy().to_string());
        let graph = discoverer.discover().unwrap();

        // auth_service references production_replica specifically -- must
        // not also match "production" as a substring false positive.
        assert_eq!(graph.edge_count(), 1);
        let auth = graph.nodes().find(|n| n.name == "auth_service").unwrap();
        let replica = graph
            .nodes()
            .find(|n| n.name == "production_replica")
            .unwrap();
        let production = graph.nodes().find(|n| n.name == "production").unwrap();
        assert!(graph.has_path(&auth.id, &replica.id).unwrap());
        assert!(!graph.has_path(&auth.id, &production.id).unwrap());
    }

    #[test]
    fn test_no_reference_means_no_edge_not_a_hardcoded_guess() {
        let dir = TempDir::new().unwrap();
        write_tf(
            &dir,
            "main.tf",
            r#"
resource "aws_db_instance" "production" {
  engine = "postgres"
}

resource "aws_ecs_service" "unrelated_service" {
  name = "unrelated"
}
"#,
        );

        let discoverer = TerraformDiscoverer::new(dir.path().to_string_lossy().to_string());
        let graph = discoverer.discover().unwrap();

        // Previously this exact resource-name combination ("aws_ecs_service"
        // with no matching name) would have produced zero edges too, but for
        // the wrong reason (a hardcoded list that just didn't happen to
        // mention these names) -- this graph must have no edges because
        // there is genuinely no reference between them, which is now
        // actually verified, not incidental.
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_multiline_and_comment_do_not_confuse_the_real_parser() {
        // A regex-based extractor can be fooled by a resource-looking
        // string inside a comment; a real HCL parser cannot.
        let dir = TempDir::new().unwrap();
        write_tf(
            &dir,
            "main.tf",
            r#"
# resource "aws_db_instance" "fake_commented_out" {
#   engine = "postgres"
# }

resource "aws_db_instance" "real_one" {
  engine = "postgres"
  tags = {
    Name = "multi
line
value"
  }
}
"#,
        );

        let discoverer = TerraformDiscoverer::new(dir.path().to_string_lossy().to_string());
        let graph = discoverer.discover().unwrap();

        assert_eq!(graph.node_count(), 1);
        assert!(graph.nodes().any(|n| n.name == "real_one"));
        assert!(!graph.nodes().any(|n| n.name == "fake_commented_out"));
    }
}
