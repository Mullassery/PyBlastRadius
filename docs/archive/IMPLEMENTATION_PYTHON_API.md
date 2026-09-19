# PyBlastRadius Python API Implementation Plan

**Reference**: [INPUT_DESIGN.md](INPUT_DESIGN.md) — Python API covers methods 5-6, plus programmatic orchestration

---

## Python API Architecture

```python
pyblastradius/
├── __init__.py
├── graph.py              # Graph, Node, Edge
├── discovery/
│   ├── __init__.py
│   ├── base.py          # Discoverer base class
│   ├── kubernetes.py    # KubernetesDiscoverer
│   ├── terraform.py     # TerraformDiscoverer
│   ├── airflow.py       # AirflowDiscoverer
│   └── otlp.py          # OtelDiscoverer
├── analysis/
│   ├── __init__.py
│   ├── blast_radius.py  # BlastRadiusAnalyzer
│   ├── criticality.py   # CriticalityScorer
│   ├── simulator.py     # Simulator
│   └── anomaly.py       # AnomalyDetector
├── integrations/
│   ├── otel.py          # OtelCollectorClient
│   ├── statguardian.py  # StatGuardianClient
│   └── dbt.py           # DbtManifestParser
└── config.py            # Configuration management
```

---

## Level 1: Quick Start API

**Related to**: INPUT_DESIGN.md § Method 5: Python API - Quick Start

```python
from pyblastradius import KubernetesDiscoverer, BlastRadiusAnalyzer

# Discover
discoverer = KubernetesDiscoverer()
graph = discoverer.discover()

# Analyze
analyzer = BlastRadiusAnalyzer(graph)
result = analyzer.analyze("payment-service")

# Results
print(f"Blast radius: {result.score:.1%}")
```

### Implementation Checklist

- [ ] **Discoverer base class**:
  ```python
  from abc import ABC, abstractmethod
  
  class Discoverer(ABC):
      @abstractmethod
      def discover(self) -> Graph:
          """Discover dependencies and return graph"""
          pass
  ```

- [ ] **KubernetesDiscoverer**:
  - [ ] Auto-detect kubeconfig location
  - [ ] Load manifests from cluster
  - [ ] Parse services, deployments, ingresses
  - [ ] Extract dependencies from environment variables, config maps
  - [ ] Return Graph object

- [ ] **TerraformDiscoverer**:
  - [ ] Load terraform files from path
  - [ ] Parse and extract resources
  - [ ] Build dependency graph
  - [ ] Handle module references

- [ ] **AirflowDiscoverer**:
  - [ ] Connect to Airflow API (or parse DAGs)
  - [ ] Extract DAG structure
  - [ ] Extract operator dependencies
  - [ ] Build graph of task dependencies

- [ ] **OtelDiscoverer**:
  - [ ] Connect to OTLP collector
  - [ ] Fetch traces
  - [ ] Extract service calls from spans
  - [ ] Build graph from runtime dependencies

- [ ] **BlastRadiusAnalyzer**:
  - [ ] Accept graph + target service
  - [ ] Run BFS traversal
  - [ ] Categorize impact by depth
  - [ ] Return BlastRadiusResult with score, directly_impacted, indirectly_impacted

- [ ] **Exception handling**:
  - [ ] Clear error messages
  - [ ] Suggest fixes
  - [ ] Include documentation links

### Code Structure

```python
# pyblastradius/discovery/kubernetes.py
from pyblastradius.discovery.base import Discoverer
from pyblastradius.graph import Graph, Node, Edge, NodeType, EdgeType

class KubernetesDiscoverer(Discoverer):
    def __init__(self, kubeconfig: str = None, context: str = None):
        """
        Initialize Kubernetes discoverer
        
        Args:
            kubeconfig: Path to kubeconfig (default: ~/.kube/config)
            context: Kubernetes context (default: current context)
        """
        self.kubeconfig = kubeconfig or self._default_kubeconfig()
        self.context = context
        self.client = self._create_client()
    
    def discover(self) -> Graph:
        """Discover all Kubernetes resources and return graph"""
        graph = Graph()
        
        # Load services
        services = self.client.get_services()
        for svc in services:
            graph.add_node(Node(
                id=svc.name,
                name=svc.name,
                type=NodeType.Service,
                metadata={"namespace": svc.namespace}
            ))
        
        # Load deployments
        deployments = self.client.get_deployments()
        for dep in deployments:
            graph.add_node(Node(...))
            # Add edges to dependent services
        
        # Extract dependencies from env vars, config maps, etc.
        self._extract_dependencies(graph)
        
        return graph
    
    def _extract_dependencies(self, graph: Graph) -> None:
        """Extract service-to-service dependencies"""
        # Scan environment variables for service names
        # Parse ingress rules for dependencies
        # etc.
        pass
```

---

## Level 2: Advanced API

**Related to**: INPUT_DESIGN.md § Method 6: Python API - Advanced

```python
from pyblastradius import (
    Graph,
    KubernetesDiscoverer,
    TerraformDiscoverer,
    OtelDiscoverer,
    BlastRadiusAnalyzer,
    CriticalityScorer,
    AnomalyDetector,
)

# Multi-source discovery
k8s = KubernetesDiscoverer()
tf = TerraformDiscoverer(path="./infra")
otl = OtelDiscoverer(endpoint="localhost:4317")

k8s_graph = k8s.discover()
tf_graph = tf.discover()
otl_graph = otl.discover()

# Merge graphs
graph = Graph.merge([k8s_graph, tf_graph, otl_graph])

# Filter
graph = graph.filter(lambda n: n.environment == "production")

# Analyze
blast = BlastRadiusAnalyzer(graph)
crit = CriticalityScorer(graph)
anomaly = AnomalyDetector(graph)

for service in graph.services():
    br = blast.analyze(service)
    score = crit.score(service)
    anomalies = anomaly.detect(service)
    print(f"{service}: BR={br.score:.1%}, Crit={score:.1%}")
```

### Implementation Checklist

- [ ] **Graph.merge()** method:
  - [ ] Accept list of graphs
  - [ ] Deduplicate nodes (by ID)
  - [ ] Merge edges intelligently
  - [ ] Validate result

- [ ] **Graph.filter()** method:
  - [ ] Accept predicate function
  - [ ] Filter nodes
  - [ ] Remove orphaned edges
  - [ ] Return new filtered graph

- [ ] **Graph iteration methods**:
  - [ ] `graph.services()` → list of service nodes
  - [ ] `graph.databases()` → list of database nodes
  - [ ] `graph.nodes(type=NodeType.Service)` → filtered nodes
  - [ ] `graph.edges_from(node_id)` → outgoing edges
  - [ ] `graph.edges_to(node_id)` → incoming edges

- [ ] **CriticalityScorer**:
  - [ ] Weighted formula (downstream × 0.4 + criticality × 0.3 + recovery × 0.2 + exposure × 0.1)
  - [ ] Single point of failure detection
  - [ ] Ranking

- [ ] **AnomalyDetector**:
  - [ ] Detect 6 anomaly types (latency spikes, error rates, throughput, resources, traffic, health)
  - [ ] Time-to-failure estimation
  - [ ] Severity scoring

- [ ] **Simulation engine**:
  - [ ] Scenario definition (cascade, cascade-with-fallback, etc.)
  - [ ] Failure propagation
  - [ ] Recovery time estimation

### Code Structure

```python
# pyblastradius/graph.py
class Graph:
    def __init__(self):
        self.nodes: Dict[str, Node] = {}
        self.edges: List[Edge] = []
    
    @staticmethod
    def merge(graphs: List['Graph']) -> 'Graph':
        """Merge multiple graphs into one"""
        result = Graph()
        seen_nodes = set()
        
        for graph in graphs:
            # Add nodes (deduplicate)
            for node_id, node in graph.nodes.items():
                if node_id not in seen_nodes:
                    result.nodes[node_id] = node
                    seen_nodes.add(node_id)
            
            # Add edges
            for edge in graph.edges:
                if edge.source in seen_nodes and edge.target in seen_nodes:
                    result.edges.append(edge)
        
        return result
    
    def filter(self, predicate: Callable[[Node], bool]) -> 'Graph':
        """Filter graph by predicate"""
        result = Graph()
        
        # Filter nodes
        filtered_ids = set()
        for node_id, node in self.nodes.items():
            if predicate(node):
                result.nodes[node_id] = node
                filtered_ids.add(node_id)
        
        # Keep only edges between filtered nodes
        for edge in self.edges:
            if edge.source in filtered_ids and edge.target in filtered_ids:
                result.edges.append(edge)
        
        return result
    
    def services(self) -> List[Node]:
        """Get all service nodes"""
        return [n for n in self.nodes.values() if n.type == NodeType.Service]
```

---

## Level 3: Configuration Management

**Related to**: INPUT_DESIGN.md § Method 4: Config File (programmatic usage)

```python
from pyblastradius.config import Config

# Load from file
config = Config.load_from_file("blastradius.yml")

# Create programmatically
config = Config(
    discovery=[
        {"type": "kubernetes", "kubeconfig": "~/.kube/config"},
        {"type": "terraform", "path": "./infra"},
    ],
    analysis={"max_depth": 5},
)

# Use in discovery
discoverers = config.create_discoverers()
graphs = [d.discover() for d in discoverers]
merged = Graph.merge(graphs)
```

### Implementation Checklist

- [ ] **Config class**:
  - [ ] Load from YAML file
  - [ ] Create from dict
  - [ ] Validate schema
  - [ ] Resolve environment variables
  - [ ] Create discoverer instances

- [ ] **Type hints throughout**:
  ```python
  from typing import List, Dict, Optional
  
  class Config:
      discovery: List[Dict[str, Any]]
      analysis: Dict[str, Any]
      
      def create_discoverers(self) -> List[Discoverer]:
          """Create discoverer instances from config"""
          pass
  ```

### Code Structure

```python
# pyblastradius/config.py
from dataclasses import dataclass
from typing import List, Dict, Any
import os

@dataclass
class DiscoveryConfig:
    type: str
    params: Dict[str, Any]

class Config:
    def __init__(self, discovery: List[Dict] = None, analysis: Dict = None):
        self.discovery_configs = discovery or []
        self.analysis_config = analysis or {}
    
    @classmethod
    def load_from_file(cls, path: str) -> 'Config':
        import yaml
        with open(path, 'r') as f:
            data = yaml.safe_load(f)
        
        config = cls(
            discovery=data.get('discovery', {}).get('sources', []),
            analysis=data.get('analysis', {}),
        )
        config._resolve_env_vars()
        return config
    
    def _resolve_env_vars(self) -> None:
        """Replace ${VAR} with environment variables"""
        def replace_env(obj):
            if isinstance(obj, str):
                # Replace ${VAR_NAME} with os.environ['VAR_NAME']
                import re
                return re.sub(r'\$\{(\w+)\}', lambda m: os.environ.get(m.group(1), m.group(0)), obj)
            elif isinstance(obj, dict):
                return {k: replace_env(v) for k, v in obj.items()}
            elif isinstance(obj, list):
                return [replace_env(item) for item in obj]
            return obj
        
        self.discovery_configs = replace_env(self.discovery_configs)
        self.analysis_config = replace_env(self.analysis_config)
    
    def create_discoverers(self) -> List[Discoverer]:
        """Create discoverer instances from config"""
        discoverers = []
        
        for config in self.discovery_configs:
            disc_type = config['type']
            params = {k: v for k, v in config.items() if k != 'type'}
            
            if disc_type == 'kubernetes':
                discoverers.append(KubernetesDiscoverer(**params))
            elif disc_type == 'terraform':
                discoverers.append(TerraformDiscoverer(**params))
            # ... more types
        
        return discoverers
```

---

## Level 4: Callbacks & Streaming

**Related to**: INPUT_DESIGN.md § Method 6 (advanced) - for enterprise audits

```python
from pyblastradius import KubernetesDiscoverer, BlastRadiusAnalyzer

# Callback for each analyzed service
def on_service_analyzed(service_id: str, result):
    print(f"{service_id}: {result.score:.1%}")

# Stream results instead of waiting for all
discoverer = KubernetesDiscoverer()
graph = discoverer.discover()

analyzer = BlastRadiusAnalyzer(graph)
analyzer.analyze_all(callback=on_service_analyzed)
```

### Implementation Checklist

- [ ] **Callback support**:
  ```python
  class BlastRadiusAnalyzer:
      def analyze_all(self, callback: Callable = None):
          for service in self.graph.services():
              result = self.analyze(service)
              if callback:
                  callback(service.id, result)
  ```

- [ ] **Async/await support** (for long-running analyses):
  ```python
  async def discover_async() -> Graph:
      """Async discovery (useful for multiple sources)"""
      pass
  ```

- [ ] **Progress reporting**:
  ```python
  def analyze_all(self, callback=None, progress=None):
      total = len(self.graph.services())
      for i, service in enumerate(self.graph.services()):
          result = self.analyze(service)
          if callback:
              callback(service.id, result)
          if progress:
              progress(i + 1, total)
  ```

---

## Level 5: Integration Clients

**Related to**: INPUT_DESIGN.md - statguardian, dbt, OpenTelemetry

```python
from pyblastradius.integrations import (
    OtelCollectorClient,
    StatGuardianClient,
    DbtManifestParser,
)

# OpenTelemetry
otel = OtelCollectorClient("localhost:4317")
runtime_graph = otel.discover_dependencies(time_window_minutes=60)

# Data quality
sg = StatGuardianClient("https://statguardian.example.com", api_key="...")
quality_events = sg.get_quality_events()

# dbt lineage
dbt = DbtManifestParser("./dbt/manifest.json")
lineage = dbt.extract_column_lineage()
```

### Implementation Checklist

- [ ] **OtelCollectorClient**:
  - [ ] Connect to OTLP collector (gRPC and HTTP)
  - [ ] Query traces
  - [ ] Extract service-to-service calls
  - [ ] Build dependency graph

- [ ] **StatGuardianClient**:
  - [ ] Connect to StatGuardian API
  - [ ] Fetch quality rules
  - [ ] Track quality incidents
  - [ ] Cascade impact analysis

- [ ] **DbtManifestParser**:
  - [ ] Parse manifest.json
  - [ ] Extract model dependencies
  - [ ] Extract column lineage
  - [ ] Link to dashboards/BI tools

---

## Error Handling & Validation

### Exception Hierarchy

```python
class BlastRadiusException(Exception):
    """Base exception"""
    pass

class DiscoveryException(BlastRadiusException):
    """Discovery failed"""
    pass

class ConfigException(BlastRadiusException):
    """Config validation failed"""
    pass

class AnalysisException(BlastRadiusException):
    """Analysis failed"""
    pass
```

### Validation

```python
from dataclasses import dataclass
from typing import Optional

@dataclass
class BlastRadiusResult:
    score: float
    directly_impacted: list
    indirectly_impacted: list
    
    def __post_init__(self):
        assert 0 <= self.score <= 1, "Score must be 0-1"
        assert isinstance(self.directly_impacted, list)
```

---

## Testing Plan

### Unit Tests
```python
# tests/test_graph.py
def test_graph_merge():
    g1 = Graph()
    g2 = Graph()
    # ... add nodes and edges
    merged = Graph.merge([g1, g2])
    assert len(merged.nodes) == ...

# tests/test_analyzer.py
def test_blast_radius():
    graph = load_test_graph()
    analyzer = BlastRadiusAnalyzer(graph)
    result = analyzer.analyze("service-1")
    assert result.score > 0
```

### Integration Tests
```python
# tests/integration/test_kubernetes.py
def test_kubernetes_discovery():
    # Requires real K8s cluster or mock
    discoverer = KubernetesDiscoverer()
    graph = discoverer.discover()
    assert len(graph.nodes) > 0
```

---

## Documentation Map

| API | Doc | Example | Reference |
|-----|-----|---------|-----------|
| Quick API | PYTHON_API.md | `KubernetesDiscoverer().discover()` | INPUT_DESIGN § Method 5 |
| Advanced API | PYTHON_API.md | `Graph.merge()`, custom filtering | INPUT_DESIGN § Method 6 |
| Config | PYTHON_API.md | `Config.load_from_file()` | INPUT_DESIGN § Method 4 |
| Callbacks | PYTHON_API.md | `analyze_all(callback=...)` | INPUT_DESIGN § Method 6 |
| Integration | PYTHON_API.md | `OtelCollectorClient()` | INPUT_DESIGN |

---

## Version Progression

### v0.1 (Current)
- ✅ Core classes (Graph, Node, Edge)
- ✅ Discoverers (Kubernetes, Terraform, Airflow, OTLP)
- ✅ Basic analyzers (BlastRadius, Criticality)
- ✅ Integrations (StatGuardian, dbt, OTel)

### v0.2 (Target)
- [ ] Config class + loading
- [ ] Graph.merge() and Graph.filter()
- [ ] Full analysis suite (Simulator, AnomalyDetector)
- [ ] Exception hierarchy + validation

### v0.3
- [ ] Async/await support
- [ ] Callbacks + progress reporting
- [ ] Advanced filtering/querying
- [ ] Type hints throughout

### v1.0
- [ ] Plugin system for custom discoverers
- [ ] Dataclass serialization (JSON, Parquet)
- [ ] Performance optimizations

---

## Success Criteria

✅ Simple scripts can discover dependencies in 3 lines of code
✅ Complex analyses can be written with proper abstractions
✅ Config files can be loaded and used programmatically
✅ All exceptions have helpful messages
✅ Type hints cover all public APIs
✅ Documentation includes runnable examples
