# PyBlastRadius

![PyPI](https://img.shields.io/pypi/v/pyblastradius)
[![CI](https://github.com/Mullassery/PyBlastRadius/actions/workflows/ci.yml/badge.svg)](https://github.com/Mullassery/PyBlastRadius/actions/workflows/ci.yml)
![License](https://img.shields.io/badge/license-Proprietary-blue)
![Python](https://img.shields.io/badge/python-3.10+-blue)
![Rust](https://img.shields.io/badge/rust-1.70+-orange)

**Operational Blast Radius Intelligence Platform** — Predict cascading failures and quantify business impact across infrastructure, data pipelines, and AI systems.

## Features

Working today, backed by the real Rust engine (`pyblastradius._core`):

- **Blast Radius Analysis** — BFS-based impact categorization (direct, indirect, tertiary), via `BlastRadiusAnalyzer`
- **Criticality Scoring** — Risk-weighted ranking using downstream dependencies, recovery time, business exposure, via `CriticalityScorer`
- **Cascade Simulation** — Simulate outage/degradation/high-latency/high-error-rate/data-quality failures and estimate recovery time and business impact, via `Simulator`
- **Graph import/export** — JSON round-trip and Graphviz DOT export
- **REST API server** — FastAPI server with real bearer-token auth and an explicit CORS allowlist (`pyblastradius server`)
- **CLI** — `analyze`, `criticality`, `simulate`, `visualize` commands operate on a graph you build or load from JSON
- **Terraform discovery** — `TerraformDiscoverer` (used by `pyblastradius scan terraform://path=...`, `type: terraform` in `blastradius.yml`, and directly as `pyblastradius.TerraformDiscoverer`) parses real `.tf` files via `hcl-rs`, discovers any `resource "type" "name" { ... }` block (not limited to a handful of hardcoded AWS types), and derives real dependency edges from actual resource references found in each resource's own attribute values (`aws_db_instance.x.endpoint` bare traversals, and `"${aws_db_instance.x.endpoint}"` interpolated strings) — not a hardcoded example dependency list. Benchmarked in `benches/large_graph.rs` on graphs up to 1,000 nodes.

Not yet implemented (present as scaffolding/placeholders, listed here so you don't spend time debugging them):

- **Dependency discovery (Kubernetes/Airflow/Otel)** — `KubernetesDiscoverer`, `AirflowDiscoverer`, `OtelDiscoverer` (used by `pyblastradius scan`) currently always return an empty graph. There is no YAML parsing or live cluster/API connection behind them yet. (Terraform discovery is real -- see Features above.)
- **Hidden Dependency Detection / Anomaly Detection** — `HiddenDependencyDetector` and `AnomalyDetector` always return empty results; they are not wired to any metrics/trace source.
- **`pyblastradius.integrations`** — `OtelCollectorClient` and `StatGuardianClient` are placeholders that return empty/fake data; `DbtManifestParser` genuinely parses a dbt `manifest.json` you already have on disk and is real.
- **GraphQL API** — not exposed anywhere; the Rust crate has a query-string builder (`src/api/graphql.rs`) that nothing serves or executes.
- **Chat and incident-management integrations** — the Rust structs (`src/api/integrations.rs`) build correct webhook payloads for chat notifications and incident-management alerting, but never send them (the HTTP calls are commented out), and are not exposed to Python at all yet.

## Quick Start

### Installation

```bash
# Basic installation
pip install pyblastradius

# With all dependencies
pip install pyblastradius[all]

# For CLI features only
pip install pyblastradius[cli]

# For REST API server
pip install pyblastradius[server]
```

### CLI Usage

`scan`'s `terraform://` source is real (see Features above) and will
discover actual resources/dependencies from `.tf` files at the given path.
`kubernetes://`/`airflow://`/`otel://` sources still always produce an
empty graph — no YAML parsing or live cluster/API connection behind them
yet. To analyze something real today with those, build a graph via the
Python API below (or hand-write one and export it with `graph.to_json()`)
and feed that JSON file into `analyze`/`criticality`/`simulate`.

```bash
# terraform:// is real; kubernetes:// still always produces an empty graph
# for its part of the discovered graph (see Features)
pyblastradius scan \
  kubernetes://kubeconfig=~/.kube/config \
  terraform://path=./infrastructure

# Analyze blast radius for a service (graph.json from Graph.to_json())
pyblastradius analyze graph.json --service payment-service

# Rank systems by criticality
pyblastradius criticality graph.json --limit 10

# Simulate failure cascade
pyblastradius simulate graph.json --target postgres

# Generate visualization
pyblastradius visualize graph.json --output blast-radius.html

# Start REST API server
pyblastradius server --host 0.0.0.0 --port 8080
```

### Configuration File

Create `blastradius.yml` in your project:

```yaml
version: 1

discovery:
  sources:
    - type: kubernetes
      kubeconfig: ~/.kube/config
    - type: terraform
      path: ./infrastructure

analysis:
  max-depth: 5

outputs:
  - type: json
    path: blast-radius.json
  - type: html
    path: blast-radius.html
```

Then run:
```bash
pyblastradius analyze
```

### Python API

This example is real and runnable end-to-end against the Rust engine —
build a small graph by hand, then run blast-radius, criticality, and
cascade-simulation analysis on it:

```python
from pyblastradius import (
    Graph, Node, Edge, NodeType, EdgeType, FailureType,
    BlastRadiusAnalyzer, CriticalityScorer, Simulator,
)

graph = Graph()
api = graph.add_node(Node("api-server", NodeType.Service))
auth = graph.add_node(Node("auth-service", NodeType.Service))
db = graph.add_node(Node("postgresql", NodeType.Database))

graph.add_edge(Edge(EdgeType.Calls, api, auth))
graph.add_edge(Edge(EdgeType.Calls, auth, db))

# Blast radius: what does api-server's own call graph reach?
result = BlastRadiusAnalyzer.analyze(graph, api)
print(f"Blast radius score: {result.blast_radius_score:.2f}")
print(f"Directly impacted: {len(result.directly_impacted)} services")
print(f"Indirectly impacted: {len(result.indirectly_impacted)} services")

# Criticality ranking across the whole graph
for score in CriticalityScorer.score_all(graph):
    print(score.node_name, score.criticality_score, score.is_single_point_of_failure)

# Simulate a database outage
sim = Simulator.simulate(graph, db, FailureType.Outage)
print(f"Estimated recovery: {sim.estimated_recovery_time_minutes} min, "
      f"business impact: {sim.business_impact_score:.1f}")

# Persist/reload as JSON (this is what the CLI's `analyze`/`criticality`/
# `simulate` commands read via `Graph.from_json(...)`)
with open("graph.json", "w") as f:
    f.write(graph.to_json())
```

## Architecture

PyBlastRadius orchestrates an open-source observability ecosystem:

```text
┌─────────────────────────────────────────────────┐
│         PyBlastRadius Orchestration Layer        │
├─────────────────────────────────────────────────┤
│                                                   │
│  OpenTelemetry (Traces)  → Runtime Dependencies  │
│  dbt (Manifest)         → Column Lineage         │
│  StatGuardian (Quality) → Data Quality Cascade   │
│                                                   │
│  Discovery Engines:                              │
│  • Kubernetes manifests  • Terraform HCL         │
│  • Airflow DAGs         • OTLP traces            │
│                                                   │
│  Analysis Engines:                               │
│  • Blast radius (BFS)   • Criticality scoring    │
│  • Anomaly detection    • Hidden dependencies    │
│  • Cascade prediction   • Time-to-failure        │
│                                                   │
└─────────────────────────────────────────────────┘
```

This is the intended end-state architecture. As of this release, the
**Analysis Engines** box (blast radius, criticality, cascade simulation)
is real and Rust-backed; the **Discovery Engines** and OpenTelemetry/dbt/
StatGuardian integration boxes are scaffolding that returns empty/fake
data today (see Features above for the exact status of each piece).

## Documentation

- **[Quick Start](docs/QUICK_START.md)** — 5-minute tutorial with examples
- **[Architecture](docs/ARCHITECTURE.md)** — System design, algorithms, performance
- **[Contributing](CONTRIBUTING.md)** — Development setup and guidelines

## Why PyBlastRadius?

### Problem
- SRE teams often spend hours manually tracing dependencies during an incident
- Existing tools answer "What broke?" but not "What will break next?"
- Business impact is guesswork, not data-driven
- Data quality issues cascade invisibly through pipelines

### Solution
PyBlastRadius provides:
- **Unified graph** across infrastructure, data, and applications
- **Column-level lineage** (raw data → dashboards → business metrics)
- **Cascade prediction** with quantified business impact
- **Anomaly detection** with time-to-failure estimates
- **Open-source first** — works with any observability backend

### Value (aspirational — not yet measured on real deployments)
- **Faster MTTR** through automated dependency discovery, once discovery is implemented
- **Illustrative cost-of-incident math** (e.g. $705K+ per prevented incident, based on an
  assumed $15K/min of downtime) to help build a business case — not a measured result
- **Lower cost than commercial observability platforms** by building on the open-source
  OpenTelemetry ecosystem instead of a proprietary SaaS backend
- **Zero vendor lock-in** with an open-source stack

These are the goals the project is designed around, not measured results —
Terraform is the only discovery engine wired up to real parsing so far; the
"automated dependency discovery" this depends on more broadly (Kubernetes,
Airflow, OpenTelemetry) still isn't. Treat this section as intent, not a
full claim about the current release.

## Platform Support

Discovery source integration status (i.e. what `pyblastradius scan` /
`KubernetesDiscoverer` etc. actually do today):

| Discovery Source | Status | Notes |
|-----------------|--------|----------|
| Kubernetes | Not implemented | `discover()` always returns an empty graph; no manifest parsing or API calls |
| Terraform | **Real** | `discover()` parses actual `.tf` files via `hcl-rs` and derives real dependency edges from resource references (see Features above) |
| Airflow | Not implemented | `discover()` always returns an empty graph |
| OpenTelemetry | Not implemented | `discover()` always returns an empty graph (there is a separate, real Rust OTLP client used by `examples/cli.rs`, but it is not exposed to the Python package) |
| dbt | Partial | `DbtManifestParser` genuinely parses a local `manifest.json` for column lineage |
| StatGuardian | Not implemented | `StatGuardianClient` returns empty/hardcoded fake data |

What **is** real and working: build a `Graph` directly via the Python API
(`Graph`, `Node`, `Edge`) or load one from JSON, then run
`BlastRadiusAnalyzer`, `CriticalityScorer`, and `Simulator` on it — that
whole path is backed by real, tested Rust code. See the Python API example
above.

## API Examples

### GraphQL

Not implemented. The Rust crate contains a query-string builder
(`src/api/graphql.rs`) for the intended future schema, but nothing serves
or executes GraphQL queries today — there is no GraphQL endpoint to call.

### Python SDK

```python
from pyblastradius.integrations import DbtManifestParser

# Parse dbt lineage (the one integration in this module that's real)
dbt = DbtManifestParser("./dbt/manifest.json")
lineage = dbt.extract_column_lineage()
```

`OtelCollectorClient` and `StatGuardianClient` also live in
`pyblastradius.integrations` but are placeholders — `discover_dependencies()`
always returns an empty graph and `get_quality_rules()`/
`track_quality_incident()` return empty/hardcoded data, not real results
from a live collector or StatGuardian instance.

## Integrations

**Not usable from Python today.** The Rust crate has integration structs
(`src/api/integrations.rs`) that build correct webhook payloads for chat
notifications and incident-management alerting, but:

- neither is exposed to the Python package — the equivalent Python import
  raises `ModuleNotFoundError`, there is no `pyblastradius.api` module
- even on the Rust side, the actual HTTP calls are commented out; every
  method returns `Ok(())` (or a hardcoded fake incident id) without sending
  anything

This is tracked as a known gap, not a working feature. If you need alerting
today, call your chat or incident-management provider's API directly with
the analysis results from `BlastRadiusAnalyzer`/`Simulator`.

## Performance

Numbers below are targets, not verified end-to-end (Python-visible)
measurements — treat them as design goals. `cargo bench` benchmark
scaffolding now exists (`benches/large_graph.rs`; run with `cargo bench`)
and exercises `BlastRadiusAnalyzer`/`CriticalityScorer`/graph construction
on layered graphs up to 1,000 nodes at the Rust level — a spot-check run
put blast-radius analysis at ~10µs and criticality scoring at ~85µs on a
1,000-node graph, comfortably under the targets below, though that's a
core-computation-only number (no pyo3 call overhead, no formal published
run) rather than a rigorous end-to-end benchmark.

| Operation | Target |
|-----------|--------|
| Dependency discovery (1000 nodes) | <2 min |
| Blast radius analysis | <100ms |
| Criticality scoring (all nodes) | <500ms |
| Graph visualization export | <1s |
| End-to-end scan → analyze → alert | <5 min |

## Project Status

| Phase | Release | Status | Delivered |
|-------|---------|--------|-----------|
| Phase 0 | Research & Planning | Complete | Architecture, personas, roadmap |
| Phase 1 | Foundation (Graph + Analysis) | Complete | Core graph engine, algorithms |
| Phase 2 | CLI Implementation | Complete | Full CLI with all commands |
| Phase 3 | Config & YAML | Complete | blastradius.yml support |
| Phase 4 | CI/CD Integration | Complete | GitHub Actions workflow |
| Phase 5 | Examples & Documentation | Complete | Working examples, configs |
| Phase 6 | REST API Server | Complete | FastAPI endpoints + OpenAPI, real API-key auth, explicit CORS allowlist |
| v0.2.0 | Working Rust-Python bindings | Shipped | `pyblastradius._core` extension module, real end-to-end analysis, first-ever working `import pyblastradius`; discovery engines still not implemented (see Platform Support) |
| v1.0.0 | Enterprise Edition | Planned | Q1 2027 |

## Known Issues

No issues are currently open in the GitHub tracker. The gaps below are the ones found by
reading the source directly; the Features, Platform Support, API Examples, and Integrations
sections above give the full detail on each:

- Dependency discovery (`pyblastradius scan` and the `*Discoverer` classes) always returns an
  empty graph for Kubernetes/Airflow/OTLP sources — no parsing or live connection is implemented
  yet for those. Terraform discovery is real (see Features/Platform Support above).
- `HiddenDependencyDetector` is a mix: `detect_from_trace_gaps` (fan-out + async-keyword
  heuristic) and `detect_semantic_coupling` (name-similarity heuristic) are real logic, not
  stubs — but two of its four methods, `detect_from_query_logs` and `detect_from_config`, are
  explicit MVP stubs that always return empty (see their own doc comments). `AnomalyDetector`
  always returns empty results.
- `OtelCollectorClient` and `StatGuardianClient` return empty/hardcoded placeholder data, not
  live results.
- The GraphQL query-string builder in the Rust crate is not served anywhere; there is no
  GraphQL endpoint.
- The chat/incident-management integration structs in the Rust crate build correct payloads
  but never send them, and are not exposed to Python.

`CriticalityScorer`'s two previously-documented correctness bugs are now fixed:
`criticality_score` no longer divides by 100 twice (it returns the documented 0–100 scale),
and `is_single_point_of_failure` now checks the target node's own in-degree (exactly one
predecessor means no redundant path in, i.e. a SPOF) instead of incorrectly checking whether
that predecessor also has other outgoing edges — branching topologies are now detected
correctly. Both fixes are covered by dedicated branching-topology regression tests in
`src/analysis/criticality.rs` and `tests/test_analysis.py`.

As of this writing, GitHub Actions CI (`ci.yml`) is still red on `main`, but only because of an
unrelated, pre-existing failure in Rust-internal code that isn't exposed to Python or claimed
anywhere in this README: `HiddenDependencyDetector::detect` and the data-quality
column-lineage impact analysis (`src/integrations/data_quality.rs`) both return empty results
where their own unit tests expect non-empty ones — the same underlying gap as the
`HiddenDependencyDetector` bullet above. Check the
[Actions tab](https://github.com/Mullassery/PyBlastRadius/actions) for current status rather
than trusting the badge at the top of this file in isolation.

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for:
- Development setup
- Code style guidelines
- Testing requirements
- Commit message format
- Pull request process

## Code of Conduct

This project adheres to the Contributor Covenant. By participating, you are expected to uphold this code. Please report unacceptable behavior.

## License

This project is licensed under a Proprietary License. See [LICENSE](LICENSE) for details.

**Attribution Required**: Any use of this software must include attribution: "Powered by PyBlastRadius (https://github.com/Mullassery/PyBlastRadius)"

## Getting Help

- **Documentation**: [docs/](docs/)
- **Quick Start**: [docs/QUICK_START.md](docs/QUICK_START.md)
- **Architecture**: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- **Issues**: [GitHub Issues](https://github.com/Mullassery/PyBlastRadius/issues)
- **Email**: mullassery@gmail.com

## Citation

If you use PyBlastRadius in your research or production systems, please cite:

```bibtex
@software{pyblastradius2026,
  title={PyBlastRadius: Operational Blast Radius Intelligence Platform},
  author={Mullassery, Georgi},
  year={2026},
  url={https://github.com/Mullassery/PyBlastRadius}
}
```

## Roadmap

### Near-term (Q4 2026)
- [ ] v0.1 public MVP release
- [ ] Beta customer feedback program
- [ ] Kubernetes Operator for continuous discovery
- [ ] Helm chart for self-hosted deployment

### Medium-term (Q1 2027)
- [ ] v1.0 enterprise release
- [ ] ML-based failure prediction
- [ ] Neo4j backend for 100K+ node graphs
- [ ] GraphQL Federation for multi-service deployments

### Long-term (2027+)
- [ ] Multi-region cascade analysis
- [ ] Autonomous remediation recommendations
- [ ] Industry-specific impact templates
- [ ] GDPR/data-residency support

## Related Projects

Part of the Mullassery observability and data quality ecosystem:

- **[StatGuardian](https://github.com/Mullassery/StatGuardian)** — Data quality monitoring and lineage
- **[PyStreamMCP](https://github.com/Mullassery/PyStreamMCP)** — Streaming data intelligence
- **[PyDependencyCheck](https://github.com/Mullassery/PyDependencyCheck)** — Dependency scanning

## Authors

**Georgi Mammen Mullassery**
- GitHub: [@Mullassery](https://github.com/Mullassery)
- Email: mullassery@gmail.com

## Acknowledgments

Built on proven open-source foundations:
- [petgraph](https://github.com/petgraph/petgraph) — Graph algorithms
- [serde](https://serde.rs/) — Serialization
- [tokio](https://tokio.rs/) — Async runtime
- [OpenTelemetry](https://opentelemetry.io/) — Distributed tracing
- [dbt](https://www.getdbt.com/) — Data lineage standard

---

**Made by Georgi Mammen Mullassery**

[GitHub](https://github.com/Mullassery/PyBlastRadius) · [Documentation](docs/) · [Issues](https://github.com/Mullassery/PyBlastRadius/issues)
