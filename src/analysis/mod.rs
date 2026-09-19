pub mod anomaly_detection;
pub mod blast_radius;
pub mod criticality;
pub mod hidden_dependencies;
pub mod simulator;

pub use anomaly_detection::{AnomalyDetector, AnomalyType, DetectedAnomaly};
pub use blast_radius::{BlastRadius, BlastRadiusAnalyzer, BlastRadiusResult};
pub use criticality::{CriticalityScore, CriticalityScorer};
pub use hidden_dependencies::{
    DependencyConfidenceScorer, HiddenDependency, HiddenDependencyDetector,
};
pub use simulator::{FailureType, SimulationResult, Simulator};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Edge, EdgeType, Graph, Node};

    fn create_sample_graph() -> Graph {
        let mut graph = Graph::new();

        let api = graph
            .add_node(Node::service("api-server").with_criticality(0.9))
            .unwrap();

        let auth = graph
            .add_node(Node::service("auth-service").with_criticality(0.95))
            .unwrap();

        let db = graph
            .add_node(Node::database("postgresql").with_criticality(0.98))
            .unwrap();

        let cache = graph
            .add_node(Node::service("redis").with_criticality(0.7))
            .unwrap();

        graph
            .add_edge(Edge::new(EdgeType::Calls, api, auth).with_criticality(true))
            .unwrap();
        graph
            .add_edge(Edge::new(EdgeType::Calls, auth, db).with_criticality(true))
            .unwrap();
        graph
            .add_edge(Edge::new(EdgeType::Calls, api, cache).with_criticality(false))
            .unwrap();

        graph
    }

    #[test]
    fn test_blast_radius_analysis() {
        let graph = create_sample_graph();
        // graph.nodes() iterates a HashMap, so its order isn't stable
        // across runs -- picking nodes[0] blindly meant this test only
        // passed when iteration order happened to land on "api-server"
        // (which has outgoing edges) rather than "postgresql" or "redis"
        // (both leaves with none), making it flaky by construction. Look
        // up the specific node this test actually means to start from.
        let start = graph
            .nodes()
            .find(|n| n.name == "api-server")
            .map(|n| n.id)
            .expect("api-server node must exist in the sample graph");

        let result = BlastRadiusAnalyzer::analyze(&graph, start).unwrap();
        assert!(!result.directly_impacted.is_empty());
    }

    #[test]
    fn test_criticality_scoring() {
        let graph = create_sample_graph();
        let scores = CriticalityScorer::score_all(&graph).unwrap();
        assert!(!scores.is_empty());

        // DB should have highest criticality
        let db_node = graph.nodes().find(|n| n.name == "postgresql").unwrap();
        let db_score = scores.iter().find(|s| s.node_id == db_node.id).unwrap();

        assert!(db_score.criticality_score > 0.8);
    }
}
