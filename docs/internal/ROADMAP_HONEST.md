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
- **`KubernetesDiscoverer` / `OtelDiscoverer`** — at the time of this
  2026-09-19/20 audit, README's "Kubernetes/OTel: Not implemented" claim
  was correct for the Python-facing API: `python/pyblastradius/discovery.py`'s
  `KubernetesDiscoverer.discover()` only globbed for local `k8s`/`kubernetes`
  YAML files and then did `pass` in the loop body (never parsing them),
  and `OtelDiscoverer.discover()` was a one-line `return Graph()` stub.
  Neither imported from `_core`, unlike `TerraformDiscoverer`. **Since
  then (2026-10-05): both wired up**, the same way `TerraformDiscoverer`
  was — `src/python.rs`'s `PyKubernetesDiscoverer`/`PyOtelDiscoverer` now
  expose the real `src/discovery/kubernetes.rs`/`src/discovery/otlp.rs`
  parsers. `python/pyblastradius/__init__.py` imports both from `_core`;
  the old pure-Python stubs are deleted from `discovery.py`. Covered by
  `tests/test_discovery_bindings.py` (7 tests against the real compiled
  extension — real manifest parsing, real OTLP receiver bind/timeout/GIL
  release, `kubeconfig`/`context`/`namespaces` interface-compat behavior).
  `AirflowDiscoverer.discover()` is still a one-line `return Graph()` stub
  — no equivalent real Rust-level Airflow parsing exists to wire up.
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

## 3. CI errors / broken

- ~~`action.yml` (the repo's own composite GitHub Action) is
  non-functional~~ **Fixed 2026-09-21**: rewritten against the real
  `pyblastradius` Python API (`BlastRadiusAnalyzer.analyze(graph, node_id,
  max_depth=...)` staticmethod, `graph.nodes()` filtered by
  `NodeType.Service` instead of a nonexistent `services()`, instance-method
  `graph.merge(other)` instead of a static `Graph.merge([...])`), switched
  `::set-output` to `$GITHUB_OUTPUT`, and fixed a step-id mismatch that
  meant the top-level `outputs:` block was reading from the wrong step
  even before the API bugs. Verified by running the exact embedded Python
  end-to-end against a synthetic Terraform fixture — see CHANGELOG.md
  `[Unreleased] > Fixed` for the full verification detail.
- ~~`.github/workflows/example-blast-radius.yml` references a different,
  unverifiable repo~~ **Fixed 2026-09-21**: confirmed via authenticated
  `gh api`/`gh repo view` (this sandbox now has real network + an
  authenticated `gh`, unlike the earlier pass that recorded this as
  unverifiable) that `Mullassery/pyblastradius-action` does not exist and
  that `Mullassery/PyBlastRadius` has no `v1` tag/release. Repointed to
  `uses: Mullassery/PyBlastRadius@main`.
- **`cargo audit` now runs locally (network to github.com is reachable in
  this environment as of 2026-09-21, unlike the earlier pass) and finds 3
  real vulnerabilities**, not yet fixed (this is a version-bump problem,
  out of scope for a quick-fix pass — flagged here for a dedicated
  follow-up):
  - `RUSTSEC-2026-0258` — `h2 0.3.27` unbounded empty DATA frames. Pulled
    in transitively via `reqwest 0.11.27` -> `hyper 0.14.32` -> `h2`; fixing
    requires bumping `reqwest` off its `0.11` line (a major version bump,
    since `hyper 1.x`/`h2 0.4.16` aren't reachable from `hyper 0.14`).
  - `RUSTSEC-2025-0020` / `RUSTSEC-2026-0177` — `pyo3 0.23.5` (buffer
    overflow risk in `PyString::from_object`; missing `Sync` bound on
    `PyCFunction::new_closure` closures). Fix requires `pyo3` >=0.24.1 /
    >=0.29.0 — a major bump touching every binding in `src/python.rs`.
  - `RUSTSEC-2025-0134` — `rustls-pemfile 1.0.4` is unmaintained (warning,
    not a vulnerability).
  The `cargo audit` job already added to `.github/workflows/ci.yml` will
  now actually catch these on CI runners too.
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

- ~~**`python/pyblastradius/config.py` (257 lines) has zero test coverage.**~~
  **Fixed:** `tests/test_config.py` covers file loading, env-var expansion,
  validation, CLI merging, and round-tripping; coverage is ~82%.
- **`python/pyblastradius/cli.py` — 44% coverage** (135/239 lines missed
  per `pytest --cov` run 2026-09-19), including large blocks at
  lines 34-47, 52-79, 91-111, 126-166, 211-231, 267-283, 344-368. The CLI
  is the primary documented user interface; more than half its lines are
  unexercised by the test suite.
- ~~Real, tested Rust-level Kubernetes and OTLP parsing code
  (`src/discovery/kubernetes.rs`, `src/discovery/otlp.rs`) is sitting
  completely unreachable from Python~~ **Fixed 2026-10-05**: added
  `PyKubernetesDiscoverer`/`PyOtelDiscoverer` to `src/python.rs`'s
  `#[pymodule]`, the same way `TerraformDiscoverer` was wired up in
  v0.3.0. `pyblastradius.KubernetesDiscoverer`/`OtelDiscoverer` now import
  the real bindings instead of the old disconnected pure-Python stubs
  (deleted from `discovery.py`). See "What actually works" above and
  CHANGELOG's `[Unreleased]` entry for the full detail, including the
  OTLP receiver's GIL-release behavior during its blocking collection
  window (verified with a concurrent-thread test, not just a lack-of-crash
  check) and the `kubeconfig`/`namespaces` honesty notes.
- ~~`action.yml` / `example-blast-radius.yml`~~ **Fixed 2026-09-21** — see
  bucket 3. Note a full end-to-end test of the composite action running
  inside actual GitHub Actions infrastructure (as opposed to running its
  embedded Python logic locally, which was done) is still outstanding —
  worth doing once this lands on GitHub.
- ~~CONTRIBUTING.md's "Adding a New Discovery Engine" example references
  `src/bin/cli.rs`~~ **Fixed 2026-09-21** — rewritten to describe the
  actual pyo3-binding + `UriParser.create_discoverer` wiring path; also
  fixed a second stale reference in "Clone & Build" (`cargo run --
  --version`, which fails today — no `[[bin]]` target exists).
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
