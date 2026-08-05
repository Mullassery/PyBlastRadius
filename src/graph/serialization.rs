use crate::error::{Error, Result};
use crate::graph::Graph;
use serde::{Deserialize, Serialize};

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
}
