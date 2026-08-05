use serde::{Deserialize, Serialize};
use super::node::NodeId;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EdgeId(pub Uuid);

impl EdgeId {
    pub fn new() -> Self {
        EdgeId(Uuid::new_v4())
    }
}

impl Default for EdgeId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeType {
    Calls,           // A calls B (sync RPC)
    Produces,        // A produces to B (async, Kafka/queue)
    Consumes,        // A consumes from B
    Queries,         // A queries B (database)
    Feeds,           // A feeds data to B
    DependsOn,       // A depends on B
    Transforms,      // A transforms data from B (data pipeline)
    ExportsTo,       // A exports to B (reverse ETL)
    Retrieves,       // A retrieves from B (vector DB)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeProperties {
    pub latency_ms: Option<f32>,
    pub error_rate: Option<f32>, // 0.0-1.0
    pub throughput: Option<f32>,
    pub is_critical: bool,
    pub is_async: bool,
    pub has_fallback: bool,
    pub has_retry: bool,
    pub timeout_ms: Option<f32>,
    pub confidence: f32, // 0.0-1.0
    pub discovered_via: Vec<String>, // Which discovery methods found this
    pub tags: HashMap<String, String>,
}

impl Default for EdgeProperties {
    fn default() -> Self {
        EdgeProperties {
            latency_ms: None,
            error_rate: None,
            throughput: None,
            is_critical: true,
            is_async: false,
            has_fallback: false,
            has_retry: false,
            timeout_ms: None,
            confidence: 0.5,
            discovered_via: vec![],
            tags: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: EdgeId,
    pub edge_type: EdgeType,
    pub source: NodeId,
    pub target: NodeId,
    pub properties: EdgeProperties,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Edge {
    pub fn new(edge_type: EdgeType, source: NodeId, target: NodeId) -> Self {
        Edge {
            id: EdgeId::new(),
            edge_type,
            source,
            target,
            properties: EdgeProperties::default(),
            created_at: chrono::Utc::now(),
        }
    }

    pub fn with_criticality(mut self, is_critical: bool) -> Self {
        self.properties.is_critical = is_critical;
        self
    }

    pub fn with_latency(mut self, latency_ms: f32) -> Self {
        self.properties.latency_ms = Some(latency_ms);
        self
    }

    pub fn with_error_rate(mut self, error_rate: f32) -> Self {
        self.properties.error_rate = Some(error_rate.clamp(0.0, 1.0));
        self
    }

    pub fn with_async(mut self, is_async: bool) -> Self {
        self.properties.is_async = is_async;
        self
    }

    pub fn with_fallback(mut self) -> Self {
        self.properties.has_fallback = true;
        self
    }

    pub fn with_retry(mut self) -> Self {
        self.properties.has_retry = true;
        self
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.properties.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    pub fn add_discovered_via(mut self, method: impl Into<String>) -> Self {
        self.properties.discovered_via.push(method.into());
        self
    }

    pub fn with_timeout(mut self, timeout_ms: f32) -> Self {
        self.properties.timeout_ms = Some(timeout_ms);
        self
    }

    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.properties.tags.insert(key.into(), value.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_creation() {
        let node_id1 = NodeId::new();
        let node_id2 = NodeId::new();
        let edge = Edge::new(EdgeType::Calls, node_id1, node_id2);

        assert_eq!(edge.source, node_id1);
        assert_eq!(edge.target, node_id2);
        assert_eq!(edge.edge_type, EdgeType::Calls);
    }

    #[test]
    fn test_edge_builder() {
        let node_id1 = NodeId::new();
        let node_id2 = NodeId::new();
        let edge = Edge::new(EdgeType::Queries, node_id1, node_id2)
            .with_criticality(true)
            .with_latency(50.0)
            .with_confidence(0.95);

        assert!(edge.properties.is_critical);
        assert_eq!(edge.properties.latency_ms, Some(50.0));
        assert_eq!(edge.properties.confidence, 0.95);
    }
}
