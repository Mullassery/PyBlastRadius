# PyBlastRadius: Product Vision & Roadmap (v2.0 — Research-Backed)

**Research Date**: August 2026  
**Status**: Validated market opportunity  
**Target Launch**: Q4 2026 (v0.1 MVP)

---

## 1. Product Vision

### The Problem

Organizations operate hundreds of interconnected systems (microservices, data pipelines, AI agents, business platforms), and when failures occur, SRE teams spend **4-24 hours on MTTR** manually tracing dependencies and estimating impact.

Existing observability tools (Datadog, New Relic) answer "What is broken?" but leave critical questions unanswered:
- **What will break next?** (Cascading failures are invisible)
- **How bad will it get?** (Business impact is guesswork)
- **Where should I fix first?** (Criticality scoring is missing)
- **Did my deployment cause this?** (Pre-deployment risk is unknown)
- **Is this a data quality issue or an infrastructure outage?** (Fragmented tools hide the truth)

### Why This Matters (Business Case)

**Financial Impact (2026)**:
- Average downtime: **$15,000/minute**
- Enterprise outages: **$1M+/hour**
- One prevented major outage (47 minutes cascading failure): **$705,000 saved**
- ROI for PyBlastRadius: **Pays for itself in one incident prevented**

**Team Impact**:
- Elite SRE teams achieve <60 min MTTR; most teams take 4-24 hours
- 20-40% of time lost to **dependency discovery** (manual tracing)
- Another 20-40% to **triage** (which services are actually affected?)
- Predictive blast radius intelligence cuts these by **50-70%**

### The Solution

**PyBlastRadius** is an operational intelligence platform that automatically discovers dependencies across infrastructure, data pipelines, applications, and AI systems—then predicts which failures cascade and quantifies business impact.

**Core Promise**:
> "Stop reacting to outages. Start predicting them. Know not just what will fail, but why it matters."

### What Makes It Different

| Aspect | Datadog/New Relic | dbt/Airflow Lineage | PyBlastRadius |
|--------|------------------|---------------------|---------------|
| **Discovers** | Applications | Data pipelines | All layers (infra + data + app + AI) |
| **Shows impact** | Services affected | Tables affected | Business outcomes affected + $ cost |
| **Predicts failures** | ❌ | ❌ | ✅ (cascade prediction) |
| **Finds hidden deps** | ❌ | ❌ | ✅ (trace + query analysis) |
| **Reverse ETL aware** | ❌ | ❌ | ✅ (rare competitive advantage) |
| **AI/agent lineage** | ❌ | ❌ | ✅ (emergent need) |
| **Business impact** | ❌ (technical language only) | ❌ | ✅ ($X/hour revenue exposure) |

---

## 2. Core Product Experience

### 2.1 The Three Core Workflows

#### Workflow 1: Rapid Incident Response
**When**: Incident occurs, SRE needs to understand blast radius

```bash
$ pyblastradius analyze kafka-cluster --failure-type outage

Kafka Cluster Outage — Blast Radius Analysis

DIRECTLY IMPACTED (1 hop):
├─ spark-streaming-job-orders
├─ kafka-fraud-detection-consumer
├─ event-processor-v2

INDIRECTLY IMPACTED (2 hops):
├─ feature-store-api (lag increases)
├─ recommendation-engine (stale features)
└─ dashboard-analytics (no events flowing)

BUSINESS IMPACT:
├─ Checkout experience (impaired)
│   └─ Orders backing up (50 orders/min)
│   └─ Revenue leak: ~$250K/hour
├─ Fraud detection (degraded confidence)
│   └─ Risk: Increased fraud leakage
└─ Customer analytics (16hr delay)
   └─ Impact: Executive reporting miss

BLAST RADIUS SCORE: 94/100 (CRITICAL)

RECOVERY STRATEGY:
├─ Failover to backup Kafka cluster (5 min)
├─ Restart streaming jobs (3 min)
└─ Estimated full recovery: 12 min

SUGGESTED ACTIONS:
1. [IMMEDIATE] Activate Kafka cluster failover
2. [WITHIN 2 MIN] Alert fraud team to manual review
3. [WITHIN 5 MIN] Notify customers of ~15min order delay
```

**Value**: Triage from 40 min → 5 min (saves $1.25K for this incident alone)

#### Workflow 2: Pre-Deployment Risk Assessment
**When**: Before deploying changes, understand what could break

```bash
$ pyblastradius analyze-deployment checkout-service-v2.3.0.yaml

DEPLOYMENT RISK ANALYSIS: checkout-service v2.3.0

DEPLOYMENT AFFECTS:
├─ Direct: checkout-service
├─ Blast radius: 23 services + 8 pipelines + 2 AI agents
└─ Customer exposure: 50K+ active users/hour

CRITICAL DEPENDENCIES:
├─ 🔴 payment-processor (no fallback, critical)
├─ 🟡 fraud-detection (degraded mode available)
└─ 🟢 analytics-pipeline (non-blocking)

PRE-DEPLOYMENT CHECKS:
✅ payment-processor is healthy
✅ no active incidents in blast radius
❌ fraud-detection has 2 min latency p99 (elevated)
❌ 3 feature store nodes are down (2/5 available)

RISK ASSESSMENT:
├─ Technical risk: MEDIUM (elevated latency observed)
├─ Business risk: HIGH (payment path affected)
├─ Deployment recommendation: WAIT (defer 30 min until fraud-detection recovers)

SUGGESTED SAFE WINDOW:
├─ Optimal: 2:00 AM UTC (lowest traffic, full blast radius healthy)
├─ Alternative: 14:00 UTC (moderate traffic, most services healthy)

ROLLBACK PLAN:
└─ Previous version had no payments bugs, rollback < 5 min
```

**Value**: Prevents incident before it happens (avoids $705K+ cascade)

#### Workflow 3: Ongoing Dependency Intelligence
**When**: Dashboard shows system health and dependencies in real-time

**Dashboard features**:
- Live dependency graph (infrastructure + data + app + AI layers)
- Health status color-coding (green/yellow/red)
- Blast radius heat map (which systems are single points of failure?)
- Hidden dependencies discovered since last scan (with confidence scores)
- Business criticality ranking (systems by revenue exposure)
- Incident timeline (showing how failures cascaded in past incidents)

---

### 2.2 Integration Points (MVP Phase)

#### Discovery Sources (Static)
- **Kubernetes**: Service manifests, network policies, ConfigMaps
- **Terraform**: AWS/Azure/GCP resources and relationships
- **Airflow**: DAG lineage, task dependencies
- **dbt**: Model dependencies, manifest analysis
- **Docker Compose**: Service definitions
- **OpenAPI/GraphQL specs**: API endpoint dependencies

#### Discovery Sources (Dynamic)
- **OpenTelemetry**: Distributed traces (primary source for runtime deps)
- **Kafka**: Topic subscriptions, consumer groups
- **Data warehouse APIs**: Query lineage (Snowflake, BigQuery, Databricks)
- **Vector DB APIs**: Embedding lineage (Pinecone, Weaviate, Milvus)

#### Observability Integration (For Health/Latency Data)
- **Prometheus**: Scrape metrics for latency, error rates
- **OpenTelemetry Collector**: Ingest spans and metrics
- **Datadog Agent** (future): Real-time status sync

#### Business Impact Mapping (Configuration)
- **YAML-based**: Custom mappings from technical failure → business impact
- **Pre-built templates**: Common patterns (payment failure → revenue, fraud detector → risk)
- **Manual override**: Teams enrich mappings as they learn

---

## 3. Architecture (Phased Layers)

### Phase 1: Foundation (v0.1) — 6 weeks

**Layer 1: Hybrid Dependency Discovery**
- Static discovery: K8s manifests, Terraform, Airflow DAGs, dbt models
- Dynamic discovery: OTel traces (consume from collector or OTLP endpoint)
- Hidden dep detection: Query logs from data warehouses + trace gap analysis
- Accuracy target: 85%+ coverage of actual dependencies

**Layer 2: Unified Graph**
- Rust-based graph engine (petgraph)
- Node types: Service, Database, Pipeline, Topic, VectorDB, CRM, Dashboard, Agent
- Edge types: calls, produces, consumes, queries, feeds, depends_on
- 10,000+ node support (in-memory)

**Layer 3: Basic Blast Radius Analysis**
- BFS traversal of dependency graph
- Categorize by impact depth (direct, indirect, tertiary)
- Calculate blast radius score (0-100)
- Identify critical paths vs. best-effort paths

**Layer 4: Criticality Scoring**
- Score formula: (downstream_count × 0.4) + (criticality × 0.3) + (recovery_complexity × 0.2) + (business_exposure × 0.1)
- Rank all systems by blast radius risk
- Identify single points of failure

**Layer 5: Simple Simulation**
- Simulate outage propagation through graph
- Calculate direct + cascading impact
- Estimate recovery time (based on node properties)

**CLI & Visualization**:
- `scan`: Discover dependencies from configs
- `analyze <service>`: Show blast radius
- `simulate <service>`: Run failure scenario
- `criticality`: Rank systems by risk
- `graph`: Generate interactive HTML visualization
- Interactive Cytoscape-based graph explorer

### Phase 2: Data + Business (v0.2) — 8 weeks

**Layer 6: Data Pipeline Blast Radius**
- Extended discovery: dbt, Spark, Kafka topics
- Data quality checks: null rates, schema drift detection
- Reverse ETL blast radius: "warehouse delay → CRM sync failures → campaign impact"
- Data lineage tracing: Column-level impact analysis

**Layer 8: Business Impact Quantification**
- YAML-based business impact config (easily customizable)
- Mapping: Technical failure → Business consequence → $ impact
- Example mapping:
  ```yaml
  warehouse_delay_30min:
    business_impact: "Executive Reporting"
    metrics_impacted:
      - metric: "revenue_forecast_accuracy"
        impact_pct: -15
    financial_impact: "$250K/day exposure"
  ```
- Dashboard integration: Show revenue exposure in blast radius analysis

**Layer 9: Pre-Deployment Risk Analysis**
- Parse K8s manifests, Terraform plans, deployment descriptors
- Calculate blast radius of proposed changes
- Alert if deploying during incident or into degraded state
- Suggest safe deployment windows

### Phase 3: Scale + Prediction (v1.0) — 12 weeks

**Layer 7: Hidden Dependency Detection** (advanced)
- OTel trace mining: Find undocumented dependencies via span analysis
- Query log analysis: Database query pattern mining
- Configuration mining: ENV var and secret reference extraction
- Semantic analysis: Find implicit dependencies via shared business logic
- Confidence scoring: 0-100% confidence per detected dependency

**Layer 10: Failure Prediction** (emerging)
- ML-based anomaly detection (consensus method)
- Predict latency spikes before impact
- Predict resource exhaustion before cascade
- Predict data quality issues before business sees impact
- Early warning: 20-40 min before customer-facing impact

**Layer 11: Neo4j Backend Option**
- Support for orgs with 10,000+ nodes
- Query federation for multi-cluster scenarios
- Persistent storage vs. ephemeral

---

## 4. Go-to-Market Strategy

### 4.1 Phase 1: Community & Validation (Weeks 1-16)

**Goal**: Build trust with SRE community, validate key assumptions, get early feedback

**Tactics**:
1. **Open-source MVP** (Apache 2.0 license)
   - Publish v0.1 to GitHub with pre-built examples
   - Document with 2-3 real incident scenarios
   - Include sample K8s + Airflow configs for quick demo

2. **Content Marketing**
   - Blog: "How Cascading Failures Happen (And How to Stop Them)"
   - Blog: "The Hidden Dependencies Hiding in Your Infrastructure"
   - Case study: Retrospective on real incident (anonymized)
   - Research: "State of Incident Response 2026" (with PyBlastRadius data)

3. **Community Engagement**
   - Present at KubeCon, DataCouncil, Observability Summit
   - Post on r/devops, SRE Lounge, Hacker News
   - Partner with OTel community

4. **Beta Customer Acquisition** (3-5 customers)
   - Target: SaaS companies with 50-500 engineers
   - Offer: Free tier for 6 months
   - Measurement: Track MTTR improvement, incidents prevented
   - NPS & feedback collection

### 4.2 Phase 2: Enterprise Positioning (Weeks 17-32)

**Goal**: Establish productized offering, build SLA, create packaging tiers

**Tactics**:
1. **Professional Tier Launch** ($5-15K/month)
   - Includes: Support, integrations, custom business impact mappings
   - Add: GraphQL API for programmatic access
   - Add: Webhook notifications for blast radius changes
   - Add: Audit logs for compliance

2. **Sales Motion**
   - Hire first AE
   - Target: Fintech (high cost of downtime), SaaS platforms (competitive pressure)
   - Focus on MTTR reduction + incident prevention value prop
   - ROI calculator: "This product pays for itself in one incident"

3. **Enterprise Features**
   - SSO/SAML support
   - Multi-tenant architecture
   - SLA: 99.9% uptime for service
   - On-prem option (if demanded)

### 4.3 Phase 3: Ecosystem Expansion (Weeks 33+)

**Goal**: Become the standard for blast radius intelligence

**Tactics**:
1. **Strategic Partnerships**
   - Datadog marketplace listing
   - OTel community project
   - Kubernetes ecosystem (CNCF)

2. **Platform Expansion**
   - Native Datadog, New Relic, Splunk integrations
   - Pre-built business impact templates by industry
   - API for third-party apps to query blast radius

3. **AI/ML Enhancement**
   - Failure prediction model (trained on customer data, opt-in)
   - Automatic business impact learning (learn from incidents)
   - Autonomous remediation recommendations

---

## 5. Phased Delivery Roadmap

### MVP: v0.1 (6 weeks, target Oct 2026)
**Target**: SREs at data-heavy + incident-heavy companies

```
Week 1-2: Core architecture + graph engine
Week 3-4: Discovery (K8s + Airflow + Terraform)
Week 5: Blast radius analysis + criticality scoring
Week 6: CLI + basic visualization + docs

Deliverables:
- CLI: scan, analyze, criticality, graph
- Visualization: Interactive HTML graph explorer
- Docs: Quick start (5 min setup), CLI reference, 3 example scenarios
- Tests: 100+ tests (Rust + Python)
- Sample configs: K8s + Airflow + Terraform examples
```

**Success Criteria**:
- [ ] 85%+ dependency discovery accuracy (vs. manual review)
- [ ] <100ms query time for blast radius (1000 node graph)
- [ ] 3-5 beta customers testing
- [ ] 500+ GitHub stars
- [ ] 100+ PyPI downloads/week

### v0.2 (8 weeks, target Dec 2026)
**Target**: Platform teams managing data pipelines + reverse ETL

```
Week 1-2: Extended discovery (dbt, Spark, Kafka, Snowflake)
Week 3-4: Reverse ETL monitoring + data quality integration
Week 5: Business impact engine (YAML config)
Week 6-7: Pre-deployment risk analysis
Week 8: Docs + testing

Deliverables:
- Discovery: dbt, Spark, Kafka, 3 data warehouses
- CLI: analyze-deployment, business-impact
- YAML config templates (10 pre-built scenarios)
- Integration: Reverse ETL (Hightouch, Airbyte)
```

**Success Criteria**:
- [ ] Data pipeline cascades detected with 80%+ accuracy
- [ ] Business impact quantification validated by customers
- [ ] 1,000+ PyPI downloads/week
- [ ] 10+ paying customers in beta

### v1.0 (12 weeks, target Mar 2027)
**Target**: Enterprise SRE orgs, Fortune 500 companies

```
Week 1-3: Advanced hidden dep detection (traces + queries)
Week 4-5: ML-based anomaly detection + failure prediction
Week 6-7: Neo4j backend option
Week 8-9: GraphQL API + Slack/PagerDuty integrations
Week 10-11: Sales/marketing materials + enterprise features
Week 12: GA launch

Deliverables:
- Advanced discovery: Hidden deps (90%+ accuracy)
- Prediction: Anomaly detection + 20-40min early warning
- Scale: Neo4j backend (100K+ nodes)
- Integrations: Slack, PagerDuty, Datadog, New Relic
- Enterprise: SSO, audit logs, SLA, on-prem option
- Pricing: $1-5K (Foundation) / $5-15K (Professional) / $15-50K (Enterprise)
```

**Success Criteria**:
- [ ] 90%+ hidden dependency detection accuracy
- [ ] Failure prediction proves 70%+ useful in production
- [ ] 50+ paying customers
- [ ] $2M+ ARR trajectory
- [ ] 5,000+ PyPI downloads/week

---

## 6. Success Metrics (How We'll Know It's Working)

### Technical Metrics
- **Dependency Discovery Accuracy**: >85% (validated vs. manual tracing)
- **Blast Radius Prediction Accuracy**: >80% (does actual impact match prediction?)
- **Query Latency**: <100ms for typical graphs
- **False Positive Rate**: <5% (excess noise in blast radius)
- **Test Coverage**: >85% (Rust + Python)

### Product Metrics
- **MTTR Improvement**: 50%+ reduction for customers (measure from beta programs)
- **Incident Prevention Rate**: % of predicted cascades that are prevented
- **Time to Blast Radius Analysis**: <5 min for typical incident (vs. 40+ min manual)
- **Business Impact Accuracy**: >80% (does projected $ match actual?

)

### Adoption Metrics
- **PyPI Downloads**: 1K/week (v0.1) → 5K/week (v1.0)
- **GitHub Stars**: 500+ (v0.1) → 5,000+ (v1.0)
- **Paying Customers**: 0 (v0.1) → 50+ (v1.0)
- **NPS Score**: 40+ (launch target)
- **ARR**: $0 (v0.1) → $2M (v1.0)

### Market Metrics
- **Brand Awareness**: Become default tool for "blast radius" in SRE community
- **Integration Ecosystem**: 20+ integrations (Datadog, Splunk, OTel, dbt, Kafka, etc.)
- **Industry Adoption**: Presence in fintech, SaaS, and data-heavy segments

---

## 7. Risk Mitigation

### Risk: Dependency Discovery Accuracy Too Low
**Mitigation**:
- Use hybrid approach (static + dynamic + semantic) from day 1
- Validate accuracy against customer manual reviews
- Iterate on detection algorithms based on feedback
- Publish accuracy benchmarks publicly

### Risk: Market Adoption Slower Than Forecast
**Mitigation**:
- Start with high-pain segments (fintech, SaaS, data platforms)
- Build free tier to drive adoption and data
- Create incident response workflow integration (become habit)
- Partner with consulting firms for implementation

### Risk: Existing Tools Add Blast Radius Features
**Mitigation**:
- Move fast: v0.1 → v1.0 in 26 weeks
- Focus on gaps they ignore (data pipelines, reverse ETL, AI agents)
- Build on OTel (become standard, not proprietary)
- Create strong integration ecosystem so switching is expensive

### Risk: Data Quality Issues Create False Positives
**Mitigation**:
- Build confidence scoring into all predictions
- Allow users to tune thresholds
- Hybrid validation: Don't trust single data source
- Implement feedback loop: "Was this blast radius accurate?" → iterate

---

## 8. Success Definition (What Does v1.0 "Victory" Look Like?)

### By EOY 2027:
- ✅ PyBlastRadius is **the** blast radius intelligence platform for SREs
- ✅ 50+ paying customers across fintech, SaaS, data engineering
- ✅ Customers report **50-70% MTTR improvement**
- ✅ One major incident **prevented** worth $1M+ (validates ROI)
- ✅ Integrations with Datadog, Splunk, dbt, OTel, Kafka
- ✅ $2M+ ARR with path to $10M+
- ✅ 5,000+ GitHub stars, industry recognition
- ✅ Adopted by 3-5 marquee enterprise customers (reference-able)

### By EOY 2028:
- ✅ PyBlastRadius dominates blast radius + incident prediction market
- ✅ 200+ customers globally
- ✅ $10M+ ARR
- ✅ Series A funding enabling European expansion + R&D into ML prediction
- ✅ Become acquisition target for Datadog, Splunk, or CNCF projects

---

## 9. Open Questions for First Sprint Planning

1. **Discovery Priority**: Start with K8s + Airflow, or broader?
2. **OTel Integration**: Should we build OTel receiver, or expect OTLP endpoint?
3. **Business Impact**: Pre-built templates (fintech, SaaS, retail) or let customers build?
4. **Simulation Complexity**: Linear propagation (simple) or graph neural network (advanced)?
5. **Commercial License**: Open-source (Apache 2.0) from day 1, or delay monetization?

---

## Appendix: Research Sources

- Cost of downtime: OpenObserve, Splunk, Cisco reports
- MTTR benchmarks: Rootly, OpenObserve
- Cascading failure case studies: Google SRE Book, AWS incident reports
- Data pipeline challenges: Hightouch, Airbyte research
- OTel maturity: Official OTel roadmap (2026)
- ML for failure prediction: Springer, ArXiv recent papers
- Market sizing: Gartner observability market report
