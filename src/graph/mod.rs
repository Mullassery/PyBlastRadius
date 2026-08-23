pub mod edge;
// graph::graph::Graph reads oddly in isolation, but graph::Graph (the
// re-export below) is what every caller actually uses -- renaming the file
// would only rename the lint away, not change any real API.
#[allow(clippy::module_inception)]
pub mod graph;
pub mod node;
pub mod serialization;
pub mod traversal;

pub use edge::{Edge, EdgeProperties, EdgeType};
pub use graph::{Graph, GraphMetrics};
pub use node::{Node, NodeProperties, NodeType};
pub use serialization::{ExportFormat, GraphExporter, GraphImporter};
pub use traversal::{BfsTraversal, DfsTraversal, TopoSort, TraversalResult};

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
