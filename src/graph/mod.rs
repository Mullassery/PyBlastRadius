pub mod node;
pub mod edge;
pub mod graph;
pub mod traversal;
pub mod serialization;

pub use node::{Node, NodeType, NodeProperties};
pub use edge::{Edge, EdgeType, EdgeProperties};
pub use graph::{Graph, GraphMetrics};
pub use traversal::{BfsTraversal, DfsTraversal, TopoSort, TraversalResult};
pub use serialization::{GraphExporter, GraphImporter, ExportFormat};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_empty_graph() {
        let graph = Graph::new();
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_add_node() {
        let mut graph = Graph::new();
        let node = Node::service("api-server");
        let node_id = graph.add_node(node).unwrap();
        assert_eq!(graph.node_count(), 1);
        assert!(graph.get_node(&node_id).is_some());
    }

    #[test]
    fn test_add_edge() {
        let mut graph = Graph::new();
        let node1 = Node::service("api-server");
        let node2 = Node::service("database");

        let n1_id = graph.add_node(node1).unwrap();
        let n2_id = graph.add_node(node2).unwrap();

        let edge = Edge::new(EdgeType::Calls, n1_id, n2_id);
        graph.add_edge(edge).unwrap();

        assert_eq!(graph.edge_count(), 1);
    }
}
