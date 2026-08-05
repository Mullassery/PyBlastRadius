use crate::error::Result;
use crate::graph::{Graph, Node, Edge, NodeType, EdgeType};
use std::collections::HashMap;

/// OpenTelemetry (OTLP) trace consumer for runtime dependency discovery
pub struct OtlpDiscoverer {
    pub endpoint: String,
    pub timeout_seconds: u64,
}

impl OtlpDiscoverer {
    pub fn new(endpoint: String) -> Self {
        OtlpDiscoverer {
            endpoint,
            timeout_seconds: 30,
        }
    }

    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    /// Discover runtime dependencies from OTLP traces
    /// In production, this would connect to an OTLP Collector endpoint
    pub fn discover(&self) -> Result<Graph> {
        let mut graph = Graph::new();
        let mut node_map: HashMap<String, crate::graph::node::NodeId> = HashMap::new();

        // In MVP: Load traces from OTLP endpoint (async)
        // For now: Simulate with sample traces

        self.parse_traces(&mut graph, &mut node_map)?;

        Ok(graph)
    }

    fn parse_traces(
        &self,
        graph: &mut Graph,
        node_map: &mut HashMap<String, crate::graph::node::NodeId>,
    ) -> Result<()> {
        // In production, this would:
        // 1. Connect to OTLP endpoint (gRPC or HTTP)
        // 2. Request traces with time window
        // 3. Parse ResourceSpans -> ScopeSpans -> Spans
        // 4. Extract service names from resource attributes
        // 5. Extract span relationships (parent-child)

        // For MVP: Simulate common trace patterns
        let traces = vec![
            ("api-gateway", "auth-service", EdgeType::Calls, 45.0, 0.01),
            ("auth-service", "postgres", EdgeType::Queries, 120.0, 0.02),
            ("auth-service", "redis", EdgeType::Calls, 5.0, 0.0),
            ("api-gateway", "checkout-service", EdgeType::Calls, 60.0, 0.015),
            ("checkout-service", "payment-service", EdgeType::Calls, 200.0, 0.03),
        ];

        for (source_name, target_name, edge_type, latency_ms, error_rate) in traces {
            // Create source node if not exists
            if !node_map.contains_key(source_name) {
                let node = Node::service(source_name)
                    .with_criticality(0.7)
                    .add_discovered_via("otlp-trace");

                let node_id = graph.add_node(node)?;
                node_map.insert(source_name.to_string(), node_id);
            }

            // Create target node if not exists
            if !node_map.contains_key(target_name) {
                let node_type = if target_name == "postgres" || target_name == "redis" {
                    if target_name == "postgres" {
                        NodeType::Database
                    } else {
                        NodeType::Cache
                    }
                } else {
                    NodeType::Service
                };

                let node = Node::new(target_name, node_type)
                    .with_criticality(if target_name == "postgres" { 0.95 } else { 0.8 })
                    .add_discovered_via("otlp-trace");

                let node_id = graph.add_node(node)?;
                node_map.insert(target_name.to_string(), node_id);
            }

            // Create edge with observed latency and error rate
            if let (Some(&source_id), Some(&target_id)) = (node_map.get(source_name), node_map.get(target_name)) {
                let edge = Edge::new(edge_type, source_id, target_id)
                    .with_latency(latency_ms)
                    .with_error_rate(error_rate)
                    .with_confidence(0.95)  // High confidence from actual traces
                    .add_discovered_via("otlp");

                graph.add_edge(edge)?;
            }
        }

        Ok(())
    }

    /// Async version for real deployments
    pub async fn discover_async(&self) -> Result<Graph> {
        // TODO: Implement async OTLP client
        // Use tonic for gRPC or reqwest for HTTP
        self.discover()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_otlp_discovery() {
        let discoverer = OtlpDiscoverer::new("http://localhost:4317".to_string());
        let graph = discoverer.discover().unwrap();

        // Should have nodes from simulated traces
        assert!(graph.node_count() > 0);
        assert!(graph.edge_count() > 0);
    }

    #[test]
    fn test_otlp_with_timeout() {
        let discoverer = OtlpDiscoverer::new("http://localhost:4317".to_string())
            .with_timeout(60);

        assert_eq!(discoverer.timeout_seconds, 60);
    }
}
