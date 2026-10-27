# PyBlastRadius Quick Start Guide

Get started with PyBlastRadius in 5 minutes.

---

## Prerequisites

- **Rust 1.70+** ([Install](https://rustup.rs/))
- **Git** (to clone the repo)
- **Kubernetes** (optional, for K8s discovery)
- **Terraform** (optional, for infrastructure analysis)

---

## Installation

### From Source (Development)

```bash
git clone ../PyBlastRadius
cd PyBlastRadius

# Build the CLI
cargo build --release

# The binary is at: ./target/release/pyblastradius
./target/release/pyblastradius --version
```

### From PyPI (Coming Soon)

```bash
pip install pyblastradius
pyblastradius --version
```

---

## 5-Minute Tutorial

### 1. Discover Dependencies

Scan your Kubernetes cluster and Terraform configs:

```bash
pyblastradius scan \
  --kubernetes ~/.kube/config \
  --terraform ./infrastructure \
  --output graph.json
```

**Output**:
```
🔍 Scanning for dependencies...
  📦 Scanning Kubernetes manifests...
    ✅ Found 3 services
  🏗️  Scanning Terraform configs...
    ✅ Found infrastructure dependencies

📈 Discovery Results:
   Nodes: 42
   Edges: 87
   Avg Degree: 4.14
```

### 2. Analyze Blast Radius

What happens if your database goes down?

```bash
pyblastradius analyze \
  --graph graph.json \
  --service postgresql
```

**Output**:
```
📊 Analyzing blast radius...

🎯 Blast Radius Analysis for postgresql

   Directly Impacted: 12
   Indirectly Impacted: 8
   Blast Radius Score: 87.5/100
   Critical Paths: 5
```

### 3. Rank Systems by Criticality

Which systems are single points of failure?

```bash
pyblastradius criticality --graph graph.json
```

**Output**:
```
Criticality Ranking:

  Node                      Criticality  Downstream
  ─────────────────────────────────────────────────
  postgresql                87.5         42
  payment-processor         85.0         38
  redis-primary             72.3         22
  api-gateway               68.9         18
  auth-service              65.4         15
  kafka-broker-1            61.2         12
```

### 4. Generate Visualization

Create an interactive dependency graph:

```bash
pyblastradius graph \
  --input graph.json \
  --output graph.html \
  --format html
```

Open `graph.html` in your browser to explore the dependency graph interactively.

### 5. Simulate Failure Cascade

What if the auth service fails?

```bash
pyblastradius simulate \
  --graph graph.json \
  --service auth-service \
  --max-depth 3
```

**Output**:
```
⚡ Simulating failure cascade for auth-service

  Cascade Path: ["auth-service", "api-gateway", "checkout-service", "payment-processor"]
  Est. Recovery Time: 15 minutes
  Business Impact Score: 78.5/100
```

---

## Example Configurations

### Kubernetes Discovery

Create a sample `k8s-example.yaml`:

```yaml
apiVersion: v1
kind: Service
metadata:
  name: api-server
spec:
  selector:
    app: api
  ports:
  - port: 80
    targetPort: 8080
---
apiVersion: v1
kind: Service
metadata:
  name: auth-service
spec:
  selector:
    app: auth
  ports:
  - port: 80
    targetPort: 8080
```

Discover:
```bash
pyblastradius scan --kubernetes .
```

### Terraform Discovery

Create a sample `main.tf`:

```hcl
resource "aws_db_instance" "postgres" {
  identifier     = "my-database"
  engine         = "postgres"
  instance_class = "db.t3.micro"
}

resource "aws_ecs_service" "api" {
  name    = "api-service"
  cluster = aws_ecs_cluster.main.id

  depends_on = [aws_db_instance.postgres]
}
```

Discover:
```bash
pyblastradius scan --terraform .
```

### Airflow Discovery

Sample DAG at `dags/example.py`:

```python
from airflow import DAG
from airflow.operators.python import PythonOperator

with DAG("example_pipeline", catchup=False) as dag:
    task1 = PythonOperator(task_id="load_data", python_callable=load_data)
    task2 = PythonOperator(task_id="transform", python_callable=transform)
    task3 = PythonOperator(task_id="publish", python_callable=publish)

    task1 >> task2 >> task3
```

Discover:
```bash
pyblastradius scan --airflow ~/airflow
```

---

## CLI Reference

### Global Options

```bash
pyblastradius [OPTIONS] COMMAND

OPTIONS:
  -v, --verbose       Enable verbose logging
  -d, --debug         Enable debug logging
  -h, --help          Show help
  --version           Show version
```

### Commands

#### scan

Discover dependencies from infrastructure and application configs.

```bash
pyblastradius scan [OPTIONS]

OPTIONS:
  -k, --kubernetes <PATH>    Path to kubeconfig or K8s manifest folder
  -t, --terraform <PATH>     Path to Terraform root directory
  -a, --airflow <PATH>       Path to Airflow DAG folder
  -o, --output <FILE>        Output graph JSON file (default: graph.json)
```

#### analyze

Analyze blast radius for a specific service.

```bash
pyblastradius analyze [OPTIONS]

OPTIONS:
  -g, --graph <FILE>         Input graph JSON file
  -s, --service <NAME>       Service to analyze (required)
  -d, --depth <DEPTH>        Max impact depth (default: unlimited)
  -o, --output <FILE>        Output report file
```

#### criticality

Rank all systems by criticality.

```bash
pyblastradius criticality [OPTIONS]

OPTIONS:
  -g, --graph <FILE>         Input graph JSON file
  -t, --threshold <SCORE>    Only show nodes >= threshold (0.0-1.0)
  -o, --output <FILE>        Output report file
```

#### graph

Export graph visualization.

```bash
pyblastradius graph [OPTIONS]

OPTIONS:
  -i, --input <FILE>         Input graph JSON file
  -o, --output <FILE>        Output file (default: graph.html)
  -f, --format <FORMAT>      Export format: html, dot, graphml, json
```

#### simulate

Simulate failure cascade scenarios.

```bash
pyblastradius simulate [OPTIONS]

OPTIONS:
  -g, --graph <FILE>           Input graph JSON file
  -s, --service <NAME>         Service to simulate failure (required)
  -f, --failure-type <TYPE>    Failure type: outage, degradation, latency, errors
  -d, --max-depth <DEPTH>      Limit cascade depth
```

#### analyze-deployment

Analyze risk of a deployment.

```bash
pyblastradius analyze-deployment [OPTIONS]

OPTIONS:
  -m, --manifest <FILE>      K8s manifest or Terraform plan
  -g, --graph <FILE>         Current system graph
```

#### status

Show system metrics.

```bash
pyblastradius status [OPTIONS]

OPTIONS:
  -g, --graph <FILE>         Input graph JSON file
```

---

## Output Formats

### JSON Export

```bash
pyblastradius graph --input graph.json --format json > graph-export.json
```

**Structure**:
```json
{
  "nodes": [
    {
      "id": "uuid",
      "name": "api-server",
      "node_type": "Service",
      "criticality": 0.9,
      "health_status": "Healthy"
    }
  ],
  "edges": [
    {
      "source": "uuid1",
      "target": "uuid2",
      "edge_type": "Calls",
      "is_critical": true
    }
  ]
}
```

### Graphviz/Dot Export

```bash
pyblastradius graph --input graph.json --format dot > graph.dot
# Render with: dot -Tpng graph.dot -o graph.png
```

### GraphML Export

```bash
pyblastradius graph --input graph.json --format graphml > graph.graphml
```

---

## Common Workflows

### Incident Response

When an incident occurs:

```bash
# 1. Load your system graph
graph_file="systems/prod-graph.json"

# 2. Find blast radius for the failing component
pyblastradius analyze --graph $graph_file --service $FAILING_SERVICE

# 3. Identify critical paths
pyblastradius criticality --graph $graph_file

# 4. Visualize impact
pyblastradius graph --input $graph_file --output incident-impact.html

# 5. Check which services are most at risk
pyblastradius criticality --graph $graph_file --threshold 0.8
```

### Pre-Deployment Risk Check

Before deploying changes:

```bash
# Analyze deployment risk
pyblastradius analyze-deployment \
  --manifest k8s-deployment.yaml \
  --graph systems/prod-graph.json

# Check health of critical dependencies
pyblastradius criticality --graph systems/prod-graph.json | head -10

# Simulate what happens if new service fails
pyblastradius simulate --graph systems/prod-graph.json --service new-service
```

### System Health Audit

Regular system reviews:

```bash
# Update dependency graph
pyblastradius scan \
  --kubernetes ~/.kube/config \
  --terraform ./infrastructure \
  --output systems/prod-graph.json

# Find single points of failure
pyblastradius criticality --graph systems/prod-graph.json

# Generate audit report
pyblastradius status --graph systems/prod-graph.json
```

---

## Troubleshooting

### "Kubeconfig not found"

```bash
# Ensure KUBECONFIG is set
export KUBECONFIG=~/.kube/config
pyblastradius scan --kubernetes $KUBECONFIG
```

### "No dependencies discovered"

1. Check file paths exist
2. Verify file formats (valid YAML/HCL)
3. Enable verbose output:
   ```bash
   pyblastradius scan --kubernetes . --verbose
   ```

### Performance issues with large graphs

- Limit discovery scope:
  ```bash
  pyblastradius scan --kubernetes ns1,ns2,ns3
  ```
- Use depth limits:
  ```bash
  pyblastradius analyze --graph graph.json --service api --depth 3
  ```

---

## Next Steps

1. **Integrate with your infrastructure**: Add Kubernetes, Terraform, Airflow configs
2. **Set up continuous scanning**: Schedule `pyblastradius scan` to keep graph updated
3. **Create business impact configs**: Map technical failures to business outcomes (v0.2)
4. **Set up alerts**: Integrate with Slack/PagerDuty (v0.2+)
5. **Enable prediction**: Set up ML failure prediction (v1.0)

---

## Getting Help

- **Documentation**: [docs/](../)
- **Issues**: [GitHub Issues](../PyBlastRadius/issues)
- **Email**: mullassery@gmail.com

---

**Version**: v0.1.0 MVP  
**Last Updated**: August 5, 2026
