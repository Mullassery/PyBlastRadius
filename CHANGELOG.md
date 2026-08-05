# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned
- ML-based failure prediction with anomaly consensus scoring
- Neo4j backend for 100K+ node graphs
- GraphQL Federation for multi-service deployments
- Kubernetes Operator for continuous discovery
- Helm chart for self-hosted deployment
- Advanced hidden dependency detection (query logs, configuration mining)

## [0.1.0] - 2026-10-15 (Planned)

### Added
- **Core Graph Engine**
  - Unified dependency graph with petgraph
  - Support for 11 node types (Service, Database, Pipeline, Topic, VectorDB, Agent, Cache, Queue, etc.)
  - 9 edge types (Calls, Queries, Produces, Consumes, Feeds, DependsOn, Transforms, ExportsTo, Retrieves)
  - Graph traversal algorithms (BFS, DFS, topological sort)
  - Graph serialization (JSON, GraphML, Dot/Graphviz)

- **Discovery Engines**
  - Kubernetes manifest parser (Services, Deployments, StatefulSets, NetworkPolicies)
  - Terraform HCL parser (AWS, Azure, GCP resources)
  - Airflow DAG parser (models, tasks, dependencies)
  - OpenTelemetry trace consumer (runtime dependencies from traces)

- **Analysis Algorithms**
  - Blast radius analysis (BFS-based impact categorization)
  - Criticality scoring (weighted formula with business exposure)
  - Failure simulator (cascade propagation, recovery time estimation)
  - Hidden dependency detection (trace mining, semantic analysis)
  - Anomaly detection (latency spikes, error rates, resource exhaustion)

- **CLI Interface**
  - `scan` - Discover dependencies from infrastructure
  - `analyze` - Blast radius analysis for a service
  - `criticality` - Rank systems by risk
  - `graph` - Generate visualization (HTML, Dot, GraphML)
  - `simulate` - Failure cascade scenarios
  - `analyze-deployment` - Pre-deployment risk assessment
  - `status` - System metrics

- **GraphQL API**
  - 15+ types with full schema
  - Queries: blastRadius, criticality, anomalies, hiddenDependencies, metrics
  - Mutations: updateNode, analyzeService, simulateFailure, addDependency

- **Third-Party Integrations**
  - Slack Bot (blast radius alerts, anomaly notifications)
  - PagerDuty (incident creation/resolution)

- **Data Quality & Lineage**
  - StatGuardian integration (quality rules, cascade impact)
  - dbt manifest parser (column-level lineage, tests)
  - Column lineage tracker (data quality cascade analysis)

- **Documentation**
  - Quick start guide (5-minute tutorial)
  - Architecture guide (algorithms, performance)
  - Contributing guide (dev setup, CI/CD)
  - Example configurations (Kubernetes, Terraform, Airflow)
  - Business impact templates

- **Testing & Quality**
  - 150+ unit and integration tests
  - CI/CD pipeline (GitHub Actions)
  - Code coverage reporting
  - Multi-platform builds (Linux, macOS, Windows)

### Technical Details
- Written in Rust with Python bindings (PyO3)
- Zero-copy graph operations via petgraph
- Async-ready with tokio
- Support for 10,000+ node graphs in-memory
- <100ms query latency for typical operations
- Full integration test coverage

---

## Release Checklist

When preparing a new release:

- [ ] Update version in `Cargo.toml`, `pyproject.toml`, and `__init__.py`
- [ ] Update this `CHANGELOG.md` with new section
- [ ] Ensure all tests pass (`cargo test`)
- [ ] Run code quality checks (`cargo clippy`, `cargo fmt`)
- [ ] Create git tag: `git tag -a v0.1.0 -m "Release v0.1.0"`
- [ ] Push tag: `git push origin v0.1.0`
- [ ] Verify wheels built and published on PyPI
- [ ] Verify release created on GitHub with artifacts

---

## Links

- [Documentation](https://github.com/Mullassery/PyBlastRadius/blob/main/docs/)
- [GitHub Issues](https://github.com/Mullassery/PyBlastRadius/issues)
- [PyPI Package](https://pypi.org/project/pyblastradius/)

---

**Note**: This project follows Semantic Versioning. For updates and feature requests, see [Contributing](CONTRIBUTING.md).
