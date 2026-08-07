use crate::error::Result;
use crate::graph::{Edge, EdgeType, Graph, Node, NodeType};
use serde::Deserialize;
use std::collections::HashMap;
use walkdir::WalkDir;

/// Kubernetes manifest parser for dependency discovery.
///
/// Parses real Service/Deployment/StatefulSet manifests found under
/// `manifests_dir` (YAML/YML files, `---`-separated multi-document files
/// supported). Edges are inferred by scanning container env var values for
/// references to other discovered workload names — the same "MVP regex/
/// heuristic, not a full schema-aware parser" quality bar this project
/// already uses for Terraform discovery, not a live cluster API client.
pub struct KubernetesDiscoverer {
    pub kubeconfig_path: Option<String>,
    pub context: Option<String>,
    pub namespaces: Vec<String>,
    pub manifests_dir: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct K8sManifest {
    kind: Option<String>,
    metadata: Option<K8sMetadata>,
    spec: Option<serde_yaml::Value>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct K8sMetadata {
    name: Option<String>,
    namespace: Option<String>,
}

impl KubernetesDiscoverer {
    pub fn new() -> Self {
        KubernetesDiscoverer {
            kubeconfig_path: None,
            context: None,
            namespaces: vec!["default".to_string()],
            manifests_dir: None,
        }
    }

    /// Discover from a directory of Kubernetes YAML manifests (the mode the
    /// CLI's `--kubernetes <path>` flag uses).
    pub fn from_manifests_dir(dir: impl Into<String>) -> Self {
        KubernetesDiscoverer {
            manifests_dir: Some(dir.into()),
            ..Self::new()
        }
    }

    pub fn with_namespaces(mut self, namespaces: Vec<String>) -> Self {
        self.namespaces = namespaces;
        self
    }

    pub fn discover(&self) -> Result<Graph> {
        let mut graph = Graph::new();

        let Some(dir) = &self.manifests_dir else {
            // No manifests directory configured (e.g. the legacy
            // kubeconfig-based live-cluster mode was never implemented) —
            // an empty graph is the honest result, not fabricated demo data.
            return Ok(graph);
        };

        let mut node_map: HashMap<String, crate::graph::node::NodeId> = HashMap::new();
        let mut env_values: HashMap<String, Vec<String>> = HashMap::new();

        for entry in WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map_or(false, |ext| ext == "yaml" || ext == "yml")
            })
        {
            self.parse_file(entry.path(), &mut graph, &mut node_map, &mut env_values)?;
        }

        self.infer_edges_from_env(&mut graph, &node_map, &env_values)?;

        Ok(graph)
    }

    fn parse_file(
        &self,
        path: &std::path::Path,
        graph: &mut Graph,
        node_map: &mut HashMap<String, crate::graph::node::NodeId>,
        env_values: &mut HashMap<String, Vec<String>>,
    ) -> Result<()> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            crate::error::Error::parse_error(format!("Failed to read {}: {}", path.display(), e))
        })?;

        for document in serde_yaml::Deserializer::from_str(&content) {
            let manifest = match K8sManifest::deserialize(document) {
                Ok(m) => m,
                Err(_) => continue, // skip non-manifest / malformed documents rather than aborting the whole scan
            };
            self.process_manifest(manifest, graph, node_map, env_values)?;
        }

        Ok(())
    }

    fn process_manifest(
        &self,
        manifest: K8sManifest,
        graph: &mut Graph,
        node_map: &mut HashMap<String, crate::graph::node::NodeId>,
        env_values: &mut HashMap<String, Vec<String>>,
    ) -> Result<()> {
        let Some(kind) = manifest.kind.as_deref() else {
            return Ok(());
        };
        let Some(name) = manifest.metadata.as_ref().and_then(|m| m.name.clone()) else {
            return Ok(());
        };
        let namespace = manifest
            .metadata
            .as_ref()
            .and_then(|m| m.namespace.clone())
            .unwrap_or_else(|| "default".to_string());
        let key = format!("{}.{}", name, namespace);

        let node_type = match kind {
            "Service" | "Deployment" | "StatefulSet" | "DaemonSet" => {
                // A Service fronting a datastore StatefulSet is commonly
                // named after it (e.g. a "postgres" Service in front of a
                // "postgres" StatefulSet) — classify by name regardless of
                // which kind declared it, not just StatefulSet.
                if Self::looks_like_datastore(&name) {
                    Self::datastore_type(&name)
                } else {
                    NodeType::Service
                }
            }
            _ => return Ok(()), // ConfigMap, Ingress, etc. aren't dependency nodes on their own
        };

        if !node_map.contains_key(&key) {
            let criticality = if matches!(node_type, NodeType::Database) {
                0.95
            } else {
                0.7
            };
            let node = Node::new(&key, node_type)
                .with_criticality(criticality)
                .add_discovered_via(format!("kubernetes-{}", kind.to_lowercase()));
            let node_id = graph.add_node(node)?;
            node_map.insert(key.clone(), node_id);
        }

        if let Some(spec) = &manifest.spec {
            let values = Self::extract_env_values(spec);
            if !values.is_empty() {
                env_values.entry(key).or_default().extend(values);
            }
        }

        Ok(())
    }

    /// Common datastore StatefulSet names get a more specific node type;
    /// anything else defaults to a generic Service (still real, just less
    /// specifically classified without deeper image/label inspection).
    fn looks_like_datastore(name: &str) -> bool {
        ["postgres", "postgresql", "mysql", "redis", "kafka", "mongodb", "cassandra"]
            .iter()
            .any(|d| name.to_lowercase().contains(d))
    }

    fn datastore_type(name: &str) -> NodeType {
        let lower = name.to_lowercase();
        if lower.contains("redis") {
            NodeType::Cache
        } else if lower.contains("kafka") {
            NodeType::Topic
        } else {
            NodeType::Database
        }
    }

    /// Pulls every container env var value out of a Deployment/StatefulSet
    /// spec (`spec.template.spec.containers[].env[].value`), or a Service
    /// spec (no-op there — services don't carry env vars). Used to infer
    /// which other workloads a container references by name.
    fn extract_env_values(spec: &serde_yaml::Value) -> Vec<String> {
        let mut values = Vec::new();
        let containers = spec
            .get("template")
            .and_then(|t| t.get("spec"))
            .and_then(|s| s.get("containers"))
            .or_else(|| spec.get("containers"))
            .and_then(|c| c.as_sequence());

        let Some(containers) = containers else {
            return values;
        };

        for container in containers {
            if let Some(env) = container.get("env").and_then(|e| e.as_sequence()) {
                for var in env {
                    if let Some(value) = var.get("value").and_then(|v| v.as_str()) {
                        values.push(value.to_string());
                    }
                }
            }
        }

        values
    }

    fn infer_edges_from_env(
        &self,
        graph: &mut Graph,
        node_map: &HashMap<String, crate::graph::node::NodeId>,
        env_values: &HashMap<String, Vec<String>>,
    ) -> Result<()> {
        for (source_key, values) in env_values {
            let Some(&source_id) = node_map.get(source_key) else {
                continue;
            };
            for (target_key, &target_id) in node_map {
                if target_key == source_key {
                    continue;
                }
                let target_short_name = target_key.split('.').next().unwrap_or(target_key);
                let referenced = values
                    .iter()
                    .any(|v| v.to_lowercase().contains(&target_short_name.to_lowercase()));
                if referenced {
                    let target_node = graph.get_node(&target_id);
                    let edge_type = match target_node.map(|n| &n.node_type) {
                        Some(NodeType::Database) => EdgeType::Queries,
                        Some(NodeType::Topic) => EdgeType::Produces,
                        _ => EdgeType::Calls,
                    };
                    let edge = Edge::new(edge_type, source_id, target_id)
                        .with_criticality(true)
                        .with_confidence(0.6) // env-var substring match, not a verified call graph
                        .add_discovered_via("kubernetes-env-reference");
                    graph.add_edge(edge)?;
                }
            }
        }
        Ok(())
    }
}

impl Default for KubernetesDiscoverer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kubernetes_discovery_without_manifests_dir_is_honestly_empty() {
        let discoverer = KubernetesDiscoverer::new();
        let graph = discoverer.discover().unwrap();

        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_kubernetes_discovery_parses_real_service_manifest() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("service.yaml"),
            r#"
apiVersion: v1
kind: Service
metadata:
  name: api-gateway
  namespace: prod
spec:
  ports:
    - port: 80
"#,
        )
        .unwrap();

        let discoverer = KubernetesDiscoverer::from_manifests_dir(dir.path().to_str().unwrap());
        let graph = discoverer.discover().unwrap();

        assert_eq!(graph.node_count(), 1);
        assert_eq!(graph.nodes().next().unwrap().name, "api-gateway.prod");
    }

    #[test]
    fn test_kubernetes_discovery_infers_edges_from_env_vars() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("manifests.yaml"),
            r#"
apiVersion: v1
kind: Service
metadata:
  name: postgres
  namespace: default
spec:
  ports:
    - port: 5432
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: auth-service
  namespace: default
spec:
  template:
    spec:
      containers:
        - name: auth
          image: auth:latest
          env:
            - name: DB_HOST
              value: "postgres.default.svc.cluster.local"
"#,
        )
        .unwrap();

        let discoverer = KubernetesDiscoverer::from_manifests_dir(dir.path().to_str().unwrap());
        let graph = discoverer.discover().unwrap();

        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);
        let edge = graph.edges().next().unwrap();
        assert_eq!(edge.edge_type, EdgeType::Queries);
    }

    #[test]
    fn test_kubernetes_discovery_skips_malformed_documents_instead_of_aborting() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("mixed.yaml"),
            r#"
this: is not a k8s manifest at all
---
apiVersion: v1
kind: Service
metadata:
  name: real-service
  namespace: default
spec: {}
"#,
        )
        .unwrap();

        let discoverer = KubernetesDiscoverer::from_manifests_dir(dir.path().to_str().unwrap());
        let graph = discoverer.discover().unwrap();

        assert_eq!(graph.node_count(), 1);
        assert_eq!(graph.nodes().next().unwrap().name, "real-service.default");
    }

    #[test]
    fn test_kubernetes_discovery_classifies_datastore_statefulsets() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("redis.yaml"),
            r#"
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: redis-cache
  namespace: default
spec:
  template:
    spec:
      containers:
        - name: redis
          image: redis:7
"#,
        )
        .unwrap();

        let discoverer = KubernetesDiscoverer::from_manifests_dir(dir.path().to_str().unwrap());
        let graph = discoverer.discover().unwrap();

        assert_eq!(graph.node_count(), 1);
        assert_eq!(graph.nodes().next().unwrap().node_type, NodeType::Cache);
    }
}
