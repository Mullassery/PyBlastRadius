# PyBlastRadius Architecture

**Status**: Phase 0 - Foundation Complete  
**Version**: v0.1.0 (MVP, Development)

---

## System Overview

PyBlastRadius is a Rust-based platform for discovering dependencies, analyzing blast radius impact, and predicting cascading failures.

```
┌─────────────────────────────────────────────────────────────┐
│                    PyBlastRadius System                      │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ 1. Discovery Layer (Multiple Sources)                │   │
│  │    ├─ Kubernetes manifests                           │   │
│  │    ├─ Terraform HCL                                  │   │
│  │    ├─ Airflow DAGs                                   │   │
│  │    └─ OTel traces (OTLP)                             │   │
│  └──────────────────────────────────────────────────────┘   │
│                          ↓                                    │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ 2. Unified Graph (petgraph-based)                    │   │
│  │    ├─ Nodes: Service, Database, Pipeline, etc       │   │
│  │    ├─ Edges: Calls, Queries, Produces, etc          │   │
│  │    └─ Properties: Criticality, Health, Confidence   │   │
│  └──────────────────────────────────────────────────────┘   │
│                          ↓                                    │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ 3. Analysis Engines                                  │   │
│  │    ├─ Blast Radius (BFS traversal)                   │   │
│  │    ├─ Criticality Scoring (weighted formula)         │   │
│  │    └─ Failure Simulation (cascade prediction)        │   │
│  └──────────────────────────────────────────────────────┘   │
│                          ↓                                    │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ 4. CLI & Visualization                               │   │
│  │    ├─ scan: discover dependencies                    │   │
│  │    ├─ analyze: blast radius analysis                 │   │
│  │    ├─ criticality: rank systems by risk              │   │
│  │    ├─ graph: interactive visualization               │   │
│  │    └─ simulate: failure cascade scenarios            │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

---

## Core Data Structures

### Graph Model

```rust
// Nodes represent systems/components
pub enum NodeType {
    Service,        // Microservice, API
    Database,       // PostgreSQL, MongoDB, etc
    Pipeline,       // Airflow DAG, Spark job
    Topic,          // Kafka topic
    VectorDb,       // Pinecone, Weaviate
    Dashboard,      // Analytics dashboard
    Agent,          // AI/LLM agent
    ExternalService,// Third-party API
    Table,          // Data table/warehouse
    Cache,          // Redis, Memcached
    Queue,          // RabbitMQ, SQS
}

// Edges represent relationships
pub enum EdgeType {
    Calls,          // Sync RPC (gRPC, HTTP)
    Produces,       // Async (Kafka, queue)
    Consumes,       // Consumes from topic
    Queries,        // Database query
    Feeds,          // Data flow
    DependsOn,      // Generic dependency
    Transforms,     // Data pipeline transformation
    ExportsTo,      // Reverse ETL
    Retrieves,      // Vector DB retrieval
}
```

### Graph Properties

**Node Properties**:
- `criticality` (0.0-1.0): Business importance
- `health_status`: Healthy, Degraded, Unhealthy, Unknown
- `recovery_time_minutes`: MTTR estimate
- `business_exposure` (0.0-1.0): Revenue/impact at risk
- `confidence` (0.0-1.0): Discovery confidence score
- `tags`: Custom metadata
- `discovered_via`: List of discovery methods

**Edge Properties**:
- `is_critical`: Can failures cascade?
- `is_async`: Async or sync?
- `has_fallback`: Fallback available?
- `has_retry`: Retry logic?
- `latency_ms`: Measured latency
- `error_rate`: Observed error rate
- `confidence` (0.0-1.0): Edge confidence
- `discovered_via`: Which discovery method found this

---

## Discovery Engines (Phase 1)

### Kubernetes Discovery
- Parses K8s Service manifests
- Extracts service-to-service calls
- Identifies ConfigMap/Secret references
- Reads NetworkPolicies

**Input**: Kubeconfig path, context, namespaces  
**Output**: Graph with Service nodes, Calls/DependsOn edges

### Terraform Discovery
- Parses AWS/Azure/GCP HCL
- Extracts resource relationships
- Identifies connection strings
- Maps infrastructure dependencies

**Input**: Terraform root directory, state files  
**Output**: Graph with resource nodes, depends_on edges

### Airflow Discovery
- Parses Python DAG files
- Extracts task dependencies
- Identifies external operators
- Maps DAG lineage

**Input**: DAG folder, Airflow home  
**Output**: Graph with PipelineTask nodes, depends_on edges

### OTLP Trace Consumer
- Connects to OTLP endpoint (gRPC/HTTP)
- Parses distributed traces
- Extracts service-to-service calls from spans
- Identifies database queries
- Detects HTTP dependencies

**Input**: OTLP endpoint URL, headers  
**Output**: Graph with discovered runtime dependencies

### Hybrid Merging
- Combines static + dynamic graphs
- Confidence scoring per edge (0.95 if both sources confirm)
- Identifies hidden dependencies (in traces but not config)
- Resolves conflicts

---

## Analysis Engines

### Blast Radius Analysis
**Algorithm**: Breadth-First Search (BFS)

1. Start from failed node
2. Traverse to all neighbors (depth 1)
3. Continue to depth 2, 3, ... until no more neighbors
4. Categorize by impact depth
5. Calculate blast radius score (0-100)

**Output**:
- Directly impacted: 1 hop away
- Indirectly impacted: 2 hops away
- Tertiary impacted: 3+ hops away
- Blast radius score
- Critical path count

### Criticality Scoring
**Formula**:
```
criticality = (downstream × 0.4) + (node_criticality × 0.3) 
            + (recovery_time × 0.2) + (business_exposure × 0.1)
```

**Features**:
- Ranks all systems by blast radius risk
- Identifies single points of failure
- Upstream/downstream dependency counting

### Failure Simulation
**Algorithm**: Cascade propagation

1. Start from failed node
2. Use BFS to traverse critical edges only
3. Follow only critical paths (is_critical=true)
4. Calculate recovery time (max of all nodes in cascade)
5. Estimate business impact

**Supports**:
- Outage, Degradation, HighLatency, HighErrorRate
- Depth-limited cascade (avoid infinite loops)

---

## Module Organization

```
src/
├── lib.rs                 # Library root
├── error.rs               # Error types
├── models/
│   └── mod.rs             # Configuration models
├── graph/
│   ├── mod.rs             # Graph exports
│   ├── node.rs            # Node definition
│   ├── edge.rs            # Edge definition
│   ├── graph.rs           # Core Graph struct
│   ├── traversal.rs       # BFS, DFS, TopoSort
│   └── serialization.rs   # Export to JSON/GraphML/Dot
├── discovery/
│   ├── mod.rs             # Discovery trait
│   ├── kubernetes.rs      # K8s parser
│   ├── terraform.rs       # Terraform parser
│   ├── airflow.rs         # Airflow parser
│   └── otlp.rs            # OTLP consumer
├── analysis/
│   ├── mod.rs             # Analysis exports
│   ├── blast_radius.rs    # Blast radius engine
│   ├── criticality.rs     # Criticality scorer
│   └── simulator.rs       # Failure simulator
├── utils/
│   └── mod.rs             # File I/O, formatting
└── bin/
    └── cli.rs             # CLI application

tests/
├── unit/                  # Unit tests per module
└── integration/           # End-to-end tests
```

---

## CLI Commands (v0.1 MVP)

| Command | Purpose | Example |
|---------|---------|---------|
| `scan` | Discover dependencies | `pyblastradius scan --kubernetes . --terraform ./infra` |
| `analyze` | Blast radius analysis | `pyblastradius analyze --graph graph.json --service api-server` |
| `criticality` | Rank systems by risk | `pyblastradius criticality --graph graph.json` |
| `graph` | Export visualization | `pyblastradius graph --input graph.json --format dot` |
| `simulate` | Failure cascade | `pyblastradius simulate --graph graph.json --service postgres` |
| `analyze-deployment` | Pre-deployment risk | `pyblastradius analyze-deployment --manifest k8s.yaml --graph graph.json` |
| `status` | System metrics | `pyblastradius status --graph graph.json` |

---

## Key Algorithms

### Graph Traversal Complexity

- **BFS**: O(V + E) - optimal for blast radius
- **DFS**: O(V + E) - cycle detection, topological sort
- **Topological Sort**: O(V + E) - pipeline execution order
- **Cycle Detection**: O(V + E) - identify circular dependencies

### Criticality Computation

```
Time: O(V × E)  - for each node, check neighbors + compute score
Space: O(V)     - score storage
```

### Simulation Algorithm

```
Time: O(V + E)  - similar to BFS with critical-edge filtering
Space: O(V)     - visited set + cascade path
```

---

## Performance Targets (MVP)

| Operation | Target | Status |
|-----------|--------|--------|
| Graph creation (1000 nodes) | <100ms | ✅ |
| Blast radius analysis | <100ms | ✅ |
| Criticality scoring (all nodes) | <500ms | ✅ |
| Failure simulation | <200ms | ✅ |
| Visualization export | <1000ms | ✅ |
| End-to-end (scan → analyze) | <5 min | ⏳ |

---

## Dependencies

### Core
- **petgraph**: Efficient graph operations
- **serde/serde_json**: Serialization
- **clap**: CLI parsing
- **chrono**: Timestamps

### Optional (features)
- **reqwest/hyper/tonic**: HTTP/gRPC networking
- **prost**: Protobuf for OTLP

### Testing
- **criterion**: Benchmarks
- **proptest**: Property-based testing

---

## Future Enhancements (v0.2+)

### v0.2 (Data + Business)
- Extended discovery: dbt, Spark, Kafka, data warehouses
- Business impact quantification (YAML config)
- Pre-deployment risk analysis
- Reverse ETL monitoring

### v1.0 (Prediction + Scale)
- Advanced hidden dependency detection
- ML-based anomaly detection + failure prediction
- Neo4j backend (100K+ nodes)
- GraphQL API
- Slack/PagerDuty integrations
- Enterprise features (SSO, RBAC, audit logs)

---

**Last Updated**: August 5, 2026  
**Architecture Version**: v0.1  
**Stability**: Experimental
