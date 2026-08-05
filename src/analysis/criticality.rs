use crate::error::Result;
use crate::graph::Graph;
use crate::graph::node::NodeId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticalityScore {
    pub node_id: NodeId,
    pub node_name: String,
    pub criticality_score: f32, // 0-100
    pub downstream_count: usize,
    pub upstream_count: usize,
    pub is_single_point_of_failure: bool,
}

pub struct CriticalityScorer;

impl CriticalityScorer {
    pub fn score_all(graph: &Graph) -> Result<Vec<CriticalityScore>> {
        let mut scores = Vec::new();

        for node in graph.nodes() {
            let downstream = graph.get_neighbors(&node.id)?.len();
            let upstream = graph.get_predecessors(&node.id)?.len();

            let is_single_point_of_failure = Self::is_single_point_of_failure(graph, node.id)?;

            // Criticality formula:
            // (downstream_count × 0.4) + (node_criticality × 0.3) + (recovery_complexity × 0.2) + (business_exposure × 0.1)
            let downstream_score = (downstream as f32 / 10.0).min(1.0) * 0.4 * 100.0;
            let criticality_score = node.properties.criticality * 0.3 * 100.0;
            let recovery_score = (node.properties.recovery_time_minutes as f32 / 60.0).min(1.0) * 0.2 * 100.0;
            let business_score = node.properties.business_exposure * 0.1 * 100.0;

            let total = (downstream_score + criticality_score + recovery_score + business_score) / 100.0;

            scores.push(CriticalityScore {
                node_id: node.id,
                node_name: node.name.clone(),
                criticality_score: total,
                downstream_count: downstream,
                upstream_count: upstream,
                is_single_point_of_failure,
            });
        }

        // Sort by criticality descending
        scores.sort_by(|a, b| b.criticality_score.partial_cmp(&a.criticality_score).unwrap());

        Ok(scores)
    }

    pub fn score_node(graph: &Graph, node_id: NodeId) -> Result<CriticalityScore> {
        let node = graph
            .get_node(&node_id)
            .ok_or_else(|| crate::error::Error::not_found("Node not found"))?;

        let downstream = graph.get_neighbors(&node_id)?.len();
        let upstream = graph.get_predecessors(&node_id)?.len();
        let is_single_point_of_failure = Self::is_single_point_of_failure(graph, node_id)?;

        let downstream_score = (downstream as f32 / 10.0).min(1.0) * 0.4 * 100.0;
        let criticality_score = node.properties.criticality * 0.3 * 100.0;
        let recovery_score = (node.properties.recovery_time_minutes as f32 / 60.0).min(1.0) * 0.2 * 100.0;
        let business_score = node.properties.business_exposure * 0.1 * 100.0;

        let total = (downstream_score + criticality_score + recovery_score + business_score) / 100.0;

        Ok(CriticalityScore {
            node_id,
            node_name: node.name.clone(),
            criticality_score: total,
            downstream_count: downstream,
            upstream_count: upstream,
            is_single_point_of_failure,
        })
    }

    fn is_single_point_of_failure(graph: &Graph, node_id: NodeId) -> Result<bool> {
        let predecessors = graph.get_predecessors(&node_id)?;

        // If this node has predecessors and no backup paths, it's a single point of failure
        if predecessors.is_empty() {
            return Ok(false);
        }

        for predecessor in predecessors {
            let alternatives = graph.get_neighbors(&predecessor)?;
            if alternatives.len() <= 1 {
                // Predecessor only has this node as a neighbor -> single point of failure
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub fn get_critical_nodes(graph: &Graph, threshold: f32) -> Result<Vec<CriticalityScore>> {
        let scores = Self::score_all(graph)?;
        Ok(scores
            .into_iter()
            .filter(|s| s.criticality_score >= threshold * 100.0)
            .collect())
    }

    pub fn get_single_points_of_failure(graph: &Graph) -> Result<Vec<CriticalityScore>> {
        let scores = Self::score_all(graph)?;
        Ok(scores
            .into_iter()
            .filter(|s| s.is_single_point_of_failure)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Graph, Node, Edge, EdgeType};

    #[test]
    fn test_criticality_scoring() {
        let mut graph = Graph::new();

        let api = graph
            .add_node(Node::service("api").with_criticality(0.9))
            .unwrap();
        let db = graph
            .add_node(Node::database("db").with_criticality(0.95))
            .unwrap();

        graph.add_edge(Edge::new(EdgeType::Queries, api, db)).unwrap();

        let scores = CriticalityScorer::score_all(&graph).unwrap();
        assert_eq!(scores.len(), 2);

        // DB should have higher criticality
        assert!(scores[0].criticality_score > 0.0);
    }

    #[test]
    fn test_single_point_of_failure() {
        let mut graph = Graph::new();

        let api = graph.add_node(Node::service("api")).unwrap();
        let db = graph.add_node(Node::database("db")).unwrap();
        let cache = graph.add_node(Node::service("cache")).unwrap();

        graph.add_edge(Edge::new(EdgeType::Queries, api, db)).unwrap();
        graph.add_edge(Edge::new(EdgeType::Calls, api, cache)).unwrap();

        let db_score = CriticalityScorer::score_node(&graph, db).unwrap();
        assert!(db_score.is_single_point_of_failure);
    }
}
