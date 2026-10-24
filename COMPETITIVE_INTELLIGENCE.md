# Competitive intelligence in roadmap and upgrade work

**Policy date:** 2026-10-19

Competitive findings are an input to product decisions, not a one-off report.
Every pending roadmap item, new feature, and routine dependency upgrade should
check the repository's [`AUDIT_FINDINGS.md`](AUDIT_FINDINGS.md) before work
starts, and record the decision in the PR or changelog.

## Required check before building

1. **Read `AUDIT_FINDINGS.md`.** Find the relevant competitor row and the
   ranked next actions. If the finding is stale, update it with a dated source.
2. **Name the competitor and the user problem.** A feature is not justified by
   "competitor has it". State the workflow it unlocks, who needs it, and how
   often. Mark unverified demand as a hypothesis.
3. **Prefer reuse over new code.** Check whether an existing Mullassery repo
   already provides the capability (see `BUNDLES.md` in the profile repo and
   the bundle role file in this repo, when present). Typical reuse:
   - crawling/fetching/robots/SSRF → **MudFish**
   - architecture maps / PR diffs → **CodeAtlas**
   - data-quality contracts / validation → **StatGuardian**
   - token counting / cost tables → **PyTokenCalc**
   - dependency graph / advisories / SBOM → **PyDeepMesh**
   - segmentation / CLV / churn → **ClusterAudienceKit**
   - activation / lineage / PII → **PyReverseETL**
   - fleet state sync → **SyncFleet** (only after conflict resolution)
4. **Choose one disposition:** build, integrate, defer, or reject. Write it
   down. A rejection with a reason is a successful outcome.
5. **For routine upgrades**, check whether the upgrade closes a known
   competitor gap or breaks a differentiated capability. Run the tests that
   cover the affected path; do not merge a green build that silently changes a
   user-visible contract.

## PR / changelog checklist

- [ ] Competitor and source URL/date recorded (or "no direct competitor").
- [ ] User impact and frequency stated; unverified demand labelled.
- [ ] Reuse check done against sibling repos/bundles.
- [ ] Failure mode and rollback considered (especially auth, PII, cost).
- [ ] Docs/roadmap status updated honestly.
- [ ] Tests or a manual verification command included.

## Review cadence

- Refresh the competitor row when a release changes the capability.
- Re-check the full `AUDIT_FINDINGS.md` at least quarterly and before a major
  release.
- If a finding is wrong or stale, correct it in the same PR that relies on it.

## Non-goals

- Copying features for parity.
- Adding a dependency just because a competitor uses it.
- Treating star counts or download counts as proof of user need.

## OSS gap analysis — 2026-10-24

Scope: PyBlastRadius assessed against four maintained projects on dependency and lineage discovery, blast-radius modeling, cascade simulation, fault injection, service ownership, API/UI, and CI integration. Chaos tools are included as the empirical counterpoint to a predictive graph, not as feature-parity targets.

### Comparable OSS projects

| Project | GitHub | License | Last activity | Why it is the comparison |
| --- | --- | --- | --- | --- |
| Backstage | https://github.com/backstage/backstage | Apache-2.0 | v1.56.2, 2026-10-22 | Software catalog and entity-relation graph; the default OSS answer to "what depends on what and who owns it". |
| Marquez | https://github.com/MarquezProject/marquez | Apache-2.0 | pushed 2026-10-13; latest release 0.50.0, 2024-10-24 | OpenLineage reference server with a lineage graph and UI; direct comparison for runtime data lineage. |
| Chaos Mesh | https://github.com/chaos-mesh/chaos-mesh | Apache-2.0 | v2.8.6, 2026-10-21 | Kubernetes fault injection across network, pod, stress, IO, DNS, and time; empirical failure testing versus our prediction. |
| LitmusChaos | https://github.com/litmuschaos/litmus | Apache-2.0 | 3.33.0, 2026-10-15 | CNCF chaos platform with experiment hubs and workflows; the other main OSS path to validating blast radius. |

### Capability comparison

| Capability | Ours | Backstage | Marquez | Chaos Mesh | Litmus | Gap |
| --- | --- | --- | --- | --- | --- | --- |
| Dependency discovery | Yes | Partial | Partial | No | No | None |
| Runtime lineage ingestion | Partial | No | Yes | No | No | Medium |
| Column-level lineage | Partial | No | Partial | No | No | Medium |
| Blast-radius impact analysis | Yes | Partial | Partial | Partial | Partial | Ahead |
| Cascade simulation | Yes | No | No | No | No | Ahead |
| Criticality/SPOF scoring | Yes | No | No | No | No | Ahead |
| Failure-type modeling | Partial | No | No | Yes | Yes | Large |
| Empirical fault injection | No | No | No | Yes | Yes | Large |
| Live-cluster discovery | No | Partial | No | Yes | Yes | Large |
| Service catalog/ownership | No | Yes | No | No | No | Large |
| API access | Yes | Yes | Yes | Yes | Yes | None |
| Visualization | Partial | Yes | Yes | Yes | Yes | Medium |
| Alerting/incident integration | No | Partial | No | Partial | Partial | Medium |
| GitHub Action/CI | Yes | No | No | No | No | Ahead |
| Business-impact quantification | Partial | No | No | No | No | Ahead |
| Platform packaging | Partial | Yes | Yes | Yes | Yes | Large |

Nuance: our discovery parses local Terraform HCL, Kubernetes YAML manifests, Airflow Python source heuristically, OTLP spans from a receiver we bind, OpenLineage event files, and a local dbt `manifest.json`. There is no live Kubernetes API client, cloud inventory, or catalog connector. Our simulation walks only edges marked critical; `FailureType` is recorded as a label but does not change propagation, recovery time is the maximum node value, and business impact is the mean node exposure scaled to 0-100. No probabilities, time dynamics, retries, SLOs, or observed outcomes feed the model. Marquez stores OpenLineage lineage and exposes a lineage graph/UI; column-level support depends on OpenLineage facets, so it is marked partial. Litmus exposes GraphQL through its portal while the others expose REST APIs. Backstage's graph is catalog relations, not impact scoring. Chaos Mesh and Litmus actually inject failures and observe the result; they do not predict an impact set.

### Gaps that matter (ranked)

1. **Predictions are never calibrated against observed failures** — the blast-radius number is a static BFS result with no ground truth. Chaos Mesh and Litmus can induce the failure and measure what actually breaks; we cannot. Recording observed impact from chaos runs and comparing it to predictions is the credibility gap. L effort, but integration (webhook/experiment metadata) is S/M.
2. **The cascade model is too coarse to defend** — only `is_critical` edges propagate, failure type is ignored, there are no probabilities or latency/error propagation, and recovery is the slowest node's value. A deterministic-looking score overstates precision. M effort to add weighted propagation and uncertainty.
3. **Discovery is static and local** — no live Kubernetes API, cloud resource inventory, or service catalog, so the graph drifts from production and misses ownership. Backstage and Kubernetes-native chaos tools reflect live state. M/L effort; start with a read-only Kubernetes connector.
4. **Lineage depth is below Marquez** — OpenLineage events are flattened into job/table nodes, dbt lineage is partial, and there is no lineage store or query API. Runtime column-level impact across data pipelines is therefore coarse. M effort.
5. **No alerting or incident integration** — Rust payload builders exist but never send, and are not exposed to Python. Users must wire Slack/PagerDuty/Opsgenie themselves. S/M effort.
6. **Packaging is one platform** — PyPI 0.4.4 ships only a CPython 3.12 macOS arm64 wheel plus sdist. Linux CI containers and other Python versions must compile Rust, which blocks the GitHub Action's audience. S/M effort.

### Where we are differentiated

- A predictive, evidence-labeled graph workflow (impact set, criticality, SPOF, cascade, business exposure) in one OSS CLI, rather than a catalog or a chaos injector.
- Multiple discovery sources in one graph model: Terraform, Kubernetes manifests, Airflow source, OTLP spans, OpenLineage events, and dbt manifest.
- An authenticated REST API, a working GitHub Action, and Graphviz/DOT plus HTML output, where Backstage and Marquez are portals/stores and Chaos Mesh/Litmus are experiment runners.
- Honest boundary documentation: stubs such as anomaly detection, alert delivery, and GraphQL are labeled rather than presented as working.

### Disposition

- **Build:** observed-vs-predicted calibration, probability/latency-aware cascade propagation, read-only live Kubernetes discovery, and a lineage store with column facets.
- **Integrate/reuse:** Chaos Mesh or Litmus for empirical fault injection, OpenLineage/Marquez for lineage ingestion, Backstage for ownership/catalog, and StatGuardian for data-quality signals. Do not rebuild them.
- **Defer/reject:** service-catalog/portal parity, incident-management platform features, a served GraphQL API, and enterprise multi-region/HA scope until the core prediction is calibrated.

### Sources

- https://github.com/backstage/backstage and https://backstage.io/docs/features/software-catalog/ (accessed 2026-10-24)
- https://github.com/MarquezProject/marquez and https://openlineage.io/ (accessed 2026-10-24)
- https://github.com/chaos-mesh/chaos-mesh (accessed 2026-10-24)
- https://github.com/litmuschaos/litmus (accessed 2026-10-24)
- Repo files inspected: `README.md`, `COMPETITIVE_INTELLIGENCE.md`, `AUDIT_FINDINGS.md`, `CHANGELOG.md`, `ROADMAP_HONEST.md`, `pyproject.toml`, `Cargo.toml`, `src/analysis/simulator.rs`, `src/analysis/blast_radius.rs`, `src/discovery/airflow.rs`, `python/pyblastradius/openlineage.py`, `python/pyblastradius/discovery.py`
