use super::edge::{Edge, EdgeId, EdgeType};
use super::node::{Node, NodeId, NodeProperties};
use crate::error::{Error, Result};
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphMetrics {
    pub node_count: usize,
    pub edge_count: usize,
    pub avg_degree: f32,
    pub density: f32,
    pub connected_components: usize,
}

pub struct Graph {
    // petgraph for efficient traversal
    petgraph: DiGraph<NodeId, EdgeType>,
    // Maps for quick lookup
    nodes: HashMap<NodeId, Node>,
    edges: HashMap<EdgeId, Edge>,
    node_indices: HashMap<NodeId, NodeIndex>,
}

impl Graph {
    pub fn new() -> Self {
        Graph {
            petgraph: DiGraph::new(),
            nodes: HashMap::new(),
            edges: HashMap::new(),
            node_indices: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node: Node) -> Result<NodeId> {
        let node_id = node.id;
        let idx = self.petgraph.add_node(node_id);
        self.node_indices.insert(node_id, idx);
        self.nodes.insert(node_id, node);
        Ok(node_id)
    }

    pub fn add_edge(&mut self, edge: Edge) -> Result<EdgeId> {
        let source_idx = self
            .node_indices
            .get(&edge.source)
            .ok_or_else(|| Error::graph_error("Source node not found"))?;
        let target_idx = self
            .node_indices
            .get(&edge.target)
            .ok_or_else(|| Error::graph_error("Target node not found"))?;

        self.petgraph
            .add_edge(*source_idx, *target_idx, edge.edge_type);
        let edge_id = edge.id;
        self.edges.insert(edge_id, edge);

        Ok(edge_id)
    }

    pub fn get_node(&self, node_id: &NodeId) -> Option<&Node> {
        self.nodes.get(node_id)
    }

    pub fn get_node_mut(&mut self, node_id: &NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(node_id)
    }

    pub fn get_edge(&self, edge_id: &EdgeId) -> Option<&Edge> {
        self.edges.get(edge_id)
    }

    pub fn get_edge_mut(&mut self, edge_id: &EdgeId) -> Option<&mut Edge> {
        self.edges.get_mut(edge_id)
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        self.nodes.values()
    }

    pub fn edges(&self) -> impl Iterator<Item = &Edge> {
        self.edges.values()
    }

    pub fn get_neighbors(&self, node_id: &NodeId) -> Result<Vec<NodeId>> {
        let idx = self
            .node_indices
            .get(node_id)
            .ok_or_else(|| Error::graph_error("Node not found"))?;

        let neighbors: Vec<NodeId> = self
            .petgraph
            .neighbors(*idx)
            .filter_map(|neighbor_idx| self.petgraph.node_weight(neighbor_idx).copied())
            .collect();

        Ok(neighbors)
    }

    pub fn get_predecessors(&self, node_id: &NodeId) -> Result<Vec<NodeId>> {
        let idx = self
            .node_indices
            .get(node_id)
            .ok_or_else(|| Error::graph_error("Node not found"))?;

        let predecessors: Vec<NodeId> = self
            .petgraph
            .neighbors_directed(*idx, petgraph::Direction::Incoming)
            .filter_map(|pred_idx| self.petgraph.node_weight(pred_idx).copied())
            .collect();

        Ok(predecessors)
    }

    pub fn get_edges_from(&self, source_id: &NodeId) -> Result<Vec<&Edge>> {
        Ok(self
            .edges
            .values()
            .filter(|e| e.source == *source_id)
            .collect())
    }

    pub fn get_edges_to(&self, target_id: &NodeId) -> Result<Vec<&Edge>> {
        Ok(self
            .edges
            .values()
            .filter(|e| e.target == *target_id)
            .collect())
    }

    pub fn has_path(&self, from: &NodeId, to: &NodeId) -> Result<bool> {
        let from_idx = self
            .node_indices
            .get(from)
            .ok_or_else(|| Error::graph_error("Source node not found"))?;
        let to_idx = self
            .node_indices
            .get(to)
            .ok_or_else(|| Error::graph_error("Target node not found"))?;

        Ok(petgraph::algo::has_path_connecting(
            &self.petgraph,
            *from_idx,
            *to_idx,
            None,
        ))
    }

    pub fn metrics(&self) -> GraphMetrics {
        let node_count = self.node_count();
        let edge_count = self.edge_count();

        let avg_degree = if node_count > 0 {
            (2.0 * edge_count as f32) / node_count as f32
        } else {
            0.0
        };

        let density = if node_count > 1 {
            let max_edges = (node_count * (node_count - 1)) as f32;
            edge_count as f32 / max_edges
        } else {
            0.0
        };

        let connected_components = petgraph::algo::kosaraju_scc(&self.petgraph).len();

        GraphMetrics {
            node_count,
            edge_count,
            avg_degree,
            density,
            connected_components,
        }
    }

    pub fn remove_node(&mut self, node_id: &NodeId) -> Result<Node> {
        let node = self
            .nodes
            .remove(node_id)
            .ok_or_else(|| Error::not_found("Node not found"))?;

        if let Some(idx) = self.node_indices.remove(node_id) {
            self.petgraph.remove_node(idx);
        }

        // Remove associated edges
        self.edges
            .retain(|_, e| e.source != *node_id && e.target != *node_id);

        Ok(node)
    }

    pub fn remove_edge(&mut self, edge_id: &EdgeId) -> Result<Edge> {
        self.edges
            .remove(edge_id)
            .ok_or_else(|| Error::not_found("Edge not found"))
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
        self.edges.clear();
        self.node_indices.clear();
        self.petgraph.clear();
    }

    /// Merges another graph's nodes and edges into this one, preserving
    /// their original IDs. Used to combine results from multiple discovery
    /// sources (Kubernetes, Terraform, Airflow, OTLP) into one graph; nodes
    /// discovered independently by different sources are not deduplicated
    /// (cross-source entity resolution isn't implemented — each source's
    /// nodes stay distinct).
    pub fn merge(&mut self, other: &Graph) -> Result<()> {
        for node in other.nodes() {
            self.add_node(node.clone())?;
        }
        for edge in other.edges() {
            self.add_edge(edge.clone())?;
        }
        Ok(())
    }

    pub fn update_node_properties<F>(&mut self, node_id: &NodeId, f: F) -> Result<()>
    where
        F: FnOnce(&mut NodeProperties),
    {
        let node = self
            .nodes
            .get_mut(node_id)
            .ok_or_else(|| Error::not_found("Node not found"))?;
        f(&mut node.properties);
        Ok(())
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::node::Node;

    #[test]
    fn test_graph_operations() {
        let mut graph = Graph::new();

        let node1 = Node::service("api-server");
        let node2 = Node::database("postgresql");

        let n1_id = graph.add_node(node1).unwrap();
        let n2_id = graph.add_node(node2).unwrap();

        assert_eq!(graph.node_count(), 2);

        let edge = Edge::new(EdgeType::Queries, n1_id, n2_id);
        graph.add_edge(edge).unwrap();

        assert_eq!(graph.edge_count(), 1);
        assert!(graph.has_path(&n1_id, &n2_id).unwrap());
    }

    #[test]
    fn test_neighbors() {
        use crate::graph::edge::Edge;

        let mut graph = Graph::new();
        let n1 = graph.add_node(Node::service("api")).unwrap();
        let n2 = graph.add_node(Node::service("auth")).unwrap();
        let n3 = graph.add_node(Node::service("db")).unwrap();

        graph.add_edge(Edge::new(EdgeType::Calls, n1, n2)).unwrap();
        graph.add_edge(Edge::new(EdgeType::Calls, n1, n3)).unwrap();

        let neighbors = graph.get_neighbors(&n1).unwrap();
        assert_eq!(neighbors.len(), 2);
    }

    #[test]
    fn test_merge_combines_nodes_and_edges_preserving_ids() {
        use crate::graph::edge::Edge;

        let mut source_graph = Graph::new();
        let n1 = source_graph
            .add_node(Node::service("kube-service"))
            .unwrap();
        let n2 = source_graph.add_node(Node::database("kube-db")).unwrap();
        source_graph
            .add_edge(Edge::new(EdgeType::Queries, n1, n2))
            .unwrap();

        let mut combined = Graph::new();
        let n3 = combined.add_node(Node::service("tf-service")).unwrap();
        combined.merge(&source_graph).unwrap();

        assert_eq!(combined.node_count(), 3);
        assert_eq!(combined.edge_count(), 1);
        assert!(combined.get_node(&n1).is_some());
        assert!(combined.get_node(&n3).is_some());
    }
}
