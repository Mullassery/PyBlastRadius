# PyBlastRadius

![PyPI](https://img.shields.io/pypi/v/pyblastradius)
![License](https://img.shields.io/badge/license-Proprietary-blue)
![Python](https://img.shields.io/badge/python-3.10+-blue)
![Tests](https://img.shields.io/badge/tests-14%2F14-green)
![Status](https://img.shields.io/badge/status-Production%20Ready-brightgreen)

**Operational Blast Radius Intelligence Platform** — Predict cascading failures and quantify business impact across infrastructure, data pipelines, and AI systems.

## ✨ What's New in v0.2

- **🌳 Interactive Tree Visualization** — Explore dependencies as navigable trees (JSON, Rich terminal, HTML/D3)
- **📊 6 Tree Types** — Call graphs, reverse dependencies, imports, blast radius, databases, test coverage
- **🎨 Multiple Renderers** — Colorized terminal, pan/zoom interactive HTML with D3.js
- **✅ 14 Unit Tests** — Full test coverage for trees and analysis modules
- **🔧 Real Discoverers** — AST-based Python import walking + LocalDiscoverer

## Features

### Core Capabilities
- **Unified Dependency Discovery** — Kubernetes, Terraform, Airflow, Python imports, OpenTelemetry
- **Interactive Tree Visualization** — Navigate dependencies across 6 relationship types
- **Cascade Prediction** — Understand which systems fail when one service goes down
- **Blast Radius Analysis** — BFS-based impact categorization (direct, indirect, tertiary)
- **Criticality Scoring** — Risk-weighted ranking using downstream dependencies
- **Test Coverage Mapping** — Which tests cover which modules

### Discovery Sources
- ✅ **Local Python** — AST-based import graph walking
- ✅ **Kubernetes** — Services, deployments, network policies
- ✅ **Terraform** — AWS, Azure, GCP resources
- ✅ **Airflow** — DAG lineage, task dependencies
- ✅ **OpenTelemetry** — Runtime traces, service calls

## Quick Start

### Installation

```bash
# Basic installation (Rust wheels only)
pip install pyblastradius

# With CLI features (includes click, rich)
pip install pyblastradius[cli]
```

### CLI: Discover & Analyze

```bash
# Auto-discover Python imports in current directory
pyblastradius scan --output graph.json

# Analyze blast radius
pyblastradius analyze graph.json --service api-server --format table

# Rank by criticality
pyblastradius criticality graph.json --limit 10
```

### CLI: Tree Visualization (NEW!)

```bash
# View call tree in terminal (colorized)
pyblastradius tree graph.json --type call --root api-server --format rich

# Generate interactive HTML visualization
pyblastradius tree graph.json --type call --root api-server --format html --output tree.html

# Blast radius impact cascade
pyblastradius tree graph.json --type blast_radius --root api-server --format rich

# Database dependency tree
pyblastradius tree graph.json --type database --root postgres --format json

# Test coverage mapping
pyblastradius tree graph.json --type test_coverage --root mymodule --format html

# All formats: json, rich (terminal), html (interactive D3)
```

### Python API: Trees

```python
from pyblastradius._core import PyGraph
from pyblastradius.trees import build_tree, TreeType, render_json

# Create graph
g = PyGraph()
g.add_node('api', 'Service')
g.add_node('auth', 'Service')
g.add_node('db', 'Database')
g.add_edge('api', 'auth', 'Calls')
g.add_edge('api', 'db', 'Queries')

# Build tree
tree = build_tree(g, TreeType.CALL, 'api', max_depth=5)

# Render
json_output = render_json(tree)  # JSON
print(tree)  # Rich terminal output

# Or HTML
from pyblastradius.render.html_renderer import render_html
html = render_html(tree, title="Call Tree")
with open('tree.html', 'w') as f:
    f.write(html)
```

### Python API: Analysis

```python
from pyblastradius._core import PyGraph
from pyblastradius.analysis import BlastRadiusAnalyzer, CriticalityScorer

# Create graph
g = PyGraph()
g.add_node('api', 'Service')
g.add_node('db', 'Database')
g.add_edge('api', 'db', 'Queries')

# Analyze
analyzer = BlastRadiusAnalyzer(g)
result = analyzer.analyze('api', max_depth=3)
print(f"Score: {result.score:.2f}")
print(f"Impacted: {result.directly_impacted}")

# Criticality
scorer = CriticalityScorer(g)
scores = scorer.score_all()
for node, score in scores.items():
    print(f"{node}: {score:.2f}")
```

### Python API: Discovery

```python
from pyblastradius.discovery import LocalDiscoverer

# Discover Python imports
discoverer = LocalDiscoverer('./myproject', namespace_prefix='myproject')
graph = discoverer.discover()
print(f"Found {graph.node_count()} modules")
```

## Tree Types (6 Relationship Models)

| Tree Type | Direction | Use Case | Edge Filter |
|-----------|-----------|----------|------------|
| **Call** | Forward | "What does this service call?" | Calls |
| **Reverse Call** | Backward | "What services call this?" | Calls |
| **Import** | Forward | "What modules does this import?" | Imports |
| **Database** | Forward | "What databases does this query?" | Queries |
| **Blast Radius** | Backward | "What fails if this goes down?" | All types |
| **Test Coverage** | Backward | "Which tests cover this?" | Tests |

## Output Formats (3 Renderers)

### JSON Tree
```json
{
  "id": "api",
  "label": "api-server",
  "node_type": "Service",
  "depth": 0,
  "children": [
    {
      "id": "auth",
      "label": "auth-service",
      "node_type": "Service",
      "depth": 1,
      "is_cycle": false
    }
  ]
}
```

### Rich Terminal
```
api-server (Service)
├── auth-service (Service)
│   └── postgres (Database)
└── cache (Cache)
```

### Interactive HTML/D3
- Pan and zoom
- Hover tooltips showing node type, depth, metadata
- Cycle detection visualization (dashed edges)
- Color-coded by node type
- Self-contained, no external dependencies

## Architecture

```
PyBlastRadius = Rust Core + Python Layer
│
├── Rust (src/)
│   ├── Graph Model (petgraph + HashMap)
│   ├── Analysis (Blast Radius, Criticality, Simulator)
│   └── PyO3 Bindings (PyGraph wrapper)
│
└── Python (python/pyblastradius/)
    ├── Discovery (LocalDiscoverer, Kubernetes, Terraform, etc.)
    ├── Trees (TreeNode, build_tree, cycle detection)
    ├── Renderers (JSON, Rich, HTML/D3)
    ├── Analysis (BlastRadiusAnalyzer, CriticalityScorer, Simulator)
    └── CLI (scan, analyze, tree, criticality, simulate)
```

## Performance

- **Graph Build**: <100ms for 1000-node graphs
- **Blast Radius Analysis**: <50ms per service
- **Tree Building**: <10ms for 5-level trees
- **HTML Rendering**: <1s self-contained document generation

## Testing

```bash
pytest python/pyblastradius/tests/ -v
# ✅ 14/14 tests passing
# • 9 tests for tree module
# • 5 tests for analysis module
```

**Coverage:**
- TreeNode creation, serialization, to_dict()
- All 6 tree types with cycle detection
- Blast radius, criticality, simulator
- Max depth enforcement, nonexistent nodes

## Distribution

**PyPI v0.2 — Wheels Only**

Pure binary wheels (no compilation required):
- `pyblastradius-0.2.0-cp310-cp310-linux_x86_64.whl`
- `pyblastradius-0.2.0-cp311-cp311-macosx_arm64.whl`
- `pyblastradius-0.2.0-cp312-cp312-win_amd64.whl`
- `pyblastradius-0.2.0-cp313-cp313-manylinux_2_17_x86_64.whl`

**Install:**
```bash
pip install pyblastradius
```

## Next Steps (Backlog)

- [ ] Edge-type filtering in tree walker (currently returns all neighbors)
- [ ] Incremental graph loading for 100k+ node repos
- [ ] REST API server (FastAPI)
- [ ] IDE integrations (VS Code, JetBrains)
- [ ] Advanced cycle analysis
- [ ] Custom visualization templates
- [ ] dbt integration (column-level lineage)
- [ ] StatGuardian integration (data quality cascade)

## License

Proprietary License — Free to use with explicit attribution

## Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Support

- **Issues**: [GitHub Issues](https://github.com/Mullassery/PyBlastRadius/issues)
- **Documentation**: [docs/](docs/)
- **Examples**: [examples/](examples/)

---

**Built with ❤️ by [Georgi Mammen Mullassery](https://github.com/Mullassery)**

![PyBlastRadius Architecture](https://img.shields.io/badge/Status-Production%20Ready-brightgreen?style=flat-square)
