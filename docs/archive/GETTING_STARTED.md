# PyBlastRadius Getting Started Guide

**Complete reference**: [INPUT_DESIGN.md](INPUT_DESIGN.md) (architecture) + [IMPLEMENTATION_CLI.md](IMPLEMENTATION_CLI.md) + [IMPLEMENTATION_PYTHON_API.md](IMPLEMENTATION_PYTHON_API.md)

---

## 5-Minute Quick Start

### Installation

```bash
pip install pyblastradius
```

### Analyze Your Kubernetes Cluster (30 seconds)

```bash
# Auto-discover and analyze
pyblastradius scan

# View results
cat blast-radius.json | jq '.blast_radius_score'
```

**Output**:
```
Discovered resources:
  • Kubernetes: 127 services
  • Terraform: 43 resources
  • Airflow: 0 DAGs

Graph merged: 170 nodes

Blast radius analysis available at: blast-radius.json
Run: pyblastradius analyze to get full report
```

### Run Full Analysis (1 minute)

```bash
# Analyze a specific service
pyblastradius analyze blast-radius.json \
  --service payment-service \
  --max-depth 3

# See visualizations
pyblastradius visualize blast-radius.json \
  --output blast-radius.html
open blast-radius.html
```

---

## What is PyBlastRadius?

PyBlastRadius predicts **cascading failures** in your infrastructure by analyzing:

- **Dependencies** (what calls what)
- **Failure impact** (if X fails, what else breaks?)
- **Criticality** (which services are most critical?)
- **Anomalies** (what's unusual about performance?)

**Time to insight**: 30 seconds for quick analysis, 5 minutes for comprehensive audit.

---

## 10 Ways to Analyze (Pick Your Method)

### Method 1: Auto-Discover Everything (Simplest)

```bash
pyblastradius scan
```

**When to use**: Quick incident response, local testing  
**Time**: 30 seconds  
**Works with**: Kubernetes, Terraform, Airflow, Docker Compose (all auto-detected)

---

### Method 2: Analyze Specific Kubernetes Cluster

```bash
pyblastradius scan kubernetes://kubeconfig=~/.kube/config,context=production
```

**When to use**: Target specific cluster  
**Time**: 1-2 minutes  
**Auth**: Uses kubeconfig (same as `kubectl`)

---

### Method 3: Analyze Terraform Infrastructure

```bash
pyblastradius scan terraform://path=./infrastructure,backend=local
```

**When to use**: Infrastructure-as-code analysis  
**Time**: 30 seconds  
**Supports**: Local, S3, Azure, GCS backends

---

### Method 4: Combine Multiple Sources

```bash
pyblastradius scan \
  kubernetes://kubeconfig=~/.kube/config \
  terraform://path=./infra \
  otlp://endpoint=localhost:4317,duration=5m
```

**When to use**: Full-stack analysis (infra + runtime)  
**Time**: 2-5 minutes  
**Result**: Unified graph showing all dependencies

---

### Method 5: Use Configuration File

**Create `blastradius.yml`**:
```yaml
version: 1
discovery:
  sources:
    - type: kubernetes
      kubeconfig: ~/.kube/config
    - type: terraform
      path: ./infrastructure
    - type: otlp
      endpoint: localhost:4317
      duration: 5m

analysis:
  max-depth: 5

outputs:
  - type: json
    path: blast-radius.json
  - type: html
    path: blast-radius.html
```

**Run**:
```bash
pyblastradius analyze
```

**When to use**: Production setups, CI/CD, version-controlled config  
**Time**: 2-5 minutes  
**Benefit**: Repeatable, shareable, auditable

---

### Method 6: Python Script (Simple)

```python
from pyblastradius import KubernetesDiscoverer, BlastRadiusAnalyzer

# Discover
discoverer = KubernetesDiscoverer()
graph = discoverer.discover()

# Analyze
analyzer = BlastRadiusAnalyzer(graph)
result = analyzer.analyze("payment-service")

# Print
print(f"Blast radius: {result.score:.1%}")
print(f"Directly impacted: {len(result.directly_impacted)} services")
```

**When to use**: Notebooks, scripts, automation  
**Time**: 1-2 minutes  
**Benefit**: Programmatic, integrates with other tools

---

### Method 7: Python Script (Advanced)

```python
from pyblastradius import (
    Graph,
    KubernetesDiscoverer,
    TerraformDiscoverer,
    BlastRadiusAnalyzer,
    CriticalityScorer,
)

# Multi-source
k8s = KubernetesDiscoverer().discover()
tf = TerraformDiscoverer(path="./infra").discover()
graph = Graph.merge([k8s, tf])

# Filter to production
graph = graph.filter(lambda n: n.environment == "production")

# Comprehensive analysis
blast = BlastRadiusAnalyzer(graph)
crit = CriticalityScorer(graph)

for service in graph.services():
    br = blast.analyze(service)
    score = crit.score(service)
    print(f"{service.name}: BR={br.score:.1%}, Crit={score:.1%}")
```

**When to use**: Custom analysis, integration with BI tools  
**Time**: 5-30 minutes  
**Benefit**: Full control, reusable code

---

### Method 8: GitHub Actions (CI/CD)

**Create `.github/workflows/blast-radius.yml`**:
```yaml
name: Blast Radius Check

on: [pull_request]

jobs:
  analyze:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - uses: Mullassery/pyblastradius-action@v1
        with:
          discovery-sources: |
            kubernetes://kubeconfig=${{ secrets.KUBECONFIG }}
            terraform://path=./infra
          max-depth: 5
          failure-on-score: 0.8
      
      - name: Comment PR
        if: always()
        uses: actions/github-script@v6
        with:
          script: |
            const fs = require('fs');
            const result = JSON.parse(
              fs.readFileSync('blast-radius.json')
            );
            github.rest.issues.createComment({
              issue_number: context.issue.number,
              owner: context.repo.owner,
              repo: context.repo.repo,
              body: `## Blast Radius Analysis\nScore: ${result.blast_radius_score.toFixed(1)}%`
            });
```

**When to use**: Pre-merge validation, compliance checks  
**Time**: 2-3 minutes (automated)  
**Benefit**: Prevents risky changes from merging

---

### Method 9: Local Docker Compose

```bash
pyblastradius scan compose://file=docker-compose.yml
```

**When to use**: Local development, testing  
**Time**: 10 seconds  
**Works with**: docker-compose.yml, services defined locally

---

### Method 10: GitHub Repository Analysis

```bash
# Public repository
pyblastradius scan github://owner=kubernetes,repo=kubernetes

# Private repository (with token)
pyblastradius scan github://owner=myorg,repo=myapp,token=$GITHUB_TOKEN

# Pull request comparison
pyblastradius scan github://owner=myorg,repo=myapp,pr=123
```

**When to use**: External repo analysis, benchmarking  
**Time**: 2-5 minutes  
**Benefit**: No local clone needed

---

## Choosing Your Method

```
┌─ Quick analysis?          → Method 1 (auto-discover)
├─ Specific source?         → Method 2-4 (explicit URIs)
├─ Repeatable/production?   → Method 5 (config file)
├─ Script/notebook?         → Method 6-7 (Python)
├─ Validate PRs?            → Method 8 (GitHub Actions)
├─ Local dev?               → Method 9 (Docker Compose)
└─ External repo?           → Method 10 (GitHub)
```

---

## Understanding Results

### Blast Radius Score

**Scale**: 0 (isolated) to 1 (affects everything)

- **0.0 - 0.2**: Low risk (few dependencies)
- **0.2 - 0.5**: Medium risk (moderate impact)
- **0.5 - 0.8**: High risk (widespread impact)
- **0.8 - 1.0**: Critical risk (cascades across system)

### Example Output

```bash
$ pyblastradius analyze graph.json --service payment-service

Service: payment-service
Blast Radius Score: 0.82 (HIGH RISK)

Directly Impacted (1 hop):
  ✗ api-gateway (critical dependency)
  ✗ payment-processor (critical)
  • notification-service (optional)

Indirectly Impacted (2 hops):
  • user-service
  • order-service
  • billing-service
  (9 services total)

Critical Paths (highest risk):
  payment-service → api-gateway → load-balancer
  payment-service → database → backup-system

Recovery Time Estimate: 45-120 minutes
Single Point of Failure: api-gateway
```

---

## Common Workflows

### Workflow 1: Incident Response

**Goal**: "What breaks if X fails?"

```bash
# 30 seconds
pyblastradius scan
pyblastradius analyze graph.json --service postgres

# Output: See which services are impacted
# Action: Prioritize incident response
```

---

### Workflow 2: Pre-Deployment Check

**Goal**: "Is it safe to deploy this change?"

```bash
# Add to CI/CD (GitHub Actions, GitLab CI, etc.)
# Automatically runs on every PR
# Fails if blast radius score > threshold

# Example GitHub Actions workflow (see Method 8)
```

---

### Workflow 3: Architecture Review

**Goal**: "Which services are single points of failure?"

```bash
pyblastradius scan kubernetes:// terraform://path=./infra
pyblastradius criticality graph.json --limit 20

# Output: Ranked list of risky services
# Action: Plan redundancy improvements
```

---

### Workflow 4: Performance Optimization

**Goal**: "Which service failures cause cascading slowdowns?"

```python
from pyblastradius import Graph, AnomalyDetector

graph = load_graph()
detector = AnomalyDetector(graph)

for service in graph.services():
    anomalies = detector.detect(service)
    if anomalies:
        print(f"{service}: {len(anomalies)} anomalies")
        for a in anomalies:
            print(f"  - {a.type}: {a.severity:.1%}")
```

---

## Next Steps

### Quick Learning Path

**Day 1: Basics** (30 minutes)
1. Run `pyblastradius scan` on your system
2. View results with `pyblastradius visualize`
3. Try Method 2-3 with specific sources

**Day 2: Automation** (1 hour)
1. Create `blastradius.yml` config file
2. Set up GitHub Actions workflow
3. Test on a pull request

**Week 1: Integration** (2-4 hours)
1. Write custom Python script
2. Integrate with alerting system
3. Set up dashboard

**Week 2+: Advanced** (ongoing)
1. Build custom discovery plugins
2. Connect to observability stack
3. Automate incident response

---

## Troubleshooting

### "Connection refused" (Kubernetes)

```bash
# Problem: Can't connect to cluster
Error: Connection refused at 127.0.0.1:6443

# Solutions:
1. Check cluster is running
   kubectl cluster-info

2. Verify kubeconfig path
   echo $KUBECONFIG

3. Try explicit kubeconfig
   pyblastradius scan kubernetes://kubeconfig=/path/to/config
```

### "No such file or directory" (Terraform)

```bash
# Problem: Can't find Terraform files
Error: No such file or directory: ./infrastructure

# Solutions:
1. Check path exists
   ls -la ./infrastructure

2. Use absolute path
   pyblastradius scan terraform://path=/full/path/to/infra

3. Recursive search
   pyblastradius scan local://path=.,recursive=true
```

### "Config validation failed"

```bash
# Check config file syntax
pyblastradius config validate

# Show resolved config (with env vars filled)
pyblastradius config show

# Enable verbose mode
pyblastradius scan --verbose
```

---

## Performance Tips

### For Large Deployments (1000+ services)

1. **Limit discovery scope**:
   ```yaml
   discovery:
     sources:
       - type: kubernetes
         namespaces: [production]  # Only production
   ```

2. **Use filters**:
   ```python
   graph = graph.filter(lambda n: n.environment == "production")
   ```

3. **Parallel analysis**:
   ```python
   analyzer.analyze_all(callback=process_result)
   ```

4. **Stream results instead of waiting**:
   ```python
   for result in analyzer.analyze_streaming():
       # Process immediately, don't load all results
   ```

### For Real-Time Updates

1. **Run on schedule**:
   ```bash
   # Cron job every hour
   0 * * * * /usr/local/bin/pyblastradius analyze
   ```

2. **Stream from OTLP**:
   ```python
   otl = OtelDiscoverer(endpoint="localhost:4317", continuous=True)
   for update in otl.discover_streaming():
       graph.update(update)
   ```

---

## Integration Examples

### With Slack Alerts

```python
from pyblastradius import BlastRadiusAnalyzer
import json
from slack_sdk import WebClient

client = WebClient(token=SLACK_TOKEN)
graph = load_graph()
analyzer = BlastRadiusAnalyzer(graph)

for service in graph.services():
    result = analyzer.analyze(service)
    if result.score > 0.8:  # High risk
        client.chat_postMessage(
            channel="#incidents",
            text=f"⚠️ {service.name} has high blast radius ({result.score:.1%})"
        )
```

### With PagerDuty

```python
from pyblastradius import CriticalityScorer
from pdpyras import APISession

session = APISession()  # Uses PD_API_TOKEN env var
graph = load_graph()
scorer = CriticalityScorer(graph)

for service in graph.services():
    score = scorer.score(service)
    if score > 0.9:  # Critical
        # Create incident
        incident = session.post(
            "/incidents",
            json={
                "incident": {
                    "title": f"Critical service: {service.name}",
                    "urgency": "high",
                }
            }
        )
```

### With Datadog Dashboard

See [INTEGRATION_DATADOG.md](./INTEGRATION_DATADOG.md) for complete example (REST API endpoint).

---

## Documentation Map

| Need | Document | Read Time |
|------|----------|-----------|
| Quick start | **This file** | 10 min |
| Architecture | [INPUT_DESIGN.md](INPUT_DESIGN.md) | 20 min |
| CLI reference | [IMPLEMENTATION_CLI.md](IMPLEMENTATION_CLI.md) | 15 min |
| Python API | [IMPLEMENTATION_PYTHON_API.md](IMPLEMENTATION_PYTHON_API.md) | 15 min |
| Discovery sources | [DISCOVERY_SOURCES.md](./DISCOVERY_SOURCES.md) (TBD) | 20 min |
| CI/CD setup | [CI_CD_INTEGRATION.md](./CI_CD_INTEGRATION.md) (TBD) | 15 min |

---

## Support & Feedback

- **Issues**: [GitHub Issues](../PyBlastRadius/issues)
- **Discussions**: [GitHub Discussions](../PyBlastRadius/discussions)
- **Email**: mullassery@gmail.com

---

## Next: Build It

Ready to analyze your infrastructure? Start with Method 1:

```bash
pip install pyblastradius
cd your-project
pyblastradius scan
```

That's it! Open `blast-radius.html` in your browser.
