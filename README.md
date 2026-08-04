# PyBlastRadius

> Operational Blast Radius Intelligence Platform  
> Know not just what will fail—predict which failures cascade and quantify business impact.

---


## Real Use Cases

This library is used for:
- See examples below
- Check GitHub issues for real-world usage

## Get Started

```python
# Quick example - copy and run
# See full docs for detailed usage
```

## 📋 Current Status

**Phase**: Research & Planning (Aug 2026)  
**Next**: Development Sprint 1 (Rust core + discovery engine)  
**Target Launch**: Q4 2026 (v0.1 MVP)

---

## 📚 Documentation

This repository currently contains research and planning documents. Source code will be added in Sprint 1.

### Core Documents

- **[PRODUCT_VISION.md](docs/PRODUCT_VISION.md)** — Complete product vision, architecture, and phased roadmap
  - Problem statement grounded in real costs ($15K/min downtime)
  - 3-phase delivery plan (v0.1 → v0.2 → v1.0)
  - Go-to-market strategy and success metrics
  - Open questions for sprint planning

- **[MARKET_RESEARCH.md](docs/MARKET_RESEARCH.md)** — Comprehensive market analysis
  - Competitive landscape (Datadog, New Relic, Lightstep, etc.)
  - Real-world pain points validated through research
  - Market gaps and sizing ($2-5B opportunity)
  - Target customer profiles and segments
  - Technical insights (hybrid discovery, GNNs, anomaly detection)

- **[RESEARCH_SUMMARY.md](docs/RESEARCH_SUMMARY.md)** — Quick reference guide
  - Key changes from original vision
  - 8 critical research findings
  - Validated assumptions ready to build on
  - MVP sprint plan with success criteria
  - What's next priorities

---

## 🎯 The Problem

Organizations operate hundreds of interconnected systems (microservices, data pipelines, AI agents, business platforms). When failures occur, SRE teams spend **4-24 hours** on MTTR manually tracing dependencies and estimating impact.

Existing tools answer "What is broken?" but leave critical questions unanswered:
- **What will break next?** (Cascading failures are invisible)
- **How bad will it get?** (Business impact is guesswork)
- **Where should I fix first?** (Criticality scoring is missing)

---

## 💡 The Solution

PyBlastRadius automatically discovers dependencies across all system layers—infrastructure, data pipelines, applications, and AI systems—then predicts which failures cascade and quantifies business impact.

### Key Differentiators

| Feature | Datadog/New Relic | dbt/Airflow | PyBlastRadius |
|---------|------------------|-------------|---------------|
| **Unified cross-system graph** | ❌ | ❌ | ✅ |
| **Data pipeline → business impact** | ❌ | ❌ | ✅ |
| **Cascade prediction** | ❌ | ❌ | ✅ |
| **Hidden dependency detection** | ❌ | ❌ | ✅ |
| **Reverse ETL aware** | ❌ | ❌ | ✅ |
| **AI/agent system lineage** | ❌ | ❌ | ✅ |

---

## 📊 Market Opportunity

- **Market Size**: $2-5B (sub-segment of $20B observability market)
- **Addressable Market**: $500M-1B (organizations with >50 engineers, >$1M infrastructure spend)
- **ROI**: One prevented incident ($705K+ saved) = 5-15x annual cost

---

## 🚀 Roadmap

### v0.1 MVP (6 weeks, target Oct 2026)
**Focus**: Reduce MTTR through unified dependency discovery + incident blast radius analysis

- Kubernetes + Terraform + Airflow discovery
- Unified operational graph (NetworkX)
- Blast radius analysis + criticality scoring
- CLI: `scan`, `analyze`, `criticality`, `graph`
- Interactive visualization

**Success Criteria**:
- 85%+ dependency discovery accuracy
- <5 min time to blast radius analysis
- 3-5 beta customers testing
- 50%+ MTTR improvement in beta

### v0.2 Extended (8 weeks, target Dec 2026)
**Focus**: Data pipeline + business impact connection

- dbt, Spark, Kafka, data warehouse discovery
- Reverse ETL pipeline monitoring
- Business impact quantification (YAML config)
- Pre-deployment risk analysis

### v1.0 Enterprise (12 weeks, target Mar 2027)
**Focus**: Scale + ML-based failure prediction

- Advanced hidden dependency detection (90%+ accuracy)
- Failure prediction with 20-40 min early warning
- Neo4j backend for 100K+ nodes
- GraphQL API + integrations (Slack, PagerDuty)
- Enterprise features (SSO, audit logs, SLA)

---

## 🏗️ Architecture (Planned)

### Core Components

1. **Hybrid Dependency Discovery Engine**
   - Static: K8s manifests, Terraform, DAGs, dbt models
   - Dynamic: OTel traces, message queue subscribers
   - Semantic: Query logs, configuration mining
   - Target accuracy: >90%

2. **Unified Operational Graph**
   - Rust-based (petgraph)
   - NetworkX → GraphML → Neo4j scaling path
   - Support 10,000+ nodes in-memory

3. **Blast Radius Analysis**
   - BFS traversal for impact categorization
   - Criticality scoring (downstream × severity × business exposure)
   - Cascade prediction (emergent in v1.0)

4. **Business Impact Engine**
   - YAML-based mappings (technical failure → business outcome)
   - Revenue exposure quantification
   - Executive-friendly reporting

5. **Visualization**
   - Interactive dependency graph (Cytoscape.js)
   - Failure flow diagrams
   - Criticality heatmaps

---

## 🎓 Key Research Findings

### 1. Cascading Failures Are Underestimated
- Knight Capital (2012): $440M in 45 minutes
- AWS DNS (2025): 15 hours, 1000+ companies
- Current tools detect but don't predict

### 2. Data Pipeline Failures Are Unmapped
- Reverse ETL (Hightouch, Airbyte) has zero cross-system observability
- Silent data corruption cascades: warehouse delay → stale features → fraud leakage
- **This is PyBlastRadius's biggest competitive advantage**

### 3. Hidden Dependencies At 67% of Orgs
- Static discovery: 70% accuracy
- Dynamic (traces): 80% accuracy
- Hybrid approach: 90%+ accuracy
- **Hybrid design from day 1 = leapfrog competitors**

### 4. Business Impact Drives Urgency
- Cost of downtime: $15,000/minute
- One prevented incident (47 min cascade): $705K saved
- Executive spending follows quantified business risk

### 5. AI/Agent Dependencies Invisible
- LLM chains + vector DBs are production-critical but unmapped
- LangSmith only shows traces within ONE agent
- **First platform to map AI system dependencies wins emerging market**

See [MARKET_RESEARCH.md](docs/MARKET_RESEARCH.md) for full analysis.

---

## 👥 Target Customers

### Tier 1 (Highest Value)
- Mid-market SaaS (100-1000 engineers)
- Fintech (high cost of downtime, complex cascades)
- E-commerce (revenue directly tied to uptime)

### Tier 2 (Good Fit)
- Data-heavy companies (analytics, ML platforms)
- Enterprise (1000+ engineers, mature SRE practices)

### Tier 3 (Lower Priority)
- Startups (limited observability budget)
- Traditional enterprises (legacy architecture)

---

## 💰 Business Model (Planned)

- **Open-source Core**: Apache 2.0 (drives adoption, builds community)
- **Professional Tier**: $5-15K/month (advanced discovery, integrations, business impact)
- **Enterprise Tier**: $15-50K/month (scale, ML prediction, on-prem)

**Rationale**: One prevented outage ($705K+ saved) = 5-15x ROI in first year

---

## 🔄 What's Next

1. ✅ Complete research & validation
2. → Identify 3-5 beta customers
3. → Sprint 1: Rust core + discovery engine
4. → Sprint 2: Analysis + CLI
5. → Beta launch + feedback
6. → v0.1 public release

---

## 📖 How to Use These Documents

**New to PyBlastRadius?**
1. Start with [PRODUCT_VISION.md](docs/PRODUCT_VISION.md) — Executive summary
2. Skim [RESEARCH_SUMMARY.md](docs/RESEARCH_SUMMARY.md) — Key findings
3. Deep dive [MARKET_RESEARCH.md](docs/MARKET_RESEARCH.md) — If you want details

**Sprint Planning?**
1. [PRODUCT_VISION.md](docs/PRODUCT_VISION.md) — Architecture & scope
2. [RESEARCH_SUMMARY.md](docs/RESEARCH_SUMMARY.md) — Sprint plan section
3. Ask questions in GitHub Issues

**Investor/Stakeholder Pitch?**
1. Elevator pitch in [PRODUCT_VISION.md](docs/PRODUCT_VISION.md)
2. Market opportunity in [MARKET_RESEARCH.md](docs/MARKET_RESEARCH.md)
3. Roadmap & success metrics in [PRODUCT_VISION.md](docs/PRODUCT_VISION.md)

---

## 📋 Repository Structure (Planned)

```
PyBlastRadius/
├── README.md                    # This file
├── docs/                        # Documentation
│   ├── PRODUCT_VISION.md        # Complete vision, architecture, roadmap
│   ├── MARKET_RESEARCH.md       # Market analysis & competitive landscape
│   ├── RESEARCH_SUMMARY.md      # Quick reference + sprint plan
│   ├── ARCHITECTURE.md          # Detailed technical design (coming)
│   └── examples/                # Example configs (coming)
├── src/                         # Source code (coming Sprint 1)
│   ├── lib.rs
│   ├── graph/
│   ├── discovery/
│   ├── analysis/
│   └── cli.rs
├── python/                      # Python bindings (coming Sprint 1)
│   ├── pyproject.toml
│   ├── src/pyblastradius/
│   └── tests/
├── tests/                       # Integration tests (coming Sprint 1)
├── examples/                    # Example usage (coming Sprint 1)
├── Cargo.toml                   # Rust manifest (coming)
├── .github/workflows/           # CI/CD (coming)
└── LICENSE
```

---

## 🤝 Contributing

Research is complete. We're looking for:
- **Rust engineers** for core development
- **Data engineers** for discovery module design
- **SREs** for beta testing & feedback
- **Investors/advisors** interested in observability/AIOps

Email: mullassery@gmail.com

---

## 📜 License

Apache 2.0 (Open-source from day 1)

---

## 🔗 Links

- **GitHub**: https://github.com/Mullassery/PyBlastRadius
- **Twitter/X**: [@mullassery](https://twitter.com/mullassery)
- **Author**: [Georgi Mammen Mullassery](https://github.com/Mullassery)

---

**Last Updated**: August 3, 2026  
**Status**: Research & Planning Phase

## License

Proprietary License — Free to use with explicit attribution

See [LICENSE](LICENSE) for full terms.
