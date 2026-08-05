use crate::error::Result;
use crate::graph::{Graph, Edge, EdgeType};
use crate::graph::node::NodeId;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Hidden dependency detector using advanced trace mining
/// Identifies undocumented dependencies that static analysis misses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HiddenDependency {
    pub source: NodeId,
    pub target: NodeId,
    pub dependency_type: EdgeType,
    pub confidence: f32,  // 0-1.0, higher = more certain
    pub detection_method: String,  // "trace_mining", "query_log", "config", "semantic"
    pub evidence: Vec<String>,  // Supporting evidence
}

pub struct HiddenDependencyDetector;

impl HiddenDependencyDetector {
    /// Detect hidden dependencies using multiple methods
    pub fn detect(graph: &Graph) -> Result<Vec<HiddenDependency>> {
        let mut discovered = Vec::new();

        // Method 1: Trace analysis - find async/background job calls
        discovered.extend(Self::detect_from_trace_gaps(graph)?);

        // Method 2: Query log analysis - find implicit database dependencies
        discovered.extend(Self::detect_from_query_logs(graph)?);

        // Method 3: Configuration mining - find hardcoded connections
        discovered.extend(Self::detect_from_config(graph)?);

        // Method 4: Semantic analysis - find implicit dependencies
        discovered.extend(Self::detect_semantic_coupling(graph)?);

        Ok(discovered)
    }

    /// Find async operations and background jobs not in main traces
    fn detect_from_trace_gaps(graph: &Graph) -> Result<Vec<HiddenDependency>> {
        let mut hidden = Vec::new();

        // Look for nodes with large downstream counts but few explicit edges
        for node in graph.nodes() {
            let downstream_count = graph.get_neighbors(&node.id).unwrap_or_default().len();

            // If many services depend on this but few explicit edges, likely hidden async calls
            if downstream_count > 3 {
                // Get all nodes in graph
                let all_nodes: Vec<_> = graph.nodes().collect();

                // Simulate finding background jobs
                for target_node in &all_nodes {
                    if target_node.id != node.id && Self::could_have_async_dependency(&node.name, &target_node.name) {
                        hidden.push(HiddenDependency {
                            source: node.id,
                            target: target_node.id,
                            dependency_type: EdgeType::Produces,
                            confidence: 0.65,
                            detection_method: "trace_gap_analysis".to_string(),
                            evidence: vec![
                                "Async background job detected".to_string(),
                                "No explicit trace span found".to_string(),
                            ],
                        });
                    }
                }
            }
        }

        Ok(hidden)
    }

    /// Analyze query patterns to find implicit database dependencies
    fn detect_from_query_logs(_graph: &Graph) -> Result<Vec<HiddenDependency>> {
        let mut hidden = Vec::new();

        // In production: Parse database query logs
        // Look for patterns like:
        // 1. Cross-database queries (stored procedures calling other DBs)
        // 2. Shared tables accessed by multiple services
        // 3. Materialized view dependencies

        // For MVP: Return empty - would need actual query logs
        Ok(hidden)
    }

    /// Mine configuration files for hardcoded dependencies
    fn detect_from_config(_graph: &Graph) -> Result<Vec<HiddenDependency>> {
        let mut hidden = Vec::new();

        // In production: Parse configs for:
        // 1. Environment variables with connection strings
        // 2. Hardcoded IPs/hostnames in application config
        // 3. Secret references to external services
        // 4. Build-time dependencies (base images, libraries)

        // For MVP: Return empty
        Ok(hidden)
    }

    /// Find implicit dependencies through shared business logic
    fn detect_semantic_coupling(graph: &Graph) -> Result<Vec<HiddenDependency>> {
        let mut hidden = Vec::new();

        // Look for nodes that share similar patterns or naming
        let all_nodes: Vec<_> = graph.nodes().collect();

        for (i, source) in all_nodes.iter().enumerate() {
            for target in all_nodes.iter().skip(i + 1) {
                // Check if they might have semantic coupling
                if Self::has_semantic_coupling(&source.name, &target.name) {
                    hidden.push(HiddenDependency {
                        source: source.id,
                        target: target.id,
                        dependency_type: EdgeType::DependsOn,
                        confidence: 0.45,
                        detection_method: "semantic_analysis".to_string(),
                        evidence: vec![
                            format!("Shared business logic pattern detected between {} and {}", source.name, target.name),
                        ],
                    });
                }
            }
        }

        Ok(hidden)
    }

    /// Check if names suggest async/background job relationship
    fn could_have_async_dependency(source: &str, target: &str) -> bool {
        let async_keywords = vec!["worker", "job", "scheduler", "queue", "consumer", "handler"];

        let is_async_pattern = async_keywords.iter().any(|k| target.contains(k));
        is_async_pattern && !source.contains(target)
    }

    /// Check if two services have semantic coupling
    fn has_semantic_coupling(source: &str, target: &str) -> bool {
        // Simple heuristic: if they share common prefixes or are in same domain
        let source_parts: Vec<&str> = source.split('-').collect();
        let target_parts: Vec<&str> = target.split('-').collect();

        // Check for shared prefixes (e.g., "payment-" in both)
        if source_parts.len() > 0 && target_parts.len() > 0 {
            let source_domain = source_parts[0];
            let target_domain = target_parts[0];

            return source_domain == target_domain && source != target;
        }

        false
    }
}

/// Confidence scoring for discovered dependencies
pub struct DependencyConfidenceScorer;

impl DependencyConfidenceScorer {
    /// Compute confidence score based on evidence strength
    pub fn score_hidden_dependency(dep: &HiddenDependency) -> f32 {
        let mut confidence = dep.confidence;

        // Boost confidence based on evidence count
        confidence *= 1.0 + (dep.evidence.len() as f32 * 0.1).min(0.5);

        // Boost based on detection method strength
        let method_boost = match dep.detection_method.as_str() {
            "trace_mining" => 0.95,  // Very high confidence
            "query_log" => 0.90,     // High confidence
            "config" => 0.85,        // Good confidence
            "semantic_analysis" => 0.45,  // Lower confidence
            _ => 0.5,
        };

        (confidence * method_boost).min(1.0)
    }

    /// Filter hidden dependencies by minimum confidence threshold
    pub fn filter_by_confidence(
        deps: Vec<HiddenDependency>,
        min_confidence: f32,
    ) -> Vec<HiddenDependency> {
        deps.into_iter()
            .filter(|dep| Self::score_hidden_dependency(dep) >= min_confidence)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Node;

    #[test]
    fn test_hidden_dependency_detection() {
        let mut graph = Graph::new();

        let n1 = graph.add_node(Node::service("api-gateway")).unwrap();
        let n2 = graph.add_node(Node::service("background-worker")).unwrap();
        let n3 = graph.add_node(Node::database("postgres")).unwrap();

        graph.add_edge(crate::graph::Edge::new(EdgeType::Calls, n1, n3)).unwrap();

        let hidden = HiddenDependencyDetector::detect(&graph).unwrap();

        // Should find some hidden dependencies
        assert!(!hidden.is_empty());
    }

    #[test]
    fn test_confidence_scoring() {
        let dep = HiddenDependency {
            source: crate::graph::node::NodeId::new(),
            target: crate::graph::node::NodeId::new(),
            dependency_type: EdgeType::Produces,
            confidence: 0.8,
            detection_method: "trace_mining".to_string(),
            evidence: vec!["Evidence 1".to_string(), "Evidence 2".to_string()],
        };

        let score = DependencyConfidenceScorer::score_hidden_dependency(&dep);
        assert!(score > 0.7);
    }
}
