/// GraphQL API and REST integrations
/// Phase 3 (v1.0) feature set

pub mod graphql;
pub mod integrations;

pub use graphql::{GraphQLSchema, GraphQLQuery};
pub use integrations::{SlackIntegration, PagerDutyIntegration};

/// GraphQL schema for querying blast radius data
pub const GRAPHQL_SCHEMA: &str = r#"
type Query {
  # Get node by ID
  node(id: ID!): Node

  # Get all nodes
  nodes(limit: Int, offset: Int): [Node!]!

  # Analyze blast radius for a service
  blastRadius(nodeId: ID!, maxDepth: Int): BlastRadiusResult!

  # Get criticality ranking
  criticality(limit: Int): [CriticalityScore!]!

  # Detect anomalies
  anomalies(minSeverity: Float): [DetectedAnomaly!]!

  # Find hidden dependencies
  hiddenDependencies(minConfidence: Float): [HiddenDependency!]!

  # System metrics
  metrics: SystemMetrics!
}

type Mutation {
  # Update node properties
  updateNode(id: ID!, criticality: Float, healthStatus: String): Node!

  # Trigger blast radius analysis
  analyzeService(name: String!): AnalysisResult!

  # Simulate failure
  simulateFailure(nodeId: ID!, failureType: String!): SimulationResult!

  # Add manual dependency
  addDependency(sourceId: ID!, targetId: ID!, edgeType: String!): Edge!
}

type Node {
  id: ID!
  name: String!
  type: NodeType!
  criticality: Float!
  healthStatus: HealthStatus!
  recoveryTimeMinutes: Int!
  businessExposure: Float!
  tags: [KeyValue!]!
  dependencies: [Node!]!
  dependents: [Node!]!
}

type Edge {
  id: ID!
  source: Node!
  target: Node!
  type: EdgeType!
  isCritical: Boolean!
  latencyMs: Float
  errorRate: Float
  confidence: Float!
}

type BlastRadiusResult {
  nodeId: ID!
  directlyImpacted: [Node!]!
  indirectlyImpacted: [Node!]!
  tertiaryImpacted: [Node!]!
  score: Float!
  criticalPaths: Int!
}

type CriticalityScore {
  nodeId: ID!
  nodeName: String!
  score: Float!
  downstreamCount: Int!
  upstreamCount: Int!
  isSinglePointOfFailure: Boolean!
}

type DetectedAnomaly {
  nodeId: ID!
  nodeName: String!
  type: AnomalyType!
  severity: Float!
  confidence: Float!
  predictedTTF: Int!
  evidence: [String!]!
}

type HiddenDependency {
  source: Node!
  target: Node!
  type: EdgeType!
  confidence: Float!
  detectionMethod: String!
  evidence: [String!]!
}

type SimulationResult {
  nodeId: ID!
  cascadePath: [String!]!
  estimatedImpactNodes: Int!
  estimatedRecoveryMinutes: Int!
  businessImpactScore: Float!
}

type AnalysisResult {
  success: Boolean!
  blastRadius: BlastRadiusResult!
  anomalies: [DetectedAnomaly!]!
  recommendations: [String!]!
}

type SystemMetrics {
  nodeCount: Int!
  edgeCount: Int!
  avgDegree: Float!
  density: Float!
  connectedComponents: Int!
  lastUpdated: String!
}

type KeyValue {
  key: String!
  value: String!
}

enum NodeType {
  SERVICE
  DATABASE
  PIPELINE
  TOPIC
  VECTOR_DB
  AGENT
  CACHE
  QUEUE
}

enum EdgeType {
  CALLS
  PRODUCES
  CONSUMES
  QUERIES
  FEEDS
  DEPENDS_ON
  TRANSFORMS
  EXPORTS_TO
  RETRIEVES
}

enum HealthStatus {
  HEALTHY
  DEGRADED
  UNHEALTHY
  UNKNOWN
}

enum AnomalyType {
  LATENCY_SPIKE
  ERROR_RATE_INCREASE
  THROUGHPUT_DROP
  RESOURCE_EXHAUSTION
  TRAFFIC_ANOMALY
  HEALTH_CHECK_FAILURE
}
"#;
