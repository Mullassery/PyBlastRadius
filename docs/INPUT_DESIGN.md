# PyBlastRadius v1.0 Input Method Design

## Vision

Users should be able to analyze blast radius in their systems through **multiple, progressive input methods** — from simple one-liners to sophisticated programmatic APIs.

---

## User Personas & Workflows

### 1. **DevOps Engineer** (Quick Check)
- **Goal**: "Show me what breaks if my database goes down"
- **Context**: Standing at terminal during incident
- **Input**: Local directory with infrastructure-as-code files
- **Method**: CLI one-liner
- **Time Budget**: < 1 minute

### 2. **SRE on-call** (Incident Response)
- **Goal**: Quantify blast radius and impact before making changes
- **Context**: Running exploratory analysis during incident
- **Input**: Live Kubernetes cluster + current traces
- **Method**: CLI with config file
- **Time Budget**: 2-5 minutes

### 3. **Platform Engineer** (CI/CD Integration)
- **Goal**: Evaluate blast radius of pending changes
- **Context**: Pull request pre-merge validation
- **Input**: Git repo + manifest changes
- **Method**: CI/CD pipeline integration
- **Time Budget**: 2-3 minutes (automated)

### 4. **Application Developer** (Local Testing)
- **Goal**: Understand how my service impacts others
- **Context**: Development workflow
- **Input**: Local Docker Compose / single service config
- **Method**: Python API or CLI
- **Time Budget**: 30 seconds - 5 minutes

### 5. **Enterprise Architecture Team** (Strategic Planning)
- **Goal**: Comprehensive dependency audit across org
- **Context**: Long-running analysis for planning/compliance
- **Input**: All infrastructure, all services, all traces
- **Method**: Programmatic API with callbacks
- **Time Budget**: 10-60 minutes

---

## Input Method Hierarchy

```
Level 1 (Simplest)
└─ CLI: Single Command
   └─ pyblastradius analyze kubernetes://

Level 2
├─ CLI: Config File
│  └─ blastradius.yml + blastradius analyze
│
└─ Python API: Quick Start
   └─ from pyblastradius import *

Level 3
├─ CI/CD Integration
│  └─ GitHub Actions / GitLab CI
│
└─ Python API: Advanced
   └─ Custom discovery, callbacks, filtering

Level 4 (Most Complex)
└─ Programmatic Orchestration
   └─ Multi-source discovery + streaming analysis
```

---

## Input Methods

### Method 1: CLI - Auto-Discovery (Simplest)

**Use Case**: "Analyze this directory NOW"

```bash
# Scan current directory for all discoverable resources
pyblastradius scan

# Scan specific directory
pyblastradius scan /path/to/infra

# Scan and analyze immediately
pyblastradius scan --analyze

# Scan with verbose output
pyblastradius scan -v
```

**Behavior**:
- Auto-discovers: Kubernetes manifests, Terraform files, docker-compose.yml, Airflow DAGs
- Outputs: JSON graph to stdout or file
- Exit code: 0 (success) / 1 (errors) / 2 (warnings)

---

### Method 2: CLI - Discovery Sources (Explicit)

**Use Case**: "I know exactly what I want to analyze"

```bash
# Kubernetes cluster
pyblastradius scan kubernetes://kubeconfig=$HOME/.kube/config

# Terraform state
pyblastradius scan terraform://path=./infra,backend=local

# Airflow instance
pyblastradius scan airflow://host=localhost,port=8080

# OpenTelemetry traces
pyblastradius scan otlp://endpoint=localhost:4317,duration=5m

# Multiple sources (combined graph)
pyblastradius scan \
  kubernetes://... \
  terraform://... \
  otlp://...

# Local directory
pyblastradius scan local://path=.,recursive=true
```

**Behavior**:
- Each source is a URI: `scheme://key=value,key=value`
- Sources are merged into unified graph
- Authentication via environment variables or inline params
- Timeout/retry configuration available

---

### Method 3: CLI - Analysis Workflows

**Use Case**: "Analyze and tell me the risks"

```bash
# Analyze from graph file
pyblastradius analyze graph.json \
  --service payment-service \
  --max-depth 3

# Criticality ranking
pyblastradius criticality graph.json \
  --limit 10 \
  --format table

# Detect single points of failure
pyblastradius critical-path graph.json

# Simulate failures
pyblastradius simulate graph.json \
  --target postgres \
  --scenario cascade

# Export visualization
pyblastradius visualize graph.json \
  --output graph.html \
  --format interactive
```

---

### Method 4: Config File (blastradius.yml)

**Use Case**: "Store analysis configuration in code"

```yaml
# blastradius.yml
version: 1

discovery:
  sources:
    - type: kubernetes
      kubeconfig: ~/.kube/config
      namespaces: [default, production]
    
    - type: terraform
      paths:
        - ./infrastructure
      backends: [local, remote]
    
    - type: otlp
      endpoint: localhost:4317
      duration: 5m
      sample-rate: 0.1
    
    - type: airflow
      host: localhost
      port: 8080
      dags-folder: ./dags

analysis:
  # Blast radius settings
  max-depth: 5
  include-transitive: true
  
  # Quality gates
  quality-checks:
    - dbt-manifest: ./dbt/manifest.json
    - statguardian-endpoint: https://statguardian.example.com
  
  # Output
  outputs:
    - type: json
      path: blast-radius.json
    
    - type: html
      path: blast-radius.html
      interactive: true
    
    - type: slack
      webhook-url: ${SLACK_WEBHOOK}
      on-critical: true

# Advanced: Custom discovery plugins
plugins:
  - name: custom-service-mesh
    type: python
    path: ./plugins/mesh_discoverer.py
    config:
      mesh-endpoint: localhost:5000

# Filtering
filters:
  # Only analyze production
  namespaces: [production]
  environments: [prod]
  
  # Exclude external services
  exclude-patterns:
    - "external-.*"
    - "vendor-.*"

# Alerts/actions
alerts:
  - condition: "blast_radius_score > 0.8"
    actions:
      - type: slack
        channel: "#incidents"
      - type: pagerduty
        severity: critical
```

**Behavior**:
```bash
# Use config file
pyblastradius analyze

# Override config with CLI
pyblastradius analyze --max-depth 10

# Validate config
pyblastradius config validate

# Show resolved config (with env vars filled)
pyblastradius config show
```

---

### Method 5: Python API - Quick Start

**Use Case**: "One-off analysis in a script"

```python
from pyblastradius import (
    KubernetesDiscoverer,
    BlastRadiusAnalyzer,
)

# Discover from Kubernetes
discoverer = KubernetesDiscoverer()
graph = discoverer.discover()

# Analyze blast radius
analyzer = BlastRadiusAnalyzer(graph)
result = analyzer.analyze("payment-service", max_depth=3)

# Access results
print(f"Blast radius score: {result.score:.1%}")
print(f"Directly impacted: {len(result.directly_impacted)} services")
print(f"Indirectly impacted: {len(result.indirectly_impacted)} services")
```

---

### Method 6: Python API - Advanced

**Use Case**: "Custom analysis with programmatic control"

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
k8s = KubernetesDiscoverer(kubeconfig="~/.kube/config")
tf = TerraformDiscoverer(path="./infra")
otl = OtelDiscoverer(endpoint="localhost:4317", duration_minutes=60)

# Discover from each source
k8s_graph = k8s.discover()
tf_graph = tf.discover()
otl_graph = otl.discover()

# Merge graphs
graph = Graph.merge([k8s_graph, tf_graph, otl_graph])

# Filter to production only
graph = graph.filter(lambda node: node.environment == "production")

# Run comprehensive analysis
blast = BlastRadiusAnalyzer(graph)
criticality = CriticalityScorer(graph)
anomaly = AnomalyDetector(graph)

# Get multi-faceted results
for service in graph.services():
    br_result = blast.analyze(service)
    crit_score = criticality.score(service)
    anomalies = anomaly.detect(service)
    
    print(f"{service}: BR={br_result.score:.1%}, Crit={crit_score:.1%}, Anomalies={len(anomalies)}")

# Export
graph.export_html("analysis.html")
graph.export_json("graph.json")
```

---

### Method 7: CI/CD Integration

**GitHub Actions**

```yaml
# .github/workflows/blast-radius.yml
name: Blast Radius Analysis

on: [pull_request]

jobs:
  analyze:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - uses: Mullassery/pyblastradius-action@v1
        with:
          discovery-sources: |
            kubernetes://kubeconfig=$KUBECONFIG
            terraform://path=./infrastructure
          max-depth: 5
          failure-on-score: 0.8
        env:
          KUBECONFIG: ${{ secrets.KUBECONFIG }}
      
      - name: Comment on PR
        if: always()
        uses: actions/github-script@v6
        with:
          script: |
            const fs = require('fs');
            const result = JSON.parse(fs.readFileSync('blast-radius.json'));
            github.rest.issues.createComment({
              issue_number: context.issue.number,
              owner: context.repo.owner,
              repo: context.repo.repo,
              body: `## Blast Radius Analysis\n\nScore: ${result.score.toFixed(1)}%\n...`
            });
```

**GitLab CI**

```yaml
# .gitlab-ci.yml
blast-radius:
  image: python:3.10
  stage: test
  before_script:
    - pip install pyblastradius
  script:
    - pyblastradius analyze
      --kubernetes=$KUBE_CONFIG
      --terraform=./infrastructure
      --output=blast-radius.json
  artifacts:
    reports:
      dependency_scanning: blast-radius.json
  allow_failure: true
```

---

### Method 8: REST API (v1.0+)

**Use Case**: "Embed analysis in web dashboards"

```bash
# Start server
pyblastradius server --port 8080

# Analyze
curl -X POST http://localhost:8080/api/v1/analyze \
  -H "Content-Type: application/json" \
  -d '{
    "sources": [
      {
        "type": "kubernetes",
        "kubeconfig": "/path/to/kubeconfig"
      }
    ],
    "analysis": {
      "target": "payment-service",
      "max_depth": 3
    }
  }'

# Response
{
  "id": "analysis-123",
  "status": "completed",
  "blast_radius": {
    "score": 0.82,
    "directly_impacted": 5,
    "indirectly_impacted": 12
  },
  "critical_paths": [
    {
      "path": ["database", "cache", "api"],
      "risk_score": 0.95
    }
  ]
}
```

---

### Method 9: Docker Compose (Local Development)

**Use Case**: "Analyze my local dev environment"

```bash
# Analyze docker-compose.yml in current directory
pyblastradius scan local://recursive=true

# With explicit compose file
pyblastradius scan compose://file=docker-compose.yml,project=myapp

# Output
# Discovered services from docker-compose:
#   - web (depends on: db, cache, api)
#   - db (depends on: nothing)
#   - cache (depends on: nothing)
#   - api (depends on: db)
# 
# Blast radius if 'db' fails: 3 services (75%)
```

---

### Method 10: GitHub Repository Analysis

**Use Case**: "Analyze a repo I just found"

```bash
# Analyze public repo
pyblastradius scan github://owner=kubernetes,repo=kubernetes

# Analyze with PR comparison
pyblastradius scan github://owner=myorg,repo=myapp,pr=123

# Analyze commit changes
pyblastradius scan github://owner=myorg,repo=myapp,commit=abc123def

# Local clone of repo
pyblastradius scan git://repo=./myapp,branch=main
```

---

## Input Validation & Error Handling

### Validation Levels

```python
from pyblastradius import ValidationLevel, BlastRadiusAnalyzer

# STRICT: Fail on any warning
analyzer = BlastRadiusAnalyzer(graph, validation=ValidationLevel.STRICT)

# WARN (default): Log warnings, continue
analyzer = BlastRadiusAnalyzer(graph, validation=ValidationLevel.WARN)

# PERMISSIVE: Ignore warnings
analyzer = BlastRadiusAnalyzer(graph, validation=ValidationLevel.PERMISSIVE)
```

### Error Reporting

```bash
$ pyblastradius analyze

Error: Failed to connect to Kubernetes API
  Location: kubernetes://kubeconfig=/home/user/.kube/config
  Error: connection refused (127.0.0.1:6443)
  
  Suggestions:
  1. Verify kubeconfig path: /home/user/.kube/config
  2. Check cluster is running: kubectl cluster-info
  3. Verify credentials: kubectl auth can-i get nodes
  
  Use --help for more information
```

---

## Progressive Complexity Path

### Day 1 (Getting Started)
```bash
# Install
pip install pyblastradius

# Analyze local directory
cd myproject
pyblastradius scan
```

### Day 2 (Real Infrastructure)
```bash
# Analyze production cluster
pyblastradius scan kubernetes://kubeconfig=$KUBECONFIG

# Export visualization
pyblastradius analyze graph.json --visualize
```

### Day 3 (Automation)
```bash
# Add config file
cat > blastradius.yml << EOF
discovery:
  sources:
    - type: kubernetes
    - type: terraform
EOF

# Integrate into CI/CD
# (commit blastradius.yml and set up GitHub Actions)
```

### Week 2+ (Advanced)
```python
# Use Python API for custom logic
from pyblastradius import *

# Custom discovery
class MyServiceDiscoverer:
    def discover(self): ...

# Custom analysis
graph = KubernetesDiscoverer().discover()
results = analyze_with_custom_logic(graph)
```

---

## Supported Input Sources

| Source | Type | Discovery | Real-time | Auth | Status |
|--------|------|-----------|-----------|------|--------|
| Kubernetes | kubernetes:// | ✅ Manifests | ✅ Yes | kubeconfig | v0.1 |
| Terraform | terraform:// | ✅ HCL | ❌ No | ENV | v0.1 |
| Airflow | airflow:// | ✅ DAGs | ✅ Yes | API key | v0.1 |
| OpenTelemetry | otlp:// | ✅ Traces | ✅ Yes | mTLS | v0.1 |
| Local filesystem | local:// | ✅ Files | ❌ No | none | v0.1 |
| Docker Compose | compose:// | ✅ YAML | ❌ No | none | v1.0 |
| Git repository | git:// | ✅ History | ❌ No | SSH/HTTPS | v1.0 |
| GitHub | github:// | ✅ API | ❌ No | Token | v1.0 |
| dbt | dbt:// | ✅ Manifest | ❌ No | none | v0.1 |
| Config Management | ansible:// | ✅ Playbooks | ❌ No | none | v1.0 |
| Service Mesh | mesh:// | ✅ APIs | ✅ Yes | TLS | v1.0 |

---

## Configuration Precedence

```
1. CLI arguments (highest priority)
   pyblastradius analyze --max-depth 10

2. Environment variables
   PYBLASTRADIUS_MAX_DEPTH=10

3. Config file (blastradius.yml)
   discovery:
     sources: ...

4. Defaults (lowest priority)
   max_depth: 3
```

---

## Error Recovery & Partial Analysis

**Behavior when a source fails:**

```yaml
# blastradius.yml
discovery:
  sources:
    - type: kubernetes
      required: true  # Fail if this fails
    
    - type: otlp
      required: false  # Continue if this fails (optional)
    
    - type: terraform
      required: false
```

```bash
$ pyblastradius analyze

⚠️  Warning: Failed to connect to OTLP endpoint
   Continuing with Kubernetes and Terraform sources only
   
✅ Discovery complete: 523 nodes from 2 sources (1 skipped)
```

---

## User Experience Improvements

### 1. Auto-Detection
```bash
$ cd myproject
$ pyblastradius scan

Detected resources:
  • Kubernetes: 127 manifests in k8s/
  • Terraform: 43 files in infrastructure/
  • Docker Compose: 1 file (docker-compose.yml)
  • Airflow: No DAGs found

Ready to analyze 3 sources. Run:
  pyblastradius analyze
```

### 2. Command Suggestions
```bash
$ pyblastradius analyze
error: no discovery sources specified

Suggestions:
  1. Auto-discover in current directory:
     pyblastradius scan
  
  2. Specify sources explicitly:
     pyblastradius analyze \
       --kubernetes=$HOME/.kube/config \
       --terraform=./infra
  
  3. Use a config file:
     cp .pyblastradius.example blastradius.yml
     pyblastradius analyze
```

### 3. Progress Reporting
```bash
$ pyblastradius scan --verbose

[1/4] Scanning Kubernetes manifests... ✅ 127 nodes
[2/4] Parsing Terraform files... ✅ 43 nodes
[3/4] Fetching OTLP traces... ⏳ (this may take a minute)
[4/4] Merging graphs... ✅ 170 total nodes

Blast radius analysis:
  Analyzing 45 services for critical paths...
  Found 12 single points of failure
  Estimated MTTR range: 15min - 2hrs
```

---

## Implementation Roadmap

### v0.1.0 (Current)
- ✅ Core graph engine
- ✅ Discovery: Kubernetes, Terraform, Airflow, OTLP
- ✅ Analysis: Blast radius, criticality, anomalies
- ✅ Basic CLI (scan, analyze commands)
- ✅ Python API (core classes)

### v0.2.0 (Q3 2026)
- CLI config file support (blastradius.yml)
- Multi-source merging
- GitHub Actions integration
- Docker Compose discovery
- Enhanced error messages

### v0.3.0 (Q4 2026)
- REST API
- CI/CD platform integrations (GitLab, Jenkins)
- Git repository analysis
- Web dashboard
- Real-time trace streaming

### v1.0.0 (Q1 2027)
- Plugin system for custom discovery
- GraphQL API
- Advanced filtering/querying
- Production SLA support

---

## Documentation Structure

```
docs/
├── INPUT_DESIGN.md (this file)
├── QUICK_START.md
│   ├── 5-minute tutorial
│   └── "Analyze my Kubernetes cluster"
├── CLI_REFERENCE.md
│   ├── All commands with examples
│   └── Configuration reference
├── PYTHON_API.md
│   ├── API reference
│   └── Advanced examples
├── DISCOVERY_SOURCES.md
│   ├── How each source works
│   ├── Authentication setup
│   └── Limitations
├── CI_CD_INTEGRATION.md
│   ├── GitHub Actions
│   ├── GitLab CI
│   └── Jenkins
└── EXAMPLES/
    ├── kubernetes.yml
    ├── terraform.yml
    ├── airflow.py
    └── python_api.py
```

---

## Success Metrics

A user should be able to:

✅ Analyze a local Kubernetes cluster in < 1 minute with no config
✅ Set up CI/CD integration in < 5 minutes with example
✅ Understand all supported input methods within 10 minutes of reading docs
✅ Write custom discovery plugin in < 30 minutes with example
✅ Migrate from one analysis tool to PyBlastRadius in < 1 hour

---

## Next Steps

1. **Implement v0.2**: Config file support + CLI improvements
2. **Create examples**: Working examples for each input method
3. **Write documentation**: QUICK_START, CLI_REFERENCE, etc.
4. **User testing**: Get feedback from DevOps/SRE users
5. **Iterate**: Based on feedback, adjust input methods
