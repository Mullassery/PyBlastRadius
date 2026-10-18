# Audit findings — 2026-10-19

This file records the repository-specific findings from the portfolio audit
conducted on 2026-10-19. It is documentation only; no code or release artifacts
were changed in this pass unless explicitly noted under "Implementation status".

Sources: the repository's own README/source/workflows, GitHub metadata and
Actions runs, PyPI metadata, and official competitor documentation. Claims are
marked as verified, inferred, or open questions. "Absent" means not found in the
inspected repository/docs, not proof that it cannot exist.

## What it is

Operational blast-radius intelligence: dependency graph, criticality scoring, cascade simulation, and discovery from Terraform/Kubernetes/OTLP.

## Verified strengths

- Rust core; BFS impact; criticality; cascade simulation; graph I/O; API key auth/CORS; Terraform/K8s/OTLP discovery; 1,000-node benchmark; 6 test files.
- Terraform/K8s/OTLP discovery is real; API server is authenticated.

## Verified gaps and user-facing risks

- Airflow discovery returns an empty graph; broad 'business impact' inputs are user-supplied; no service-catalog integration.
- 42k README overstates readiness; no production users; macOS arm64 wheel only.

## Direct competitors

- Backstage, Datadog Service Catalog, FireHydrant, Datafold, Monte Carlo.

## Ranked next actions

- P1: implement Airflow discovery (M).
- P1: narrow to PR impact for dbt/Airflow/Terraform + GitHub Action (M).

## Do not build

- Service-catalog/incident platform parity; crowded and integration-heavy.

## Evidence gaps / validation needed

- No validated business-impact model; no real graph data.

## Recommended disposition

Maintain only after narrowing; the broad platform pitch is the risk.

## Implementation status

No code changes in this pass.
