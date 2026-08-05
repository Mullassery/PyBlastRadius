pub mod blast_radius;
pub mod criticality;
pub mod simulator;

pub use blast_radius::{BlastRadius, BlastRadiusAnalyzer, BlastRadiusResult};
pub use criticality::{CriticalityScorer, CriticalityScore};
pub use simulator::{Simulator, SimulationResult, FailureType};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Graph, Node, Edge, EdgeType};

    fn create_sample_graph() -> Graph {
        let mut graph = Graph::new();

        let api = graph.add_node(Node::service("api-server")
            .with_criticality(0.9)
        ).unwrap();

        let auth = graph.add_node(Node::service("auth-service")
            .with_criticality(0.95)
        ).unwrap();

        let db = graph.add_node(Node::database("postgresql")
            .with_criticality(0.98)
        ).unwrap();

        let cache = graph.add_node(Node::service("redis")
            .with_criticality(0.7)
        ).unwrap();

        graph.add_edge(Edge::new(EdgeType::Calls, api, auth).with_criticality(true)).unwrap();
        graph.add_edge(Edge::new(EdgeType::Calls, auth, db).with_criticality(true)).unwrap();
        graph.add_edge(Edge::new(EdgeType::Calls, api, cache).with_criticality(false)).unwrap();

        graph
    }

    #[test]
    fn test_blast_radius_analysis() {
        let graph = create_sample_graph();
        let nodes: Vec<_> = graph.nodes().map(|n| n.id).collect();
        let start = nodes[0];

        let result = BlastRadiusAnalyzer::analyze(&graph, start).unwrap();
        assert!(result.directly_impacted.len() > 0);
    }

    #[test]
    fn test_criticality_scoring() {
        let graph = create_sample_graph();
        let scores = CriticalityScorer::score_all(&graph).unwrap();
        assert!(scores.len() > 0);

        // DB should have highest criticality
        let db_node = graph.nodes()
            .find(|n| n.name == "postgresql")
            .unwrap();
        let db_score = scores.iter()
            .find(|s| s.node_id == db_node.id)
            .unwrap();

        assert!(db_score.criticality_score > 0.8);
    }
}
