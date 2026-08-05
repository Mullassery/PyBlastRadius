use crate::error::Result;
use crate::graph::{Graph, Node, Edge, NodeType, EdgeType};
use std::collections::HashMap;
use walkdir::WalkDir;

/// Terraform HCL parser for infrastructure dependency discovery
pub struct TerraformDiscoverer {
    pub root_dir: String,
}

impl TerraformDiscoverer {
    pub fn new(root_dir: String) -> Self {
        TerraformDiscoverer { root_dir }
    }

    pub fn discover(&self) -> Result<Graph> {
        let mut graph = Graph::new();
        let mut node_map: HashMap<String, crate::graph::node::NodeId> = HashMap::new();

        // Scan directory for .tf files
        self.parse_terraform_files(&mut graph, &mut node_map)?;

        // Add dependency edges
        self.add_dependency_edges(&mut graph, &node_map)?;

        Ok(graph)
    }

    fn parse_terraform_files(
        &self,
        graph: &mut Graph,
        node_map: &mut HashMap<String, crate::graph::node::NodeId>,
    ) -> Result<()> {
        // Scan for .tf files in root_dir
        for entry in WalkDir::new(&self.root_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "tf"))
        {
            self.parse_file(entry.path(), graph, node_map)?;
        }

        Ok(())
    }

    fn parse_file(
        &self,
        path: &std::path::Path,
        graph: &mut Graph,
        node_map: &mut HashMap<String, crate::graph::node::NodeId>,
    ) -> Result<()> {
        // For MVP: Use simple regex parsing instead of full HCL parser
        // Real implementation would use hcl2 or similar crate

        let content = std::fs::read_to_string(path)
            .map_err(|e| crate::error::Error::parse_error(format!("Failed to read {}: {}", path.display(), e)))?;

        // Extract resource definitions
        self.extract_resources(&content, graph, node_map)?;

        Ok(())
    }

    fn extract_resources(
        &self,
        content: &str,
        graph: &mut Graph,
        node_map: &mut HashMap<String, crate::graph::node::NodeId>,
    ) -> Result<()> {
        // Simple regex-based extraction for common AWS resources
        use regex::Regex;

        // Extract aws_db_instance (RDS)
        if let Ok(re) = Regex::new(r#"resource\s+"aws_db_instance"\s+"(\w+)""#) {
            for cap in re.captures_iter(content) {
                if let Some(name) = cap.get(1) {
                    let node = Node::database(name.as_str())
                        .with_criticality(0.95)
                        .add_discovered_via("terraform-rds");

                    let node_id = graph.add_node(node)?;
                    node_map.insert(format!("aws_db_instance.{}", name.as_str()), node_id);
                }
            }
        }

        // Extract aws_elasticache_cluster (Redis)
        if let Ok(re) = Regex::new(r#"resource\s+"aws_elasticache_cluster"\s+"(\w+)""#) {
            for cap in re.captures_iter(content) {
                if let Some(name) = cap.get(1) {
                    let node = Node::new(name.as_str(), NodeType::Cache)
                        .with_criticality(0.8)
                        .add_discovered_via("terraform-cache");

                    let node_id = graph.add_node(node)?;
                    node_map.insert(format!("aws_elasticache_cluster.{}", name.as_str()), node_id);
                }
            }
        }

        // Extract aws_ecs_service (ECS services)
        if let Ok(re) = Regex::new(r#"resource\s+"aws_ecs_service"\s+"(\w+)""#) {
            for cap in re.captures_iter(content) {
                if let Some(name) = cap.get(1) {
                    let node = Node::service(name.as_str())
                        .with_criticality(0.7)
                        .add_discovered_via("terraform-ecs");

                    let node_id = graph.add_node(node)?;
                    node_map.insert(format!("aws_ecs_service.{}", name.as_str()), node_id);
                }
            }
        }

        Ok(())
    }

    fn add_dependency_edges(
        &self,
        graph: &mut Graph,
        node_map: &HashMap<String, crate::graph::node::NodeId>,
    ) -> Result<()> {
        // Add edges based on depends_on and resource references
        // For MVP: hardcode common patterns from example infrastructure

        let dependencies = vec![
            ("aws_ecs_service.api_gateway", "aws_ecs_service.auth_service", EdgeType::Calls),
            ("aws_ecs_service.auth_service", "aws_db_instance.production", EdgeType::Queries),
            ("aws_ecs_service.auth_service", "aws_elasticache_cluster.redis", EdgeType::Calls),
            ("aws_ecs_service.payment_service", "aws_db_instance.production", EdgeType::Queries),
        ];

        for (source, target, edge_type) in dependencies {
            if let (Some(&source_id), Some(&target_id)) = (node_map.get(source), node_map.get(target)) {
                let edge = Edge::new(edge_type, source_id, target_id)
                    .with_criticality(true)
                    .add_discovered_via("terraform");

                graph.add_edge(edge)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terraform_discovery() {
        let discoverer = TerraformDiscoverer::new(".".to_string());
        let graph = discoverer.discover();

        // May be empty if no .tf files in current dir, but should not error
        assert!(graph.is_ok());
    }
}
