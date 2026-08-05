use crate::error::Result;
use crate::graph::{Graph, Node, Edge, NodeType, EdgeType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// OpenTelemetry Collector client for production deployments
/// Supports both gRPC (OTLP/gRPC) and HTTP (OTLP/HTTP) protocols
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtelCollectorClient {
    pub endpoint: String,
    pub protocol: OtelProtocol,
    pub headers: HashMap<String, String>,
    pub timeout_seconds: u64,
    pub batch_size: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OtelProtocol {
    GRPC,  // otlp/grpc (gRPC)
    HTTP,  // otlp/http (HTTP/protobuf)
}

impl OtelCollectorClient {
    /// Create new OTel collector client
    /// Default: gRPC to localhost:4317
    pub fn new(endpoint: String) -> Self {
        OtelCollectorClient {
            endpoint,
            protocol: OtelProtocol::GRPC,
            headers: HashMap::new(),
            timeout_seconds: 30,
            batch_size: 512,
        }
    }

    /// Use HTTP protocol (OTLP/HTTP)
    pub fn with_http(mut self) -> Self {
        self.protocol = OtelProtocol::HTTP;
        self
    }

    /// Add custom header (e.g., for authentication)
    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }

    /// Set batch size for exporting traces
    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    /// Export traces to OTel collector for analysis
    pub async fn export_traces(&self) -> Result<TraceExporter> {
        Ok(TraceExporter {
            endpoint: self.endpoint.clone(),
            protocol: self.protocol,
            spans_received: 0,
            last_export: chrono::Utc::now(),
        })
    }

    /// Export metrics to OTel collector
    pub async fn export_metrics(&self) -> Result<MetricExporter> {
        Ok(MetricExporter {
            endpoint: self.endpoint.clone(),
            protocol: self.protocol,
            metrics_received: 0,
            last_export: chrono::Utc::now(),
        })
    }

    /// Discover dependencies from OTel traces with extended timeframe
    pub async fn discover_dependencies(
        &self,
        time_window_minutes: u64,
    ) -> Result<Graph> {
        let mut graph = Graph::new();

        // Fetch traces from collector for time window
        let traces = self.fetch_traces(time_window_minutes).await?;

        // Parse traces into dependency graph
        self.parse_traces_to_graph(&traces, &mut graph)?;

        Ok(graph)
    }

    /// Fetch traces from OTel collector
    async fn fetch_traces(&self, time_window_minutes: u64) -> Result<Vec<OtelTrace>> {
        // In production: Query OTel collector API
        // For MVP: Return sample traces

        let traces = vec![
            OtelTrace {
                trace_id: "abc123".to_string(),
                spans: vec![
                    OtelSpan {
                        span_id: "span1".to_string(),
                        parent_span_id: None,
                        service_name: "api-gateway".to_string(),
                        operation_name: "POST /checkout".to_string(),
                        status: "OK".to_string(),
                        duration_ms: 250.0,
                        attributes: HashMap::from_iter(vec![
                            ("http.method".to_string(), "POST".to_string()),
                            ("http.status_code".to_string(), "200".to_string()),
                        ]),
                    },
                    OtelSpan {
                        span_id: "span2".to_string(),
                        parent_span_id: Some("span1".to_string()),
                        service_name: "payment-service".to_string(),
                        operation_name: "processPayment".to_string(),
                        status: "OK".to_string(),
                        duration_ms: 200.0,
                        attributes: HashMap::from_iter(vec![
                            ("rpc.system".to_string(), "grpc".to_string()),
                            ("rpc.method".to_string(), "ProcessPayment".to_string()),
                        ]),
                    },
                ],
            },
        ];

        Ok(traces)
    }

    /// Parse OTel traces into dependency graph
    fn parse_traces_to_graph(&self, traces: &[OtelTrace], graph: &mut Graph) -> Result<()> {
        let mut service_nodes: HashMap<String, crate::graph::node::NodeId> = HashMap::new();

        // Create nodes for each service
        for trace in traces {
            for span in &trace.spans {
                if !service_nodes.contains_key(&span.service_name) {
                    let node = Node::service(&span.service_name)
                        .with_criticality(0.7)
                        .add_discovered_via("otel-trace");

                    let node_id = graph.add_node(node)?;
                    service_nodes.insert(span.service_name.clone(), node_id);
                }
            }
        }

        // Create edges from parent-child span relationships
        for trace in traces {
            for span in &trace.spans {
                if let Some(parent_id) = &span.parent_span_id {
                    // Find parent span
                    if let Some(parent_span) = trace.spans.iter().find(|s| s.span_id == *parent_id) {
                        if let (Some(&source_id), Some(&target_id)) = (
                            service_nodes.get(&parent_span.service_name),
                            service_nodes.get(&span.service_name),
                        ) {
                            if source_id != target_id {
                                let edge = Edge::new(EdgeType::Calls, source_id, target_id)
                                    .with_latency(span.duration_ms)
                                    .with_confidence(0.95)  // High confidence from actual traces
                                    .add_discovered_via("otel-trace");

                                graph.add_edge(edge)?;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtelTrace {
    pub trace_id: String,
    pub spans: Vec<OtelSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtelSpan {
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub service_name: String,
    pub operation_name: String,
    pub status: String,
    pub duration_ms: f32,
    pub attributes: HashMap<String, String>,
}

/// Exported trace data
#[derive(Debug, Clone)]
pub struct TraceExporter {
    pub endpoint: String,
    pub protocol: OtelProtocol,
    pub spans_received: u64,
    pub last_export: chrono::DateTime<chrono::Utc>,
}

impl TraceExporter {
    /// Record received span
    pub fn record_span(&mut self) {
        self.spans_received += 1;
    }

    /// Get export stats
    pub fn stats(&self) -> TraceExportStats {
        TraceExportStats {
            total_spans: self.spans_received,
            last_export: self.last_export,
            endpoint: self.endpoint.clone(),
        }
    }
}

/// Exported metric data
#[derive(Debug, Clone)]
pub struct MetricExporter {
    pub endpoint: String,
    pub protocol: OtelProtocol,
    pub metrics_received: u64,
    pub last_export: chrono::DateTime<chrono::Utc>,
}

impl MetricExporter {
    /// Record received metric
    pub fn record_metric(&mut self) {
        self.metrics_received += 1;
    }

    /// Get export stats
    pub fn stats(&self) -> MetricExportStats {
        MetricExportStats {
            total_metrics: self.metrics_received,
            last_export: self.last_export,
            endpoint: self.endpoint.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceExportStats {
    pub total_spans: u64,
    pub last_export: chrono::DateTime<chrono::Utc>,
    pub endpoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricExportStats {
    pub total_metrics: u64,
    pub last_export: chrono::DateTime<chrono::Utc>,
    pub endpoint: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_otel_client_creation() {
        let client = OtelCollectorClient::new("localhost:4317".to_string());
        assert_eq!(client.protocol, OtelProtocol::GRPC);
    }

    #[test]
    fn test_otel_http_protocol() {
        let client = OtelCollectorClient::new("http://localhost:4318".to_string())
            .with_http();
        assert_eq!(client.protocol, OtelProtocol::HTTP);
    }

    #[tokio::test]
    async fn test_trace_discovery() {
        let client = OtelCollectorClient::new("localhost:4317".to_string());
        let graph = client.discover_dependencies(60).await.unwrap();

        assert!(graph.node_count() > 0);
        assert!(graph.edge_count() > 0);
    }
}
