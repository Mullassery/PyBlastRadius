use crate::error::Result;
use crate::graph::Graph;
use crate::graph::node::NodeId;
use serde::{Deserialize, Serialize};
use std::collections::{VecDeque, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FailureType {
    Outage,
    Degradation,
    HighLatency,
    HighErrorRate,
    DataQualityIssue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub source_node: NodeId,
    pub failure_type: String,
    pub cascade_path: Vec<NodeId>,
    pub estimated_impact_nodes: usize,
    pub estimated_recovery_time_minutes: u32,
    pub affected_services: Vec<String>,
    pub business_impact_score: f32, // 0-100
}

pub struct Simulator;

impl Simulator {
    pub fn simulate(
        graph: &Graph,
        source_node_id: NodeId,
        failure_type: FailureType,
    ) -> Result<SimulationResult> {
        let cascade = Self::simulate_cascade(graph, source_node_id)?;

        let source_node = graph
            .get_node(&source_node_id)
            .ok_or_else(|| crate::error::Error::not_found("Source node not found"))?;

        let estimated_recovery = Self::estimate_recovery_time(graph, &cascade)?;

        let affected_services: Vec<String> = cascade
            .iter()
            .filter_map(|&node_id| {
                graph.get_node(&node_id).map(|n| n.name.clone())
            })
            .collect();

        let business_impact_score =
            Self::calculate_business_impact(graph, &cascade)?;

        Ok(SimulationResult {
            source_node: source_node_id,
            failure_type: format!("{:?}", failure_type),
            cascade_path: cascade.clone(),
            estimated_impact_nodes: cascade.len(),
            estimated_recovery_time_minutes: estimated_recovery,
            affected_services,
            business_impact_score,
        })
    }

    fn simulate_cascade(graph: &Graph, source_node_id: NodeId) -> Result<Vec<NodeId>> {
        let mut cascade = vec![source_node_id];
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();

        queue.push_back(source_node_id);
        visited.insert(source_node_id);

        while let Some(node_id) = queue.pop_front() {
            let neighbors = graph.get_neighbors(&node_id)?;

            for neighbor in neighbors {
                if !visited.insert(neighbor) {
                    continue;
                }

                // Check if the edge to this neighbor is critical
                let edges = graph.get_edges_from(&node_id)?;
                let has_critical_edge = edges.iter().any(|e| {
                    e.target == neighbor && e.properties.is_critical
                });

                if has_critical_edge {
                    cascade.push(neighbor);
                    queue.push_back(neighbor);
                }
            }
        }

        Ok(cascade)
    }

    fn estimate_recovery_time(graph: &Graph, cascade: &[NodeId]) -> Result<u32> {
        let mut total_recovery = 0u32;

        for &node_id in cascade {
            if let Some(node) = graph.get_node(&node_id) {
                total_recovery = total_recovery.max(node.properties.recovery_time_minutes);
            }
        }

        Ok(total_recovery)
    }

    fn calculate_business_impact(graph: &Graph, cascade: &[NodeId]) -> Result<f32> {
        let mut total_exposure = 0.0;

        for &node_id in cascade {
            if let Some(node) = graph.get_node(&node_id) {
                total_exposure += node.properties.business_exposure;
            }
        }

        // Normalize to 0-100
        let impact = (total_exposure / (cascade.len() as f32)).min(1.0) * 100.0;

        Ok(impact)
    }

    pub fn simulate_with_time_limit(
        graph: &Graph,
        source_node_id: NodeId,
        failure_type: FailureType,
        max_cascade_depth: usize,
    ) -> Result<SimulationResult> {
        let cascade = Self::simulate_cascade_with_limit(graph, source_node_id, max_cascade_depth)?;

        let source_node = graph
            .get_node(&source_node_id)
            .ok_or_else(|| crate::error::Error::not_found("Source node not found"))?;

        let estimated_recovery = Self::estimate_recovery_time(graph, &cascade)?;

        let affected_services: Vec<String> = cascade
            .iter()
            .filter_map(|&node_id| {
                graph.get_node(&node_id).map(|n| n.name.clone())
            })
            .collect();

        let business_impact_score = Self::calculate_business_impact(graph, &cascade)?;

        Ok(SimulationResult {
            source_node: source_node_id,
            failure_type: format!("{:?}", failure_type),
            cascade_path: cascade,
            estimated_impact_nodes: affected_services.len(),
            estimated_recovery_time_minutes: estimated_recovery,
            affected_services,
            business_impact_score,
        })
    }

    fn simulate_cascade_with_limit(
        graph: &Graph,
        source_node_id: NodeId,
        max_depth: usize,
    ) -> Result<Vec<NodeId>> {
        let mut cascade = vec![source_node_id];
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut depth = 0;

        queue.push_back((source_node_id, 0));
        visited.insert(source_node_id);

        while let Some((node_id, current_depth)) = queue.pop_front() {
            if current_depth >= max_depth {
                continue;
            }

            let neighbors = graph.get_neighbors(&node_id)?;

            for neighbor in neighbors {
                if !visited.insert(neighbor) {
                    continue;
                }

                let edges = graph.get_edges_from(&node_id)?;
                let has_critical_edge = edges.iter().any(|e| {
                    e.target == neighbor && e.properties.is_critical
                });

                if has_critical_edge {
                    cascade.push(neighbor);
                    queue.push_back((neighbor, current_depth + 1));
                }
            }
        }

        Ok(cascade)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Graph, Node, Edge, EdgeType};

    #[test]
    fn test_cascade_simulation() {
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

        let result = Simulator::simulate(&graph, api, FailureType::Outage).unwrap();

        assert!(result.cascade_path.len() > 0);
        assert!(result.affected_services.len() > 0);
    }

    #[test]
    fn test_cascade_with_depth_limit() {
        let mut graph = Graph::new();

        let n1 = graph.add_node(Node::service("s1")).unwrap();
        let n2 = graph.add_node(Node::service("s2")).unwrap();
        let n3 = graph.add_node(Node::service("s3")).unwrap();

        graph.add_edge(Edge::new(EdgeType::Calls, n1, n2).with_criticality(true)).unwrap();
        graph.add_edge(Edge::new(EdgeType::Calls, n2, n3).with_criticality(true)).unwrap();

        let result =
            Simulator::simulate_with_time_limit(&graph, n1, FailureType::Outage, 1).unwrap();

        assert!(result.cascade_path.len() <= 2);
    }
}
