# Archive

Historical phase-completion and pre-implementation planning documents, kept
for reference only. For current, accurate status (features, install, CLI
usage, architecture, known gaps) see [`../../README.md`](../../README.md) —
that is the single source of truth for this project's current state.

## Phase-completion reports (dated August 2026)

- `PHASE_0_2_COMPLETION.md`
- `PHASE_3_4_COMPLETION.md`
- `PHASE_4_REFOCUS.md`

## Pre-implementation planning docs (dated August 2026, archived 2026-09-19)

These were written before most of the code existed and describe an intended
design, not what was actually built. Several no longer match reality at
all — e.g. `ARCHITECTURE.md` and `QUICK_START.md` describe a Rust-only CLI
binary (`src/bin/cli.rs`, `--kubernetes`/`--terraform` flags) that was later
replaced by the current Python package + `pyblastradius._core` Rust
extension design, and show fabricated example output implying Kubernetes/
Airflow discovery works when — per the current README's Known Issues — both
still always return an empty graph. Kept for historical context only; do
not follow instructions or trust status claims in these files.

- `BUILD_PLAN.md`
- `GAP_ANALYSIS.md`
- `MARKET_RESEARCH.md`
- `PRODUCT_VISION.md`
- `RESEARCH_SUMMARY.md`
- `INPUT_DESIGN.md`
- `IMPLEMENTATION_CLI.md`
- `IMPLEMENTATION_PYTHON_API.md`
- `GETTING_STARTED.md`
- `QUICK_START.md`
- `ARCHITECTURE.md`
