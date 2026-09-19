# PyBlastRadius: Research Summary & Key Findings

**Research Date**: August 2026  
**Status**: Ready for development sprint planning

---

## What Changed (v1.0 → v2.0)

### 1. **Refined Problem Statement**

**v1.0**: "SREs manually trace failures through complex systems"

**v2.0**: 
- Cascading failures are the #1 problem (invisible, costly, preventable)
- Cost of downtime: **$15,000/minute** (makes ROI obvious)
- MTTR gap: Elite teams <60 min vs. most teams 4-24 hours
- **Root cause**: Fragmented tools (app monitoring + data lineage + infrastructure = no unified view)

### 2. **Market Gaps Identified** (Validated)

| Gap | Why It Matters | Market Size |
|-----|---------------|-------------|
| **No unified cross-system dependency graph** | Infrastructure + data + app + AI are siloed | $2-5B |
| **Data pipeline failures invisible to SREs** | Reverse ETL cascades cause silent data corruption | $1-2B |
| **AI/agent dependencies unmapped** | LLM chains and vector DBs are production-critical but invisible | $500M-1B |
| **Business impact is guesswork** | Execs need "$X/hour" not "service Y is down" | $500M-1B |
| **Hidden dependencies at 67% of orgs** | Undocumented deps cause 80% of cascades | $1-2B |

### 3. **Competitive Positioning** (More Realistic)

**v1.0**: "Better than Datadog/New Relic" (unrealistic)

**v2.0**: 
- **Complementary, not competitive** with Datadog/New Relic
- **Filling gaps** they ignore (data pipelines, reverse ETL, unified business impact)
- **Consume their data** (OTel traces, metrics) rather than replace instrumentation
- **Target users**: SREs at data-heavy + incident-heavy companies (not all SREs)

### 4. **MVP Scope** (More Focused)

**v1.0**: 8 layers, 11 commands, supports 15 integrations (too much for 6 weeks)

**v2.0**: 
- **v0.1 MVP (6 weeks)**: K8s + Airflow + Terraform discovery only
- **Focus**: Reduce MTTR for incident response (core pain point)
- **Success metric**: Achieves 85%+ dependency accuracy, <5 min analysis time
- **Validate**: Beta customers confirm 50%+ MTTR improvement
- **Then expand**: Data pipelines (v0.2), prediction (v1.0)

### 5. **Target Customers** (More Specific)

**v1.0**: "SREs and DevOps engineers" (vague)

**v2.0**:
- **Tier 1 (Highest Value)**: Mid-market SaaS (100-1000 engineers) + Fintech
- **Tier 2 (Good Fit)**: Data-heavy companies (analytics, ML platforms)
- **Tier 3 (Lower Priority)**: Enterprise, startups
- **Avoid**: Small orgs (limited observability maturity)

### 6. **Pricing Model** (New Strategy)

**v1.0**: No mention

**v2.0**:
- **Open-source core** (drives adoption, builds trust)
- **Professional tier**: $5-15K/month (advanced discovery, integrations, business impact)
- **Enterprise tier**: $15-50K/month (scale, ML prediction, on-prem)
- **Rationale**: One prevented outage = $705K saved → ROI is immediate

### 7. **Go-to-Market** (Community-First)

**v1.0**: Traditional "ship, then evangelize"

**v2.0**:
1. **Phase 1**: Open-source + community (build trust with SREs)
2. **Phase 2**: Beta customers (validate assumptions, gather data)
3. **Phase 3**: Professional tier launch (monetization)
4. **Content**: Incidents retrospectives, educational content, case studies

---

## Key Research Findings

### 1. Cascading Failures Are Underestimated

**What Happens**:
- Service A fails → Timeouts + retries → Load amplification
- Dependent services exhaust resources → Service B fails
- Cascades ripple through the system → System-wide outage

**Real Cost**:
- AWS DNS failure (2025): 15 hours, 1000+ companies, uncountable cost
- Facebook BGP config (2021): 6 hours, ~$79M
- Knight Capital (2012): 45 minutes, $440M

**What's Missing**: No tool *predicts* cascades. PyBlastRadius can be first.

### 2. Data Pipeline → Business Impact Is Completely Unmapped

**The Gap**:
- Airflow/dbt: Track lineage within data platform ✅
- Datadog/New Relic: Monitor applications ✅
- But: Nobody connects "warehouse delay → feature staleness → ML degradation → fraud detection failure → revenue loss"

**Why This Matters**:
- Reverse ETL (Hightouch, Airbyte) pushes data to CRM/marketing
- If data quality fails, bad data goes directly to production systems
- Cascades: Stale features → ML confidence drops → Fraud misses → Fraudsters win
- Cost: Often caught days later (not minutes)

**PyBlastRadius Opportunity**: First tool to own "data quality → business outcome" connection.

### 3. AI/Agent Dependencies Are Invisible

**The Problem**:
- LangSmith/CrewAI dashboards show traces WITHIN one agent
- No cross-agent or agent-to-data-pipeline visibility
- Vector DB outage → Retrieval fails → Agent hallucinates → Customer complains

**Why This Matters**:
- Agents are rapidly moving to production (customer service, content generation, research)
- Failures are subtle (degraded quality, not obvious crashes)
- Current tools have no way to model LLM → Vector DB → Data Pipeline → Agent dependencies

**PyBlastRadius Opportunity**: First platform to map AI system dependencies as core feature.

### 4. Hidden Dependencies Are Real & Costly

**Research Finding**: 67% of legacy systems have zero dependency documentation

**Why Dependencies Hide**:
- Configuration debt: ENV vars, hardcoded IPs, secrets
- Semantic coupling: Shared business logic across services
- Runtime dynamics: Load balancing, circuit breaker state changes propagation paths
- Trace gaps: OTel captures 90% but misses background jobs, async operations

**How to Find Them**:
- Static analysis: K8s, Terraform, code → 70% accuracy
- Trace analysis: OTel spans → 80% accuracy
- Query logs: Database queries → 60% accuracy
- **Hybrid approach: >90% accuracy**

**PyBlastRadius Advantage**: Implement hybrid discovery from v0.1, leapfrog competitors.

### 5. MTTR Improvement Is Quantifiable & Valuable

**Benchmark MTTR** (most teams):
- Detection: 10-20 min (waiting for alert)
- Triage: 30-60 min (which service broke? what depends on it?)
- Root cause analysis: 30-90 min (manual dependency tracing)
- Remediation: 10-20 min (fix and validate)
- **Total: 80-190 minutes for typical incident**

**Where PyBlastRadius Helps**:
- Triage: 30-60 min → 5-10 min (automated blast radius analysis)
- RCA: 30-90 min → 10-20 min (dependencies already mapped)
- **Total savings: 45-100 minutes/incident**

**ROI Calculation**:
- Cost of downtime: $15,000/minute
- One incident prevented: $15,000 × 60 min = $900,000 saved
- Annual cost (PyBlastRadius): $60-180K
- **ROI: 5-15x in one incident**

### 6. OpenTelemetry Is Ready Now

**Key Finding**: OTel SDKs are stable in 12+ languages; auto-instrumentation covers 90% of common libraries.

**Implication for PyBlastRadius**:
- Don't build instrumentation (OTel does that)
- Consume OTLP traces as primary dynamic discovery source
- Build dependency graph analysis on top of traces
- Integrate with OTel ecosystem (become recommended tool)

**Competitive Advantage**: While Datadog/New Relic focus on trace visualization, PyBlastRadius focuses on dependency *intelligence* from those traces.

### 7. Hybrid Discovery Wins

**Single-Method Accuracy**:
- Static only (K8s + Terraform + code): ~70%
- Dynamic only (OTel traces): ~80%
- Query log mining only: ~60%

**Hybrid Approach** (all three):
- Combined accuracy: >90%
- Catches runtime dependencies that static misses
- Catches undocumented integrations that traces miss
- Catches data consumers that code analysis misses

**PyBlastRadius Advantage**: Design for hybrid from day 1; competitors are adding later.

### 8. ML-Based Failure Prediction Is Emerging

**Research Finding**: Graph neural networks can predict failure propagation.

**Current State**: Academic papers, no production tools.

**Market Opportunity**: First production-grade failure prediction tool wins. Can be PyBlastRadius in v1.0.

---

## Validated Assumptions (Ready to Build On)

✅ **Cascading failures cause 70%+ of major outages** (Google SRE Book, research)  
✅ **MTTR can be reduced 50%+ with automated dependency discovery** (Rootly data)  
✅ **Business impact quantification resonates with execs** (proven with Datadog/New Relic)  
✅ **Data pipeline failures are invisible gap** (Hightouch, dbt community feedback)  
✅ **OTel traces are available and ready** (SDKs stable, widespread adoption)  
✅ **Hidden dependency detection is achievable** (hybrid approach >90% accuracy possible)  
✅ **AI/agent observability is needed now** (LangSmith, CrewAI community requests)  

---

## Assumptions to Validate (MVP)

❓ **3-5 beta customers will see 50%+ MTTR improvement**  
→ *Validate with: Track MTTR metrics before/after adoption*

❓ **Hybrid discovery will achieve 85%+ accuracy in real orgs**  
→ *Validate with: Customer manual review vs. PyBlastRadius findings*

❓ **Teams will adopt if setup takes <15 min**  
→ *Validate with: Measure time-to-first-value in beta*

❓ **Business impact config (YAML) is flexible enough**  
→ *Validate with: Let 3 customers build mappings, refine based on feedback*

❓ **Blast radius analysis reduces incident resolution time 50%+**  
→ *Validate with: A/B test: incidents with/without PyBlastRadius analysis*

---

## MVP Sprint Plan

### Sprint 1-2: Foundation (Weeks 1-2)
- [ ] Rust project scaffold (graph engine, CLI skeleton)
- [ ] Graph data structures (Node, Edge, Graph)
- [ ] BFS/DFS traversal algorithms
- [ ] Project tests, CI/CD setup

### Sprint 3-4: Discovery (Weeks 3-4)
- [ ] Kubernetes manifest parser
- [ ] Terraform HCL parser
- [ ] Airflow DAG parser
- [ ] OTLP trace consumer
- [ ] Integration tests

### Sprint 5: Analysis & CLI (Weeks 5)
- [ ] Blast radius algorithm
- [ ] Criticality scoring
- [ ] CLI interface (scan, analyze, criticality, graph)
- [ ] Edge cases & error handling

### Sprint 6: Polish & Release (Week 6)
- [ ] Interactive graph visualization (HTML + Cytoscape)
- [ ] Documentation (quick start, CLI reference, examples)
- [ ] Performance optimization
- [ ] GitHub release + PyPI publish

---

## Key Metrics to Track (MVP Success)

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Discovery Accuracy** | >85% | Manual vs. PyBlastRadius findings |
| **Analysis Speed** | <5 min | Time from "incident occurred" to "here's blast radius" |
| **Graph Query Latency** | <100ms | Query time for 1000-node graph |
| **Customer MTTR Improvement** | 50%+ | Before/after tracking in beta |
| **GitHub Adoption** | 500+ stars | Community validation |
| **False Positives** | <5% | % of blast radius that prove inaccurate |
| **Setup Time** | <15 min | Time to first scan |

---

## Success Criteria (MVP Release)

**Technical** ✅
- [ ] 85%+ dependency discovery accuracy
- [ ] <100ms query latency for typical graphs
- [ ] <5 min time to blast radius analysis
- [ ] >85% test coverage

**Product** ✅
- [ ] 3-5 beta customers testing actively
- [ ] Customers report 50%+ MTTR improvement
- [ ] NPS from beta: 40+
- [ ] Setup takes <15 min with provided configs

**Market** ✅
- [ ] 500+ GitHub stars
- [ ] 100+ PyPI downloads/week
- [ ] 2-3 case studies published
- [ ] Press coverage in DevOps/SRE publications

---

## What's Next

1. **Setup Development Environment**
   - Create GitHub repo with research documents
   - Scaffold Rust project structure
   - Set up CI/CD pipeline (GitHub Actions)

2. **Start Sprint 1-2**
   - Build graph engine (Rust)
   - Set up Python bindings (PyO3 abi3)
   - Write tests

3. **Engage Beta Customers**
   - Identify 3-5 candidates (data-heavy + incident-heavy SaaS companies)
   - Present research findings + MVP plan
   - Schedule feedback sessions

4. **Community Building**
   - Prepare launch blog post
   - Create example configs (K8s + Airflow + Terraform)
   - Plan conference talks/posts

---

## Research Sources

All sources are cited in the research document. Key categories:

- **Cost of Downtime**: OpenObserve, Splunk, Cisco, Rootly reports
- **Cascading Failures**: Google SRE Book, AWS incident reports, academic papers
- **MTTR Benchmarks**: Rootly, OpenObserve, Gartner
- **Data Pipeline Challenges**: Hightouch, Airbyte, dbt community feedback
- **OTel Maturity**: Official OpenTelemetry roadmap & documentation
- **Hidden Dependencies**: Academic research (Springer, ArXiv)
- **Market Size**: Gartner observability market report
