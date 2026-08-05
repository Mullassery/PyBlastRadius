use std::collections::{VecDeque, HashSet};
use serde::{Deserialize, Serialize};
use super::node::NodeId;
use crate::error::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalResult {
    pub visited: Vec<NodeId>,
    pub depth_levels: Vec<Vec<NodeId>>, // depth -> nodes at that depth
}

pub struct BfsTraversal;

impl BfsTraversal {
    pub fn traverse(
        graph: &crate::graph::Graph,
        start: NodeId,
    ) -> Result<TraversalResult> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut depth_levels = vec![vec![]];

        queue.push_back((start, 0));
        visited.insert(start);
        depth_levels[0].push(start);

        while let Some((node_id, depth)) = queue.pop_front() {
            let neighbors = graph.get_neighbors(&node_id)?;

            for neighbor in neighbors {
                if visited.insert(neighbor) {
                    let next_depth = depth + 1;

                    if next_depth >= depth_levels.len() {
                        depth_levels.push(vec![]);
                    }

                    depth_levels[next_depth].push(neighbor);
                    queue.push_back((neighbor, next_depth));
                }
            }
        }

        Ok(TraversalResult {
            visited: visited.into_iter().collect(),
            depth_levels,
        })
    }

    pub fn traverse_with_limit(
        graph: &crate::graph::Graph,
        start: NodeId,
        max_depth: usize,
    ) -> Result<TraversalResult> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut depth_levels = vec![vec![]];

        queue.push_back((start, 0));
        visited.insert(start);
        depth_levels[0].push(start);

        while let Some((node_id, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }

            let neighbors = graph.get_neighbors(&node_id)?;

            for neighbor in neighbors {
                if visited.insert(neighbor) {
                    let next_depth = depth + 1;

                    if next_depth >= depth_levels.len() {
                        depth_levels.push(vec![]);
                    }

                    depth_levels[next_depth].push(neighbor);
                    if next_depth < max_depth {
                        queue.push_back((neighbor, next_depth));
                    }
                }
            }
        }

        Ok(TraversalResult {
            visited: visited.into_iter().collect(),
            depth_levels,
        })
    }
}

pub struct DfsTraversal;

impl DfsTraversal {
    pub fn traverse(
        graph: &crate::graph::Graph,
        start: NodeId,
    ) -> Result<TraversalResult> {
        let mut visited = HashSet::new();
        let mut stack = vec![(start, 0)];
        let mut depth_levels = vec![vec![]];
        let mut result = vec![];

        visited.insert(start);
        depth_levels[0].push(start);

        while let Some((node_id, depth)) = stack.pop() {
            result.push(node_id);

            let neighbors = graph.get_neighbors(&node_id)?;

            for neighbor in neighbors {
                if visited.insert(neighbor) {
                    let next_depth = depth + 1;

                    if next_depth >= depth_levels.len() {
                        depth_levels.push(vec![]);
                    }

                    depth_levels[next_depth].push(neighbor);
                    stack.push((neighbor, next_depth));
                }
            }
        }

        Ok(TraversalResult {
            visited: result,
            depth_levels,
        })
    }
}

pub struct TopoSort;

impl TopoSort {
    pub fn sort(graph: &crate::graph::Graph) -> Result<Vec<NodeId>> {
        let mut visited = HashSet::new();
        let mut stack = Vec::new();

        for node in graph.nodes() {
            if !visited.contains(&node.id) {
                Self::visit(graph, node.id, &mut visited, &mut stack)?;
            }
        }

        stack.reverse();
        Ok(stack)
    }

    fn visit(
        graph: &crate::graph::Graph,
        node_id: NodeId,
        visited: &mut HashSet<NodeId>,
        stack: &mut Vec<NodeId>,
    ) -> Result<()> {
        visited.insert(node_id);

        let neighbors = graph.get_neighbors(&node_id)?;
        for neighbor in neighbors {
            if !visited.contains(&neighbor) {
                Self::visit(graph, neighbor, visited, stack)?;
            }
        }

        stack.push(node_id);
        Ok(())
    }

    pub fn has_cycle(graph: &crate::graph::Graph) -> Result<bool> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for node in graph.nodes() {
            if !visited.contains(&node.id) {
                if Self::has_cycle_util(graph, node.id, &mut visited, &mut rec_stack)? {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    fn has_cycle_util(
        graph: &crate::graph::Graph,
        node_id: NodeId,
        visited: &mut HashSet<NodeId>,
        rec_stack: &mut HashSet<NodeId>,
    ) -> Result<bool> {
        visited.insert(node_id);
        rec_stack.insert(node_id);

        let neighbors = graph.get_neighbors(&node_id)?;
        for neighbor in neighbors {
            if !visited.contains(&neighbor) {
                if Self::has_cycle_util(graph, neighbor, visited, rec_stack)? {
                    return Ok(true);
                }
            } else if rec_stack.contains(&neighbor) {
                return Ok(true);
            }
        }

        rec_stack.remove(&node_id);
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Graph, Node, Edge, NodeType, EdgeType};

    fn create_test_graph() -> Graph {
        let mut graph = Graph::new();

        let n1 = graph.add_node(Node::service("api")).unwrap();
        let n2 = graph.add_node(Node::service("auth")).unwrap();
        let n3 = graph.add_node(Node::service("db")).unwrap();
        let n4 = graph.add_node(Node::service("cache")).unwrap();

        graph.add_edge(Edge::new(EdgeType::Calls, n1, n2)).unwrap();
        graph.add_edge(Edge::new(EdgeType::Calls, n2, n3)).unwrap();
        graph.add_edge(Edge::new(EdgeType::Calls, n1, n4)).unwrap();

        graph
    }

    #[test]
    fn test_bfs_traversal() {
        let graph = create_test_graph();
        let nodes: Vec<_> = graph.nodes().map(|n| n.id).collect();
        let start = nodes[0];

        let result = BfsTraversal::traverse(&graph, start).unwrap();
        assert!(result.visited.len() >= 1);
    }

    #[test]
    fn test_bfs_depth_limit() {
        let graph = create_test_graph();
        let nodes: Vec<_> = graph.nodes().map(|n| n.id).collect();
        let start = nodes[0];

        let result = BfsTraversal::traverse_with_limit(&graph, start, 2).unwrap();
        assert!(result.depth_levels.len() <= 3);
    }

    #[test]
    fn test_topo_sort() {
        let graph = create_test_graph();
        let sorted = TopoSort::sort(&graph).unwrap();
        assert_eq!(sorted.len(), graph.node_count());
    }

    #[test]
    fn test_no_cycle() {
        let graph = create_test_graph();
        let has_cycle = TopoSort::has_cycle(&graph).unwrap();
        assert!(!has_cycle);
    }
}
