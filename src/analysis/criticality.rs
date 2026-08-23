use crate::error::Result;
use crate::graph::node::NodeId;
use crate::graph::Graph;
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
            let recovery_score =
                (node.properties.recovery_time_minutes as f32 / 60.0).min(1.0) * 0.2 * 100.0;
            let business_score = node.properties.business_exposure * 0.1 * 100.0;

            // Each weighted component above is already scaled onto the 0-100
            // axis (e.g. criticality_score maxes at 0.3 * 100 = 30), so their
            // sum already lands in 0-100. Dividing by 100 again was
            // collapsing the documented 0-100 scale down to ~0-1.
            let total = downstream_score + criticality_score + recovery_score + business_score;

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
        scores.sort_by(|a, b| {
            b.criticality_score
                .partial_cmp(&a.criticality_score)
                .unwrap()
        });

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
        let recovery_score =
            (node.properties.recovery_time_minutes as f32 / 60.0).min(1.0) * 0.2 * 100.0;
        let business_score = node.properties.business_exposure * 0.1 * 100.0;

        // See score_all(): components are already on the 0-100 scale, so no
        // further division is needed here.
        let total = downstream_score + criticality_score + recovery_score + business_score;

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

        // A node is a single point of failure if there is exactly one edge
        // reaching it: no alternate/redundant path exists into this node, so
        // losing that one predecessor makes this node (and everything
        // downstream of it) unreachable.
        //
        // Note this must look at *this node's* incoming edge count, not at
        // whether its predecessor happens to fan out to other nodes too. A
        // predecessor with multiple outgoing edges (a branching topology)
        // does not create a backup path *into this node* - it just means the
        // predecessor also reaches other, unrelated nodes. The previous
        // implementation checked the predecessor's out-degree instead of
        // this node's in-degree, which produced false negatives for exactly
        // that branching case (e.g. root -> A, root -> B, A -> X: X has a
        // single predecessor A, so X is a SPOF, even though A itself also
        // points at B).
        Ok(predecessors.len() == 1)
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
    use crate::graph::{Edge, EdgeType, Graph, Node};

    #[test]
    fn test_criticality_scoring() {
        let mut graph = Graph::new();

        let api = graph
            .add_node(Node::service("api").with_criticality(0.9))
            .unwrap();
        let db = graph
            .add_node(Node::database("db").with_criticality(0.95))
            .unwrap();

        graph
            .add_edge(Edge::new(EdgeType::Queries, api, db))
            .unwrap();

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

        graph
            .add_edge(Edge::new(EdgeType::Queries, api, db))
            .unwrap();
        graph
            .add_edge(Edge::new(EdgeType::Calls, api, cache))
            .unwrap();

        let db_score = CriticalityScorer::score_node(&graph, db).unwrap();
        assert!(db_score.is_single_point_of_failure);
    }

    /// Regression test for the false-negative SPOF bug on branching
    /// topologies: a node fed by a single predecessor must be flagged as a
    /// SPOF even when that predecessor also branches off to other,
    /// unrelated nodes.
    ///
    ///   root -> A -> leaf_via_a   (leaf_via_a's only path in is through A)
    ///   root -> B
    ///   A    -> B                (B is reachable both via root and via A)
    #[test]
    fn test_single_point_of_failure_branching_topology() {
        let mut graph = Graph::new();

        let root = graph.add_node(Node::service("root")).unwrap();
        let a = graph.add_node(Node::service("a")).unwrap();
        let b = graph.add_node(Node::service("b")).unwrap();
        let leaf_via_a = graph.add_node(Node::service("leaf-via-a")).unwrap();

        graph.add_edge(Edge::new(EdgeType::Calls, root, a)).unwrap();
        graph.add_edge(Edge::new(EdgeType::Calls, root, b)).unwrap();
        graph.add_edge(Edge::new(EdgeType::Calls, a, b)).unwrap();
        graph
            .add_edge(Edge::new(EdgeType::Calls, a, leaf_via_a))
            .unwrap();

        // `a` branches to two nodes (b and leaf_via_a), but leaf_via_a still
        // has exactly one way in (through a) -> it is a real SPOF. The old
        // implementation only flagged a node when its predecessor had <= 1
        // outgoing edge, so it missed this because `a` has 2 outgoing edges.
        let leaf_score = CriticalityScorer::score_node(&graph, leaf_via_a).unwrap();
        assert!(
            leaf_score.is_single_point_of_failure,
            "node reachable via exactly one predecessor should be a SPOF, \
             even though that predecessor also branches elsewhere"
        );

        // `b` has two independent predecessors (root and a) -> there's a
        // redundant path in, so it is NOT a single point of failure.
        let b_score = CriticalityScorer::score_node(&graph, b).unwrap();
        assert!(
            !b_score.is_single_point_of_failure,
            "node reachable via multiple independent predecessors should not be a SPOF"
        );
    }
}
