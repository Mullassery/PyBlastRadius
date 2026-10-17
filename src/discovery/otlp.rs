use crate::error::{Error, Result};
use crate::graph::node::NodeId;
use crate::graph::{Edge, EdgeType, Graph, Node};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use opentelemetry_proto::tonic::collector::trace::v1::{
    trace_service_server::{TraceService, TraceServiceServer},
    ExportTraceServiceRequest, ExportTraceServiceResponse,
};
use opentelemetry_proto::tonic::common::v1::{any_value::Value as AnyValueKind, KeyValue};
use opentelemetry_proto::tonic::trace::v1::{ResourceSpans, Span};
use tonic::{transport::Server, Request, Response, Status};

/// OpenTelemetry (OTLP) trace consumer for runtime dependency discovery.
///
/// Runs a real OTLP/gRPC trace receiver (the standard
/// `TraceService.Export` RPC) bound to `endpoint`, collects whatever spans
/// are actually pushed to it by instrumented services during a
/// `timeout_seconds`-long collection window, then builds the dependency
/// graph from those real spans: a parent/child span pair that crosses a
/// service boundary becomes a `Calls` edge, annotated with the span's
/// actual observed latency and error status. If nothing is received in the
/// window, the result is an empty graph — not a fabricated one.
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

    /// Blocking wrapper around [`discover_async`] for callers outside an
    /// async runtime (e.g. the CLI).
    pub fn discover(&self) -> Result<Graph> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| Error::DiscoveryError(format!("failed to start tokio runtime: {e}")))?;
        runtime.block_on(self.discover_async())
    }

    /// Real async OTLP receiver: listens on `self.endpoint`, collects spans
    /// pushed to it for `self.timeout_seconds`, then builds a graph from
    /// what was actually received.
    pub async fn discover_async(&self) -> Result<Graph> {
        let addr: SocketAddr = self.endpoint.parse().map_err(|e| {
            Error::DiscoveryError(format!(
                "invalid OTLP listen address {:?}: {e}",
                self.endpoint
            ))
        })?;

        let collected: Arc<Mutex<Vec<ResourceSpans>>> = Arc::new(Mutex::new(Vec::new()));
        let receiver = TraceReceiver {
            collected: collected.clone(),
        };

        let server = Server::builder()
            .add_service(TraceServiceServer::new(receiver))
            .serve(addr);

        tokio::select! {
            result = server => {
                if let Err(e) = result {
                    return Err(Error::DiscoveryError(format!("OTLP receiver failed: {e}")));
                }
            }
            _ = tokio::time::sleep(Duration::from_secs(self.timeout_seconds)) => {
                // Collection window elapsed; stop listening and build the
                // graph from whatever was actually received.
            }
        }

        let spans = collected
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        build_graph(&spans)
    }
}

impl super::DiscoveryProvider for OtlpDiscoverer {
    fn scheme(&self) -> &'static str {
        "otlp"
    }

    fn description(&self) -> &'static str {
        "Bind a real OTLP/gRPC trace receiver and build edges from cross-service parent/child spans received during the collection window."
    }

    fn discover(&self) -> Result<Graph> {
        OtlpDiscoverer::discover(self)
    }
}

fn build_graph(resource_spans: &[ResourceSpans]) -> Result<Graph> {
    let mut graph = Graph::new();
    let mut node_ids: HashMap<String, NodeId> = HashMap::new();
    let mut span_service: HashMap<Vec<u8>, String> = HashMap::new();

    // First pass: record which service each observed span belongs to.
    for rs in resource_spans {
        let service_name = service_name(rs).unwrap_or_else(|| "unknown-service".to_string());
        for scope_spans in &rs.scope_spans {
            for span in &scope_spans.spans {
                span_service.insert(span.span_id.clone(), service_name.clone());
            }
        }
    }

    // Second pass: a parent -> child span pair that crosses a service
    // boundary is a real observed dependency. Same-service spans (internal
    // work within one process) are not graph edges.
    for rs in resource_spans {
        for scope_spans in &rs.scope_spans {
            for span in &scope_spans.spans {
                if span.parent_span_id.is_empty() {
                    continue;
                }
                let Some(parent_service) = span_service.get(&span.parent_span_id) else {
                    continue; // parent span wasn't captured in this window
                };
                let Some(child_service) = span_service.get(&span.span_id) else {
                    continue;
                };
                if parent_service == child_service {
                    continue;
                }

                let source_id = ensure_node(&mut graph, &mut node_ids, parent_service)?;
                let target_id = ensure_node(&mut graph, &mut node_ids, child_service)?;

                let latency_ms = if span.end_time_unix_nano > span.start_time_unix_nano {
                    (span.end_time_unix_nano - span.start_time_unix_nano) as f32 / 1_000_000.0
                } else {
                    0.0
                };
                let error_rate = if span_is_error(span) { 1.0 } else { 0.0 };

                let edge = Edge::new(EdgeType::Calls, source_id, target_id)
                    .with_latency(latency_ms)
                    .with_error_rate(error_rate)
                    .with_confidence(0.95) // directly observed from a real trace
                    .add_discovered_via("otlp");
                graph.add_edge(edge)?;
            }
        }
    }

    Ok(graph)
}

fn ensure_node(
    graph: &mut Graph,
    node_ids: &mut HashMap<String, NodeId>,
    name: &str,
) -> Result<NodeId> {
    if let Some(&id) = node_ids.get(name) {
        return Ok(id);
    }
    let node = Node::service(name)
        .with_criticality(0.7)
        .add_discovered_via("otlp-trace");
    let id = graph.add_node(node)?;
    node_ids.insert(name.to_string(), id);
    Ok(id)
}

fn service_name(rs: &ResourceSpans) -> Option<String> {
    let resource = rs.resource.as_ref()?;
    string_attr(&resource.attributes, "service.name")
}

fn string_attr(attrs: &[KeyValue], key: &str) -> Option<String> {
    attrs.iter().find(|kv| kv.key == key).and_then(|kv| {
        match kv.value.as_ref()?.value.as_ref()? {
            AnyValueKind::StringValue(s) => Some(s.clone()),
            _ => None,
        }
    })
}

fn span_is_error(span: &Span) -> bool {
    // OTLP StatusCode: 2 == STATUS_CODE_ERROR
    span.status.as_ref().map(|s| s.code == 2).unwrap_or(false)
}

#[derive(Clone)]
struct TraceReceiver {
    collected: Arc<Mutex<Vec<ResourceSpans>>>,
}

#[tonic::async_trait]
impl TraceService for TraceReceiver {
    async fn export(
        &self,
        request: Request<ExportTraceServiceRequest>,
    ) -> std::result::Result<Response<ExportTraceServiceResponse>, Status> {
        let req = request.into_inner();
        if let Ok(mut collected) = self.collected.lock() {
            collected.extend(req.resource_spans);
        }
        Ok(Response::new(ExportTraceServiceResponse {
            partial_success: None,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry_proto::tonic::common::v1::{any_value::Value, AnyValue};
    use opentelemetry_proto::tonic::resource::v1::Resource;
    use opentelemetry_proto::tonic::trace::v1::{ScopeSpans, Status as OtlpStatus};

    fn resource_spans(service: &str, spans: Vec<Span>) -> ResourceSpans {
        ResourceSpans {
            resource: Some(Resource {
                attributes: vec![KeyValue {
                    key: "service.name".to_string(),
                    value: Some(AnyValue {
                        value: Some(Value::StringValue(service.to_string())),
                    }),
                    ..Default::default()
                }],
                ..Default::default()
            }),
            scope_spans: vec![ScopeSpans {
                scope: None,
                spans,
                schema_url: String::new(),
            }],
            schema_url: String::new(),
        }
    }

    fn span(id: &[u8], parent: &[u8], start_ns: u64, end_ns: u64, is_error: bool) -> Span {
        Span {
            trace_id: vec![1; 16],
            span_id: id.to_vec(),
            parent_span_id: parent.to_vec(),
            name: "span".to_string(),
            start_time_unix_nano: start_ns,
            end_time_unix_nano: end_ns,
            status: Some(OtlpStatus {
                code: if is_error { 2 } else { 1 },
                message: String::new(),
            }),
            ..Default::default()
        }
    }

    #[test]
    fn test_empty_window_produces_empty_graph() {
        let graph = build_graph(&[]).unwrap();
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_cross_service_span_becomes_calls_edge() {
        let root = span(b"parent00", b"", 0, 1_000_000, false);
        let child = span(b"child000", b"parent00", 0, 45_000_000, false);

        let resource_spans = vec![
            resource_spans("api-gateway", vec![root]),
            resource_spans("auth-service", vec![child]),
        ];

        let graph = build_graph(&resource_spans).unwrap();

        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn test_same_service_span_is_not_an_edge() {
        let root = span(b"parent00", b"", 0, 1_000_000, false);
        let child = span(b"child000", b"parent00", 0, 1_000_000, false);

        // Both spans belong to the same resource/service.
        let resource_spans = vec![resource_spans("api-gateway", vec![root, child])];

        let graph = build_graph(&resource_spans).unwrap();

        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_latency_is_derived_from_real_span_timing() {
        let root = span(b"parent00", b"", 0, 0, false);
        // 45ms span duration, in nanoseconds.
        let child = span(
            b"child000",
            b"parent00",
            1_000_000_000,
            1_045_000_000,
            false,
        );

        let resource_spans = vec![
            resource_spans("api-gateway", vec![root]),
            resource_spans("auth-service", vec![child]),
        ];

        let graph = build_graph(&resource_spans).unwrap();
        let edge = graph.edges().next().expect("expected one edge");
        assert!((edge.properties.latency_ms.unwrap_or(0.0) - 45.0).abs() < 0.01);
    }

    #[test]
    fn test_error_status_is_reflected_in_error_rate() {
        let root = span(b"parent00", b"", 0, 1_000_000, false);
        let child = span(b"child000", b"parent00", 0, 1_000_000, true);

        let resource_spans = vec![
            resource_spans("api-gateway", vec![root]),
            resource_spans("payment-service", vec![child]),
        ];

        let graph = build_graph(&resource_spans).unwrap();
        let edge = graph.edges().next().expect("expected one edge");
        assert_eq!(edge.properties.error_rate, Some(1.0));
    }

    #[test]
    fn test_orphaned_child_span_without_captured_parent_is_skipped() {
        // Parent span was never captured (e.g. sampled out upstream) —
        // we should not fabricate an edge from partial data.
        let child = span(b"child000", b"missing0", 0, 1_000_000, false);
        let resource_spans = vec![resource_spans("auth-service", vec![child])];

        let graph = build_graph(&resource_spans).unwrap();
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_missing_service_name_falls_back_honestly() {
        let root = ResourceSpans {
            resource: Some(Resource {
                attributes: vec![],
                ..Default::default()
            }),
            scope_spans: vec![ScopeSpans {
                scope: None,
                spans: vec![span(b"parent00", b"", 0, 1_000_000, false)],
                schema_url: String::new(),
            }],
            schema_url: String::new(),
        };
        let child = resource_spans(
            "known-service",
            vec![span(b"child000", b"parent00", 0, 1_000_000, false)],
        );

        let graph = build_graph(&[root, child]).unwrap();

        assert_eq!(graph.node_count(), 2);
        assert!(graph.nodes().any(|n| n.name == "unknown-service"));
    }
}
