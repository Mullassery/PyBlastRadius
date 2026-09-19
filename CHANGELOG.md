# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-08-26

### Added

- **Real Terraform dependency discovery.** `TerraformDiscoverer` now parses
  `.tf` files with a real HCL2 parser (`hcl-rs`) instead of the previous
  regex-based extraction (limited to 3 hardcoded AWS resource types, and
  fooled by multi-line blocks, nested blocks, or comments containing
  lookalike declarations). Any `resource "type" "name" { ... }` block is
  discovered now, not just RDS/ElastiCache/ECS. Dependency *edges* are the
  bigger fix: they previously came from a hardcoded example dependency
  list (`api_gateway -> auth_service -> production`, etc.) that had
  nothing to do with the actual `.tf` content being scanned -- edges are
  now derived from real resource references found in each resource's own
  attribute values, both bare traversals
  (`aws_db_instance.production.endpoint`) and interpolated strings
  (`"${aws_db_instance.production.endpoint}"`).
- **`TerraformDiscoverer` is now reachable from Python at all.** The real
  Rust implementation existed before this release but was never exposed
  via pyo3 -- `pyblastradius.TerraformDiscoverer` (used by `cli.py`,
  `config.py`, `server.py`) resolved to a pure-Python placeholder in
  `pyblastradius/discovery.py` that walked matching files and discarded
  their contents entirely (`# For now, just create mock nodes` / `pass`),
  always returning an empty graph regardless of what `scan`'s
  `terraform://` source pointed at. `pyblastradius/__init__.py` now
  imports the real `_core` binding instead; the fake class has been
  removed from `discovery.py`.
- **`benches/large_graph.rs`**: real `criterion` benchmarks (graph
  construction, `BlastRadiusAnalyzer::analyze`,
  `CriticalityScorer::score_all`) on layered graphs up to 1,000 nodes with
  realistic fan-out topology. `criterion`/`[profile.bench]` were declared
  in `Cargo.toml` with a `// Benchmarks (add later)` comment, but no
  `benches/` directory or `[[bench]]` target existed before this.

## [0.2.1] - 2026-08-23

### Fixed

- `CriticalityScorer`: `criticality_score` was divided by 100 twice, collapsing the
  documented 0-100 scale down to ~0-1. Fixed the double-division.
- `CriticalityScorer::is_single_point_of_failure`: checked a node's *predecessor's*
  out-degree instead of the node's own in-degree, producing false negatives on branching
  topologies (a node with exactly one predecessor was missed as a SPOF if that predecessor
  also happened to reach other nodes). Fixed to check in-degree directly; added a
  regression test for the branching case.
- `ColumnLineageTracker::track_column` split `"schema.table.column"` identifiers into only
  their first and last dot-separated segments, discarding the middle table segment --
  `find_downstream_impact()` could never match a real 3-part identifier as a result. Fixed
  with `rsplit_once('.')`.
- `test_hidden_dependency_detection`'s fixture graph never met
  `detect_from_trace_gaps`'s own documented threshold (a node needs > 3 downstream
  neighbors), so it asserted on an always-empty result regardless of whether detection
  actually worked. Gave the fixture a graph that genuinely exercises the heuristic.
- `test_blast_radius_analysis` picked its start node via `nodes[0]` after collecting
  `graph.nodes()` (backed by a `HashMap`, whose iteration order isn't stable across
  process runs) into a `Vec` -- flaky depending on which node landed at index 0. Now looks
  up the intended node by name.
- CI: replaced the archived `actions-rs/toolchain` action (broken/unmaintained) with
  `dtolnay/rust-toolchain@stable` across all jobs; bumped `actions/checkout`,
  `actions/cache`, `actions/upload-artifact`, and `codecov-action` to current major
  versions; pinned Python 3.12 in the Build job (macos-latest's default python3 recently
  became 3.14, which PyO3 0.23.5 rejects); marked the README's ASCII architecture diagram
  as a `text` fence instead of an implicit (and invalid) Rust doctest.
- `cargo fmt`/`cargo clippy -D warnings` had never actually run cleanly before (blocked by
  the broken CI action above) -- fixed ~50 real warnings surfaced once they could run.

No public API changes.

## [Unreleased]

### Planned
- ML-based failure prediction with anomaly consensus scoring
- Neo4j backend for 100K+ node graphs
- GraphQL Federation for multi-service deployments
- Kubernetes Operator for continuous discovery
- Helm chart for self-hosted deployment
- Advanced hidden dependency detection (query logs, configuration mining)

## [0.2.0] - 2026-08-05

Real git tag (`v0.2.0`), dated from `git log`. No entry existed for this
release before this pass; added from the verified commit history and the
Project Status table in README.md rather than reconstructed from memory.

### Added
- First working `pyblastradius._core` PyO3 extension module — end-to-end
  `import pyblastradius` against real compiled Rust code, not a pure-Python
  placeholder.
- Real end-to-end analysis via the extension: `Graph`/`Node`/`Edge`,
  `BlastRadiusAnalyzer`, `CriticalityScorer`, `Simulator`.

### Known limitations as of this release
Discovery engines (Kubernetes, Airflow, OTel) were not implemented yet —
see Platform Support in README.md for current status of each.

## [0.1.0] - 2026-08-05

Real git tag (`v0.1.0`), dated from `git log`.

**This entry was originally written before implementation started and
described intended v1.0 scope, not what had actually shipped** — it listed
a GraphQL API, Slack/PagerDuty integrations, and full Kubernetes/Airflow/
OpenTelemetry discovery as "Added" in this release. None of that was true
at the time and, per README.md's Known Issues (verified 2026-09-19), most
of it (Kubernetes/Airflow/OTel discovery, the GraphQL endpoint, and sending
actual Slack/PagerDuty webhooks) is still not implemented today, several
releases later. Corrected below to only what the actual `v0.1.0` git tag's
codebase supports; see README.md for the authoritative current-state
breakdown of every feature.

### Added
- Initial project scaffold: Rust workspace, `petgraph`-based graph model
  (node/edge types, properties), graph traversal (BFS/DFS/topological
  sort), JSON/GraphML/Dot serialization.
- Early-stage discovery, analysis, and integration code for Kubernetes,
  Terraform, Airflow, OTel, GraphQL, Slack/PagerDuty, and dbt/StatGuardian
  — present in the codebase from this point on, but not wired up to real
  parsing/live data/Python bindings until later releases (Terraform in
  v0.3.0, the Python extension module itself in v0.2.0). Treat this as
  "code exists" not "feature works" — see README.md Known Issues for what
  is real today.
- CI/CD scaffold (GitHub Actions), example configs, initial docs.

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
