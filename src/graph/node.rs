use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub Uuid);

impl NodeId {
    pub fn new() -> Self {
        NodeId(Uuid::new_v4())
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeType {
    Service,
    Database,
    Pipeline,
    Topic,
    VectorDb,
    Dashboard,
    Agent,
    ExternalService,
    Table,
    Cache,
    Queue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeProperties {
    pub criticality: f32, // 0.0-1.0
    pub health_status: HealthStatus,
    pub recovery_time_minutes: u32,
    pub business_exposure: f32, // 0.0-1.0
    pub tags: HashMap<String, String>,
    pub discovered_via: Vec<String>, // Which discovery methods found this
    pub confidence: f32,             // 0.0-1.0
    pub owner: Option<String>,
    pub on_call: Option<String>,
    pub pagerduty_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

impl Default for NodeProperties {
    fn default() -> Self {
        NodeProperties {
            criticality: 0.5,
            health_status: HealthStatus::Unknown,
            recovery_time_minutes: 15,
            business_exposure: 0.5,
            tags: HashMap::new(),
            discovered_via: vec![],
            confidence: 0.5,
            owner: None,
            on_call: None,
            pagerduty_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub name: String,
    pub node_type: NodeType,
    pub properties: NodeProperties,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Node {
    pub fn new(name: impl Into<String>, node_type: NodeType) -> Self {
        Node {
            id: NodeId::new(),
            name: name.into(),
            node_type,
            properties: NodeProperties::default(),
            created_at: chrono::Utc::now(),
        }
    }

    pub fn service(name: impl Into<String>) -> Self {
        Node::new(name, NodeType::Service)
    }

    pub fn database(name: impl Into<String>) -> Self {
        Node::new(name, NodeType::Database)
    }

    pub fn pipeline(name: impl Into<String>) -> Self {
        Node::new(name, NodeType::Pipeline)
    }

    pub fn topic(name: impl Into<String>) -> Self {
        Node::new(name, NodeType::Topic)
    }

    pub fn vector_db(name: impl Into<String>) -> Self {
        Node::new(name, NodeType::VectorDb)
    }

    pub fn agent(name: impl Into<String>) -> Self {
        Node::new(name, NodeType::Agent)
    }

    pub fn external_service(name: impl Into<String>) -> Self {
        Node::new(name, NodeType::ExternalService)
    }

    pub fn table(name: impl Into<String>) -> Self {
        Node::new(name, NodeType::Table)
    }

    pub fn with_criticality(mut self, criticality: f32) -> Self {
        self.properties.criticality = criticality.clamp(0.0, 1.0);
        self
    }

    pub fn with_health(mut self, status: HealthStatus) -> Self {
        self.properties.health_status = status;
        self
    }

    pub fn with_recovery_time(mut self, minutes: u32) -> Self {
        self.properties.recovery_time_minutes = minutes;
        self
    }

    pub fn with_business_exposure(mut self, exposure: f32) -> Self {
        self.properties.business_exposure = exposure.clamp(0.0, 1.0);
        self
    }

    pub fn with_owner(mut self, owner: impl Into<String>) -> Self {
        self.properties.owner = Some(owner.into());
        self
    }

    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.properties.tags.insert(key.into(), value.into());
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_creation() {
        let node = Node::service("api-server");
        assert_eq!(node.name, "api-server");
        assert_eq!(node.node_type, NodeType::Service);
    }

    #[test]
    fn test_node_builder() {
        let node = Node::database("postgresql")
            .with_criticality(0.9)
            .with_health(HealthStatus::Healthy)
            .with_owner("data-team");

        assert_eq!(node.properties.criticality, 0.9);
        assert_eq!(node.properties.health_status, HealthStatus::Healthy);
        assert_eq!(node.properties.owner, Some("data-team".to_string()));
    }
}
