use crate::error::{Error, Result};
use crate::graph::{Edge, Graph, Node};
use serde::{Deserialize, Serialize};

/// Lossless graph snapshot for save/load round-tripping (unlike
/// `GraphExport`, which is a simplified, string-typed view meant for human
/// inspection / interop with other tools and can't be loaded back).
#[derive(Debug, Serialize, Deserialize)]
struct GraphSnapshot {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Json,
    Yaml,
    GraphML,
    Dot,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphExport {
    pub nodes: Vec<NodeExport>,
    pub edges: Vec<EdgeExport>,
    pub metadata: GraphMetadataExport,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeExport {
    pub id: String,
    pub name: String,
    pub node_type: String,
    pub criticality: f32,
    pub health_status: String,
    pub recovery_time_minutes: u32,
    pub tags: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EdgeExport {
    pub id: String,
    pub source: String,
    pub target: String,
    pub edge_type: String,
    pub is_critical: bool,
    pub latency_ms: Option<f32>,
    pub error_rate: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphMetadataExport {
    pub node_count: usize,
    pub edge_count: usize,
    pub exported_at: String,
}

pub struct GraphExporter;

impl GraphExporter {
    pub fn export(graph: &Graph, format: ExportFormat) -> Result<String> {
        match format {
            ExportFormat::Json => Self::export_json(graph),
            ExportFormat::Yaml => Self::export_yaml(graph),
            ExportFormat::GraphML => Self::export_graphml(graph),
            ExportFormat::Dot => Self::export_dot(graph),
        }
    }

    pub fn export_json(graph: &Graph) -> Result<String> {
        let export = Self::build_export(graph);
        serde_json::to_string_pretty(&export)
            .map_err(|e| Error::parse_error(format!("JSON export failed: {}", e)))
    }

    pub fn export_yaml(graph: &Graph) -> Result<String> {
        let export = Self::build_export(graph);
        serde_yaml::to_string(&export)
            .map_err(|e| Error::parse_error(format!("YAML export failed: {}", e)))
    }

    pub fn export_graphml(graph: &Graph) -> Result<String> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<graphml xmlns=\"http://graphml.graphdrawing.org/xmlns\">\n");
        xml.push_str("  <graph mode=\"static\" edgedefault=\"directed\">\n");

        // Add nodes
        for node in graph.nodes() {
            xml.push_str(&format!(
                "    <node id=\"{}\"><data key=\"label\">{}</data></node>\n",
                node.id.0, node.name
            ));
        }

        // Add edges
        for edge in graph.edges() {
            xml.push_str(&format!(
                "    <edge source=\"{}\" target=\"{}\"></edge>\n",
                edge.source.0, edge.target.0
            ));
        }

        xml.push_str("  </graph>\n");
        xml.push_str("</graphml>");

        Ok(xml)
    }

    pub fn export_dot(graph: &Graph) -> Result<String> {
        let mut dot = String::from("digraph {\n");
        dot.push_str("  rankdir=LR;\n");
        dot.push_str("  node [shape=box];\n\n");

        // Add nodes
        for node in graph.nodes() {
            let color = match node.node_type {
                crate::graph::NodeType::Service => "lightblue",
                crate::graph::NodeType::Database => "lightpink",
                crate::graph::NodeType::Pipeline => "lightyellow",
                _ => "lightgray",
            };
            dot.push_str(&format!(
                "  \"{}\" [label=\"{}\", style=filled, fillcolor=\"{}\"];\n",
                node.id.0, node.name, color
            ));
        }

        dot.push_str("\n");

        // Add edges
        for edge in graph.edges() {
            let style = if edge.properties.is_critical {
                "solid"
            } else {
                "dashed"
            };
            dot.push_str(&format!(
                "  \"{}\" -> \"{}\" [style=\"{}\"];\n",
                edge.source.0, edge.target.0, style
            ));
        }

        dot.push_str("}\n");
        Ok(dot)
    }

    /// Losslessly serializes the graph (all node/edge fields, original IDs)
    /// so it can be reloaded exactly via [`GraphImporter::load_json`].
    pub fn save_json(graph: &Graph) -> Result<String> {
        let snapshot = GraphSnapshot {
            nodes: graph.nodes().cloned().collect(),
            edges: graph.edges().cloned().collect(),
        };
        serde_json::to_string_pretty(&snapshot)
            .map_err(|e| Error::parse_error(format!("graph snapshot serialization failed: {}", e)))
    }

    /// Saves a graph to a JSON or YAML file, inferring format from the file
    /// extension (`.yaml`/`.yml` vs everything else treated as JSON).
    pub fn save_file(graph: &Graph, path: &std::path::Path) -> Result<()> {
        let contents = match path.extension().and_then(|e| e.to_str()) {
            Some("yaml") | Some("yml") => {
                let snapshot = GraphSnapshot {
                    nodes: graph.nodes().cloned().collect(),
                    edges: graph.edges().cloned().collect(),
                };
                serde_yaml::to_string(&snapshot)
                    .map_err(|e| Error::parse_error(format!("graph snapshot YAML serialization failed: {}", e)))?
            }
            _ => Self::save_json(graph)?,
        };
        std::fs::write(path, contents)?;
        Ok(())
    }

    fn build_export(graph: &Graph) -> GraphExport {
        let nodes = graph
            .nodes()
            .map(|n| NodeExport {
                id: n.id.0.to_string(),
                name: n.name.clone(),
                node_type: format!("{:?}", n.node_type),
                criticality: n.properties.criticality,
                health_status: format!("{:?}", n.properties.health_status),
                recovery_time_minutes: n.properties.recovery_time_minutes,
                tags: serde_json::to_value(&n.properties.tags).unwrap_or(serde_json::Value::Object(Default::default())),
            })
            .collect();

        let edges = graph
            .edges()
            .map(|e| EdgeExport {
                id: e.id.0.to_string(),
                source: e.source.0.to_string(),
                target: e.target.0.to_string(),
                edge_type: format!("{:?}", e.edge_type),
                is_critical: e.properties.is_critical,
                latency_ms: e.properties.latency_ms,
                error_rate: e.properties.error_rate,
            })
            .collect();

        GraphExport {
            nodes,
            edges,
            metadata: GraphMetadataExport {
                node_count: graph.node_count(),
                edge_count: graph.edge_count(),
                exported_at: chrono::Utc::now().to_rfc3339(),
            },
        }
    }
}

pub struct GraphImporter;

impl GraphImporter {
    /// Loads a graph previously saved with [`GraphExporter::save_json`].
    /// Node/edge IDs are preserved exactly (nodes are added first so edges
    /// can resolve their source/target against them).
    pub fn load_json(json: &str) -> Result<Graph> {
        let snapshot: GraphSnapshot = serde_json::from_str(json)
            .map_err(|e| Error::parse_error(format!("graph snapshot parse failed: {}", e)))?;

        let mut graph = Graph::new();
        for node in snapshot.nodes {
            graph.add_node(node)?;
        }
        for edge in snapshot.edges {
            graph.add_edge(edge)?;
        }
        Ok(graph)
    }

    /// Loads a graph from a JSON or YAML file, inferring format from the
    /// file extension (`.yaml`/`.yml` vs everything else treated as JSON).
    pub fn load_file(path: &std::path::Path) -> Result<Graph> {
        let contents = std::fs::read_to_string(path)?;
        match path.extension().and_then(|e| e.to_str()) {
            Some("yaml") | Some("yml") => {
                let snapshot: GraphSnapshot = serde_yaml::from_str(&contents)
                    .map_err(|e| Error::parse_error(format!("graph snapshot YAML parse failed: {}", e)))?;
                let mut graph = Graph::new();
                for node in snapshot.nodes {
                    graph.add_node(node)?;
                }
                for edge in snapshot.edges {
                    graph.add_edge(edge)?;
                }
                Ok(graph)
            }
            _ => Self::load_json(&contents),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Graph, Node, Edge, EdgeType};

    #[test]
    fn test_json_export() {
        let mut graph = Graph::new();
        let n1 = graph.add_node(Node::service("api")).unwrap();
        let n2 = graph.add_node(Node::service("db")).unwrap();
        graph.add_edge(Edge::new(EdgeType::Calls, n1, n2)).unwrap();

        let json = GraphExporter::export_json(&graph).unwrap();
        assert!(json.contains("api"));
        assert!(json.contains("db"));
    }

    #[test]
    fn test_dot_export() {
        let mut graph = Graph::new();
        let n1 = graph.add_node(Node::service("api")).unwrap();
        let n2 = graph.add_node(Node::service("db")).unwrap();
        graph.add_edge(Edge::new(EdgeType::Calls, n1, n2)).unwrap();

        let dot = GraphExporter::export_dot(&graph).unwrap();
        assert!(dot.contains("digraph"));
        assert!(dot.contains("api"));
    }

    #[test]
    fn test_save_load_json_round_trip_preserves_ids_and_properties() {
        let mut graph = Graph::new();
        let n1 = graph
            .add_node(Node::service("api").with_criticality(0.9).with_owner("sre-team"))
            .unwrap();
        let n2 = graph.add_node(Node::database("postgres").with_criticality(0.98)).unwrap();
        graph
            .add_edge(
                Edge::new(EdgeType::Queries, n1, n2)
                    .with_latency(12.5)
                    .with_error_rate(0.01)
                    .with_confidence(0.8),
            )
            .unwrap();

        let json = GraphExporter::save_json(&graph).unwrap();
        let loaded = GraphImporter::load_json(&json).unwrap();

        assert_eq!(loaded.node_count(), 2);
        assert_eq!(loaded.edge_count(), 1);

        let loaded_api = loaded.get_node(&n1).unwrap();
        assert_eq!(loaded_api.name, "api");
        assert_eq!(loaded_api.properties.criticality, 0.9);
        assert_eq!(loaded_api.properties.owner, Some("sre-team".to_string()));

        let loaded_edge = loaded.edges().next().unwrap();
        assert_eq!(loaded_edge.source, n1);
        assert_eq!(loaded_edge.target, n2);
        assert_eq!(loaded_edge.properties.latency_ms, Some(12.5));
        assert_eq!(loaded_edge.properties.error_rate, Some(0.01));
    }

    #[test]
    fn test_save_load_file_round_trip_json() {
        let mut graph = Graph::new();
        let n1 = graph.add_node(Node::service("checkout")).unwrap();
        let n2 = graph.add_node(Node::service("payment")).unwrap();
        graph.add_edge(Edge::new(EdgeType::Calls, n1, n2)).unwrap();

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("graph.json");

        GraphExporter::save_file(&graph, &path).unwrap();
        let loaded = GraphImporter::load_file(&path).unwrap();

        assert_eq!(loaded.node_count(), 2);
        assert_eq!(loaded.edge_count(), 1);
    }

    #[test]
    fn test_save_load_file_round_trip_yaml() {
        let mut graph = Graph::new();
        let n1 = graph.add_node(Node::service("checkout")).unwrap();
        let n2 = graph.add_node(Node::service("payment")).unwrap();
        graph.add_edge(Edge::new(EdgeType::Calls, n1, n2)).unwrap();

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("graph.yaml");

        GraphExporter::save_file(&graph, &path).unwrap();
        let loaded = GraphImporter::load_file(&path).unwrap();

        assert_eq!(loaded.node_count(), 2);
        assert_eq!(loaded.edge_count(), 1);
    }

    #[test]
    fn test_load_json_rejects_malformed_input() {
        let result = GraphImporter::load_json("not valid json");
        assert!(result.is_err());
    }
}
