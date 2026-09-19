# ROADMAP_HONEST.md

Blunt, code-grounded status. No hedge words for things that are simply not
built. See README.md's Features / Known Issues / Platform Support sections
for the feature-by-feature detail this file summarizes into four buckets.
Last verified: 2026-09-19/20 by reading the source and actually running the
build, lint, and test suites (commands and output below each bucket where
relevant).

## 1. Built and tested (verified working)

- **Graph model, traversal, serialization** (`src/graph/`) — 76 Rust unit
  tests pass (`cargo test --all-features`, verified 2026-09-19).
- **`BlastRadiusAnalyzer`, `CriticalityScorer`, `Simulator`** — real
  BFS/scoring/cascade logic, exercised by the Rust test suite and by
  `tests/test_analysis.py` against the compiled `pyblastradius._core`
  extension (35 pytest tests pass — see command below).
- **`TerraformDiscoverer`** — parses real `.tf` via `hcl-rs`, derives edges
  from actual resource references. Covered by dedicated tests in
  `src/discovery/terraform.rs` (e.g. `test_discovers_any_resource_type_not_just_the_three_hardcoded_ones`,
  `test_real_reference_produces_a_real_edge_bare_traversal`).
- **`DbtManifestParser`** — genuinely parses a dbt `manifest.json` for
  column lineage; covered by `integrations::dbt_integration::tests`.
- **REST API server auth** (`python/pyblastradius/server.py`) — bearer
  token auth (`PYBLASTRADIUS_API_KEY`/`_KEYS`) and CORS allowlist. Covered
  by `tests/test_server.py`, verified passing 2026-09-19 (see Validation
  below for the exact reproduction steps and the `httpx` gotcha).
- **README's "Kubernetes/OTel: Not implemented" claim in Platform Support
  is correct for the Python-facing API** — verified by reading
  `python/pyblastradius/discovery.py` directly: `KubernetesDiscoverer.discover()`
  only globs for local `k8s`/`kubernetes` YAML files and then does
  `pass` in the loop body (never parses them), always returning an empty
  graph (or raising `ValueError` if no kubeconfig path exists at all).
  `AirflowDiscoverer.discover()` and `OtelDiscoverer.discover()` are
  one-line `return Graph()` stubs. None of these import from `_core` —
  unlike `TerraformDiscoverer`, which `python/pyblastradius/__init__.py`
  imports directly from the real Rust extension.
- **`cargo fmt -- --check`** — clean.
- **`cargo clippy --all-targets -- -D warnings`** — clean (fixed 9 real
  errors during this pass; see Technical Debt).
- **`maturin sdist`** — LICENSE now verified present in the built tarball
  (`tar -tzf ... | grep LICENSE` → `pyblastradius-0.3.0/LICENSE`) after
  adding `include = ["LICENSE"]` to `[tool.maturin]` in pyproject.toml.

## 2. Built but not tested / unverified

- **`HiddenDependencyDetector.detect_from_trace_gaps` /
  `.detect_semantic_coupling`** — real heuristic logic, has unit tests, but
  never exercised against real trace/metrics data end-to-end (only
  synthetic fixtures).
- **Chat/incident-management payload builders** (`src/api/integrations.rs`)
  — build correct-looking Slack/PagerDuty payloads (tested for shape via
  `test_slack_integration`, `test_pagerduty_integration`), but the actual
  HTTP send calls are commented out, so "correct payload shape" has never
  been verified against a real Slack/PagerDuty endpoint.
- **`action.yml`'s composite GitHub Action** — see bucket 3, it is broken,
  not merely untested.

## 3. CI errors / broken

- **`action.yml` (the repo's own composite GitHub Action) is non-functional.**
  Its embedded Python script:
  - Calls `BlastRadiusAnalyzer(graph)` as a constructor
    (`action.yml` line ~88) — but `PyBlastRadiusAnalyzer` in `src/python.rs`
    is a unit struct with no `#[new]`; this raises `TypeError` immediately,
    before any analysis runs.
  - Calls `graph.services()` (`action.yml` line ~92) — no such method
    exists on `PyGraph` (`src/python.rs` only exposes `nodes()`, `edges()`,
    `node_count()`, etc.).
  - Calls `Graph.merge([graph, sub_graph])` as if it were a static
    class-level merge (`action.yml` line ~66) — the real `merge` is an
    instance method, `graph.merge(other)`, mutating in place and returning
    `None` (`src/python.rs`, `PyGraph::merge`).
  - Uses the deprecated `::set-output name=x::` command syntax
    (`action.yml` lines ~98-99), which GitHub Actions has disabled — even
    if the API calls above were fixed, outputs would silently not be set.
  This action has never worked as shipped. **Needs a dedicated follow-up
  session** to either rewrite it against the real `pyblastradius` Python
  API or decide it's not worth maintaining and delete it along with
  `.github/workflows/example-blast-radius.yml`.
- **`.github/workflows/example-blast-radius.yml` references a different,
  unverifiable repo**: `uses: Mullassery/pyblastradius-action@v1`. This
  repo's own root `action.yml` suggests *this* repo is meant to be usable
  directly as a composite action (`uses: Mullassery/PyBlastRadius@v1`), not
  via a separately-named `pyblastradius-action` repo. We could not check
  GitHub (network to github.com is blocked in this sandbox) to confirm
  whether `Mullassery/pyblastradius-action` exists. Either way, the action
  it points to is the broken one above, so this workflow does not work
  today regardless of which repo reference is intended.
- **`cargo audit` could not be run locally** in this sandbox (outbound
  network to `github.com` — where `cargo-audit` fetches the RustSec
  advisory DB from — is blocked here; `crates.io` and PyPI were reachable).
  A `cargo audit` job was added to `.github/workflows/ci.yml` in this pass
  so it will run for real on GitHub's runners, but its actual result is
  unverified as of this writing — check the Actions tab after the next
  push.
- **`Cargo.lock` was gitignored despite this crate producing a shipped
  application** (a Python extension module + formerly a CLI binary), which
  is the same "Cargo.lock must be committed" bug pattern seen across other
  repos in this org. Two concrete downstream effects: (1) builds were not
  reproducible from a fresh clone — whatever versions happened to resolve
  locally is what you got; (2) `ci.yml`'s cargo cache steps key on
  `hashFiles('**/Cargo.lock')`, but since `Cargo.lock` didn't exist in a
  fresh checkout (only gets generated after `cargo build` runs), the cache
  key was effectively constant across dependency changes, defeating the
  purpose of hashing it. Fixed in this pass: removed from `.gitignore` and
  committed `Cargo.lock`.

## 4. Features not yet functional (not built — stated plainly, no hedging)

Mirrors README.md's Known Issues / Platform Support, restated here without
hedge language for the four-bucket convention:

- **Airflow discovery** (`AirflowDiscoverer`) — does not exist as real
  parsing; always returns an empty graph.
- **`AnomalyDetector`** — does not exist; always returns empty results, not
  wired to any metrics/trace source.
- **`HiddenDependencyDetector.detect_from_query_logs` /
  `.detect_from_config`** — explicit MVP stubs, always return empty.
- **`OtelCollectorClient` / `StatGuardianClient`** (in
  `pyblastradius.integrations`) — placeholders returning empty/hardcoded
  fake data, not live results.
- **GraphQL API** — does not exist as a servable endpoint. `src/api/graphql.rs`
  is a query-string builder with nothing executing it.
- **Sending chat/incident-management alerts** — the payload-building code
  exists (see bucket 2) but does not send anything, and is not exposed to
  Python at all.
- **Neo4j backend, GraphQL Federation, Kubernetes Operator, Helm chart,
  ML-based failure prediction** (all listed under CHANGELOG's
  `[Unreleased] > Planned`) — no code, no design doc, not started.

## Technical Debt (concrete, file:line, honestly assessed)

Fixed during this pass (small, safe, mechanical — listed for the record,
not left as open debt):

- `src/graph/graph.rs:254`, `src/graph/traversal.rs:207` — unused
  `NodeType` imports; `cargo clippy --all-targets` errors. Removed.
- `src/analysis/blast_radius.rs:178`, `src/analysis/simulator.rs:215-216`,
  `src/analysis/mod.rs:68,75`, `src/graph/traversal.rs:231`,
  `src/integrations/statguardian_integration.rs:262` — `.len() > 0` /
  `.len() >= 1` instead of `!.is_empty()` (`clippy::len_zero`). Fixed.
- **Root cause of the above going undetected**: `ci.yml`'s Clippy job ran
  plain `cargo clippy -- -D warnings`, which does not check `#[cfg(test)]`
  code. All 9 errors above were inside test modules. Changed the CI job to
  `cargo clippy --all-targets -- -D warnings` so this class of gap doesn't
  silently recur.
- `pyproject.toml`'s `dev` extra was missing `httpx`, which
  `fastapi.testclient.TestClient` requires transitively. Without it,
  `pip install -e .[dev]` followed by `pytest` fails 4 of 35 tests in
  `tests/test_server.py` with an import error (not a real regression).
  Added `httpx>=0.24` to `dev`. Note: the error message from this
  particular Starlette version is itself misleading — it says "requires
  the httpx2 package" — but installing `httpx` (not a package literally
  named `httpx2`) is what actually fixes it; verified by reproducing both
  the failure and the fix.

Left as open debt — needs a dedicated follow-up session, not fixed in this
documentation-first pass:

- **`python/pyblastradius/config.py` (257 lines) has zero test coverage.**
  No test in `tests/` imports or exercises it at all
  (`grep -rn "config" tests/` finds nothing), despite `blastradius.yml`
  config-file support being an advertised, README-documented feature. This
  is the single largest test-coverage gap in the Python layer.
- **`python/pyblastradius/cli.py` — 44% coverage** (135/239 lines missed
  per `pytest --cov` run 2026-09-19), including large blocks at
  lines 34-47, 52-79, 91-111, 126-166, 211-231, 267-283, 344-368. The CLI
  is the primary documented user interface; more than half its lines are
  unexercised by the test suite.
- **Real, tested Rust-level Kubernetes and OTLP parsing code
  (`src/discovery/kubernetes.rs`, `src/discovery/otlp.rs`) is sitting
  completely unreachable from Python.** Both have genuine parsing logic
  and passing unit tests (e.g.
  `test_kubernetes_discovery_parses_real_service_manifest`,
  `test_kubernetes_discovery_infers_edges_from_env_vars`,
  `test_cross_service_span_becomes_calls_edge`,
  `test_latency_is_derived_from_real_span_timing` — all pass under
  `cargo test`), but there is no `PyKubernetesDiscoverer`/`PyOtelDiscoverer`
  in `src/python.rs`, so none of it is exposed via pyo3. The Python-facing
  `KubernetesDiscoverer`/`OtelDiscoverer` in
  `python/pyblastradius/discovery.py` are separate, disconnected
  pure-Python stubs that always return an empty graph. This is exactly the
  same situation `TerraformDiscoverer` was in before v0.3.0 wired it up
  (see CHANGELOG's `[0.3.0]` entry) — meaning a large fraction of "a second
  real discovery engine," already listed as this project's top near-term
  roadmap item in README.md, may already exist and just need pyo3 bindings
  plus swapping the Python stub for the real import, rather than being
  built from scratch. Worth a dedicated follow-up session to verify the
  Rust-level code's quality/completeness and wire it up, the same way
  Terraform was.
- **`action.yml` / `example-blast-radius.yml`** — see bucket 3. Needs a
  decision (fix vs. delete) and, if fixed, a real end-to-end test of the
  composite action in a scratch repo.
- **CONTRIBUTING.md's "Adding a New Discovery Engine" example** references
  `src/bin/cli.rs`, which git history shows was intentionally removed
  ("fix: Move CLI code to examples/ for wheels-only library build",
  "fix: Remove src/bin/cli.rs from git tracking"). The CLI is now
  `python/pyblastradius/cli.py`. This example section is stale and would
  mislead a new contributor about where to wire up a new discoverer's CLI
  flag; not fixed in this pass (needs someone who understands the current
  Python CLI's `UriParser`/scan dispatch to write an accurate example).
- **No automated Python dependency scanning in CI** — `cargo audit` was
  added, but there's no `pip-audit` (or equivalent) CI job for the Python
  side. A one-time manual `pip-audit` run during this pass found no known
  vulnerabilities in the resolved dependency set, but nothing re-checks
  this on an ongoing basis.
- **Coverage measurement gap**: `pytest --cov` reports 0% for
  `python/pyblastradius/server.py` even though `tests/test_server.py`
  passes — because those tests spawn the server code in a subprocess
  (`_run()` helper) and `coverage.py` doesn't instrument subprocesses by
  default. The 0% number in the coverage report is a measurement artifact,
  not evidence the server is untested; noted here so nobody "fixes" the
  coverage number without realizing this, and so nobody mistakes 0% for
  "actually untested" (it does have real test coverage, just not measured).

## Validation performed during this pass (2026-09-19/20)

Commands actually run, with real results (macOS, arm64; system Python 3.9
lacks a linkable libpython, so `PYO3_PYTHON`/a 3.11 venv was needed — see
CONTRIBUTING.md's new "Testing the Python Bindings" section):

```
cargo fmt -- --check                        # clean
cargo clippy --all-targets -- -D warnings   # clean (after fixes above)
cargo test --all-features                   # 76 passed; 0 failed
python3.11 -m venv .venv && source .venv/bin/activate
pip install maturin && maturin develop --release
pip install -e ".[dev,server]"
pytest tests/ -q                            # 35 passed; 0 failed
maturin sdist                               # LICENSE present in tarball
pip-audit                                   # "No known vulnerabilities found"
cargo audit                                 # could not run (network to
                                             # github.com blocked in this
                                             # sandbox); job added to CI
                                             # instead, unverified result
```
