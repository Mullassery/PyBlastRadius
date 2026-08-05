use crate::error::Result;
use crate::graph::{Graph, BfsTraversal};
use crate::graph::node::NodeId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlastRadius {
    pub directly_impacted: Vec<NodeId>,
    pub indirectly_impacted: Vec<NodeId>,
    pub tertiary_impacted: Vec<NodeId>,
    pub blast_radius_score: f32, // 0-100
    pub critical_paths: Vec<Vec<NodeId>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlastRadiusResult {
    pub source_node: NodeId,
    pub directly_impacted: Vec<NodeId>,
    pub indirectly_impacted: Vec<NodeId>,
    pub tertiary_impacted: Vec<NodeId>,
    pub blast_radius_score: f32,
    pub critical_path_count: usize,
    pub total_impacted: usize,
}

pub struct BlastRadiusAnalyzer;

impl BlastRadiusAnalyzer {
    pub fn analyze(graph: &Graph, source_node_id: NodeId) -> Result<BlastRadiusResult> {
        // Use BFS to get impact by depth
        let traversal = BfsTraversal::traverse(graph, source_node_id)?;

        // Categorize by depth
        let (directly, indirectly, tertiary) = Self::categorize_by_depth(&traversal.depth_levels);

        // Calculate blast radius score
        let blast_radius_score = Self::calculate_score(
            graph,
            source_node_id,
            &directly,
            &indirectly,
            &tertiary,
        )?;

        // Count critical paths
        let critical_path_count = Self::count_critical_paths(graph, source_node_id, &directly)?;

        let total_impacted = directly.len() + indirectly.len() + tertiary.len();

        Ok(BlastRadiusResult {
            source_node: source_node_id,
            directly_impacted: directly,
            indirectly_impacted: indirectly,
            tertiary_impacted: tertiary,
            blast_radius_score,
            critical_path_count,
            total_impacted,
        })
    }

    pub fn analyze_with_depth(
        graph: &Graph,
        source_node_id: NodeId,
        max_depth: usize,
    ) -> Result<BlastRadiusResult> {
        let traversal = crate::graph::BfsTraversal::traverse_with_limit(
            graph,
            source_node_id,
            max_depth,
        )?;

        let (directly, indirectly, tertiary) = Self::categorize_by_depth(&traversal.depth_levels);

        let blast_radius_score = Self::calculate_score(
            graph,
            source_node_id,
            &directly,
            &indirectly,
            &tertiary,
        )?;

        let critical_path_count = Self::count_critical_paths(graph, source_node_id, &directly)?;

        let total_impacted = directly.len() + indirectly.len() + tertiary.len();

        Ok(BlastRadiusResult {
            source_node: source_node_id,
            directly_impacted: directly,
            indirectly_impacted: indirectly,
            tertiary_impacted: tertiary,
            blast_radius_score,
            critical_path_count,
            total_impacted,
        })
    }

    fn categorize_by_depth(depth_levels: &[Vec<NodeId>]) -> (Vec<NodeId>, Vec<NodeId>, Vec<NodeId>) {
        let directly = if depth_levels.len() > 1 {
            depth_levels[1].clone()
        } else {
            vec![]
        };

        let indirectly = if depth_levels.len() > 2 {
            depth_levels[2].clone()
        } else {
            vec![]
        };

        let tertiary = if depth_levels.len() > 3 {
            depth_levels[3..].iter().flat_map(|v| v.clone()).collect()
        } else {
            vec![]
        };

        (directly, indirectly, tertiary)
    }

    fn calculate_score(
        graph: &Graph,
        source_node_id: NodeId,
        directly: &[NodeId],
        indirectly: &[NodeId],
        tertiary: &[NodeId],
    ) -> Result<f32> {
        // Blast radius score formula:
        // (direct_count × 0.4) + (indirect_count × 0.3) + (tertiary_count × 0.2) + (criticality × 0.1)

        let source_node = graph
            .get_node(&source_node_id)
            .expect("Source node should exist");

        let direct_score = (directly.len() as f32) * 0.4;
        let indirect_score = (indirectly.len() as f32) * 0.3;
        let tertiary_score = (tertiary.len() as f32) * 0.2;
        let criticality_score = source_node.properties.criticality * 0.1;

        // Normalize to 0-100
        let raw_score = direct_score + indirect_score + tertiary_score + criticality_score;
        let normalized = (raw_score / 10.0).min(100.0);

        Ok(normalized)
    }

    fn count_critical_paths(
        graph: &Graph,
        source_node_id: NodeId,
        directly_impacted: &[NodeId],
    ) -> Result<usize> {
        let mut count = 0;

        for &target in directly_impacted {
            // Check if there's a critical edge between source and target
            let edges = graph.get_edges_from(&source_node_id)?;
            for edge in edges {
                if edge.target == target && edge.properties.is_critical {
                    count += 1;
                }
            }
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Graph, Node, Edge, EdgeType};

    #[test]
    fn test_blast_radius_calculation() {
        let mut graph = Graph::new();

        let api = graph.add_node(Node::service("api")).unwrap();
        let auth = graph.add_node(Node::service("auth")).unwrap();
        let db = graph.add_node(Node::database("db")).unwrap();

        graph
            .add_edge(Edge::new(EdgeType::Calls, api, auth).with_criticality(true))
            .unwrap();
        graph
            .add_edge(Edge::new(EdgeType::Queries, auth, db).with_criticality(true))
            .unwrap();

        let result = BlastRadiusAnalyzer::analyze(&graph, api).unwrap();

        assert!(result.blast_radius_score > 0.0);
        assert!(result.directly_impacted.len() > 0);
    }

    #[test]
    fn test_depth_limit() {
        let mut graph = Graph::new();

        let n1 = graph.add_node(Node::service("s1")).unwrap();
        let n2 = graph.add_node(Node::service("s2")).unwrap();
        let n3 = graph.add_node(Node::service("s3")).unwrap();

        graph.add_edge(Edge::new(EdgeType::Calls, n1, n2)).unwrap();
        graph.add_edge(Edge::new(EdgeType::Calls, n2, n3)).unwrap();

        let result = BlastRadiusAnalyzer::analyze_with_depth(&graph, n1, 1).unwrap();

        assert_eq!(result.directly_impacted.len(), 1);
        assert_eq!(result.indirectly_impacted.len(), 0);
    }
}
