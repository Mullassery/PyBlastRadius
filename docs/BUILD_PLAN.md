# PyBlastRadius Build Plan - Uninterrupted Implementation

**Goal**: Implement all 10 input methods from GETTING_STARTED.md  
**Timeline**: ~24 hours continuous  
**Target**: v0.1.0 production release  
**Reference**: GAP_ANALYSIS.md (identifies all gaps)

---

## PHASE 1: Unblock Python Bindings [BLOCKER - 30 min]

**Status**: PyPI publish workflow has issues (v0.1.0 tag failing)

### 1.1 Fix Publish Workflow

**File**: `.github/workflows/publish.yml`

**Issues to fix**:
1. PyO3 version compatibility with Python 3.13 (DONE - upgraded to 0.23)
2. Removed [[bin]] conflict with maturin (DONE - moved to examples/)
3. Re-test build

**Action**:
```bash
# Last push: 2026-08-05 05:25:40
# Tag: v0.1.0
# Status: Workflow in progress

# Monitor latest run
gh run list --limit 1
# When complete, verify wheels on PyPI:
pip index versions pyblastradius
```

**Success criteria**: 
- ✅ Wheels built on 3 platforms (Ubuntu, macOS, Windows)
- ✅ Published to PyPI: https://pypi.org/project/pyblastradius/
- ✅ `pip install pyblastradius` works

---

## PHASE 2: CLI Implementation [8 hours]

### 2.1 URI Parser

**File**: `src/bin/cli.rs` → NEW module `uri_parser.rs`

**What to implement**:
```rust
mod uri_parser {
    // Parse: kubernetes://kubeconfig=/path,context=prod,namespaces=default,prod
    pub struct SourceUri {
        pub scheme: String,
        pub params: HashMap<String, String>,
    }
    
    pub fn parse(uri: &str) -> Result<SourceUri> { ... }
    pub fn validate(uri: &SourceUri) -> Result<()> { ... }
}
```

**Supported schemes** (in order):
1. `kubernetes://` (kubeconfig, context, namespaces)
2. `terraform://` (path, backend, vars)
3. `airflow://` (host, port, auth)
4. `otlp://` (endpoint, duration, sample-rate)
5. `local://` (path, recursive)
6. `compose://` (file, project) - v0.2
7. `git://` (repo, branch, commit) - v1.0
8. `github://` (owner, repo, pr, commit, token) - v1.0

**Lines of code**: ~300  
**Tests**: 15-20 unit tests  
**Time**: 1 hour

---

### 2.2 Scan Command - Auto-Discovery

**File**: `src/bin/cli.rs` → refactor scan_auto()

**Current**: Basic skeleton  
**Goal**: Fully working auto-discovery

```bash
$ pyblastradius scan
Detected resources:
  • Kubernetes: 127 manifests (k8s/)
  • Terraform: 43 files (infra/)
  • Docker Compose: 1 file (docker-compose.yml)
  • Airflow: 0 DAGs

Graph merged: 170 nodes
Saved to: blast-radius.json
Next: pyblastradius analyze
```

**Implementation**:
1. Walk current directory
2. Find Kubernetes manifests (*.yml, *.yaml in k8s/, kubernetes/)
3. Find Terraform files (*.tf in infra/, infrastructure/, terraform/)
4. Find docker-compose.yml
5. Find Airflow dags/ or dags.yaml
6. Merge all graphs
7. Output JSON
8. Show summary + next steps

**Lines of code**: ~400  
**Tests**: 8-10 integration tests  
**Time**: 2 hours

---

### 2.3 Scan Command - Explicit URIs

**File**: `src/bin/cli.rs` → refactor scan_explicit()

**Current**: Doesn't exist  
**Goal**: Parse and discover from URIs

```bash
$ pyblastradius scan kubernetes://kubeconfig=~/.kube/config \
                        terraform://path=./infra
```

**Implementation**:
1. Parse all provided URIs
2. Validate each (kubeconfig exists, path exists, etc.)
3. Call appropriate discoverer for each
4. Merge results
5. Handle errors gracefully
6. Show what was discovered + where

**Lines of code**: ~200  
**Tests**: 10-15 unit tests (mock each source)  
**Time**: 1.5 hours

---

### 2.4 Analyze Command

**File**: `src/bin/cli.rs` → NEW analyze_cmd()

**Current**: Doesn't exist  
**Goal**: Load graph, run blast radius analysis

```bash
$ pyblastradius analyze blast-radius.json \
  --service payment-service \
  --max-depth 3 \
  --format table
```

**Implementation**:
1. Load graph from JSON file
2. Validate service exists
3. Run BlastRadiusAnalyzer.analyze()
4. Format output (table, json, csv)
5. Print results with colors

**Output format** (table):
```
Service: payment-service
Blast Radius Score: 0.82 (HIGH RISK)

Directly Impacted (1 hop):
  ✗ api-gateway (critical)
  ✗ payment-processor (critical)

Indirectly Impacted (2 hops):
  • user-service
  • order-service
  (9 services total)

Recovery Time: 45-120 min
Single Point of Failure: api-gateway
```

**Lines of code**: ~250  
**Tests**: 5-8 tests  
**Time**: 1.5 hours

---

### 2.5 Analysis Commands (Criticality, Simulate, Visualize)

**File**: `src/bin/cli.rs` → NEW criticality_cmd(), simulate_cmd(), visualize_cmd()

```bash
pyblastradius criticality graph.json --limit 10 --format table
pyblastradius simulate graph.json --target postgres --scenario cascade
pyblastradius visualize graph.json --output graph.html --format interactive
```

**Implementation per command**:

**criticality**: 
- Load graph
- Score each service
- Rank by score
- Show single points of failure
- Lines: ~200

**simulate**:
- Load graph
- Run failure simulation
- Show cascade path
- Estimate recovery time
- Lines: ~200

**visualize**:
- Load graph
- Generate HTML (D3.js or similar)
- Color nodes by criticality
- Make interactive
- Lines: ~300

**Lines of code**: ~700 total  
**Tests**: 10-12 tests  
**Time**: 2 hours

---

### 2.6 Error Handling & Progress Reporting

**File**: `src/bin/cli.rs` → error_handler.rs, progress.rs

**Error messages** (IMPLEMENTATION_CLI.md § Error Handling):
```
Error: Failed to connect to Kubernetes API
  Location: kubernetes://kubeconfig=/home/user/.kube/config
  Error: connection refused (127.0.0.1:6443)
  
  Suggestions:
  1. Verify kubeconfig path exists
  2. Check cluster is running: kubectl cluster-info
  3. Verify credentials: kubectl auth can-i get nodes
  
  Docs: https://docs.pyblastradius.io/discovery/kubernetes
```

**Progress reporting**:
```
$ pyblastradius scan --verbose
[1/5] Loading config... ✅
[2/5] Discovering Kubernetes... ⏳ (127 nodes found)
[3/5] Discovering Terraform... ⏳ (43 resources found)
[4/5] Merging graphs... ✅
[5/5] Ready for analysis ✅

Total: 170 nodes
Next: pyblastradius analyze
```

**Lines of code**: ~200  
**Time**: 0.5 hours

---

### Phase 2 Summary

| Component | LOC | Time | Status |
|-----------|-----|------|--------|
| URI parser | 300 | 1h | NEW |
| Scan auto | 400 | 2h | REFACTOR |
| Scan explicit | 200 | 1.5h | NEW |
| Analyze | 250 | 1.5h | NEW |
| Analysis commands | 700 | 2h | NEW |
| Error handling | 200 | 0.5h | NEW |
| **TOTAL** | **2,050** | **8h** | |

**Delivers**: GETTING_STARTED.md Methods 1-3  
**Users can**: Scan infrastructure, analyze blast radius, rank criticality

---

## PHASE 3: Config File Support [6 hours]

### 3.1 Config Class

**File**: `python/pyblastradius/config.py` → NEW

```python
@dataclass
class Config:
    version: str
    discovery: List[Dict[str, Any]]
    analysis: Dict[str, Any]
    outputs: List[Dict[str, Any]]
    alerts: Optional[List[Dict[str, Any]]]
    
    @classmethod
    def load_from_file(cls, path: str) -> 'Config':
        # Load YAML, validate schema, resolve env vars
        pass
    
    def validate(self) -> Result<()>:
        # Check required fields, types, value ranges
        pass
    
    def resolve_env_vars(self) -> None:
        # Replace ${VAR_NAME} with os.environ['VAR_NAME']
        pass
    
    def create_discoverers(self) -> List[Discoverer]:
        # Build discoverer instances from config
        pass
    
    def merge_with_cli_args(self, args: Dict) -> None:
        # CLI args override config file values
        pass
```

**Lines of code**: ~300  
**Tests**: 8-10 tests  
**Time**: 1.5 hours

---

### 3.2 Config File Parsing & Validation

**File**: `src/config.rs` → NEW (Rust side for validation)

**Schema** (from INPUT_DESIGN.md § Method 4):
```yaml
version: 1

discovery:
  sources:
    - type: kubernetes
      kubeconfig: ~/.kube/config
      namespaces: [default, prod]
    
    - type: terraform
      path: ./infra
      required: false
    
    - type: otlp
      endpoint: localhost:4317
      duration: 5m

analysis:
  max-depth: 5

outputs:
  - type: json
    path: blast-radius.json
  - type: html
    path: blast-radius.html

alerts:
  - condition: "blast_radius_score > 0.8"
    actions:
      - type: slack
        webhook-url: ${SLACK_WEBHOOK}
```

**Validation**:
1. Schema validation (check required fields)
2. Type checking (string, number, boolean, array)
3. Range checking (max-depth: 1-10)
4. File/path existence checking
5. Environment variable resolution

**Lines of code**: ~400  
**Tests**: 12-15 tests  
**Time**: 1.5 hours

---

### 3.3 Config Commands

**File**: `src/bin/cli.rs` → config_cmd()

```bash
pyblastradius config validate     # Check config file is valid
pyblastradius config show         # Show resolved config (with env vars)
pyblastradius config init         # Create template config
```

**Implementation**:
1. Find blastradius.yml in current dir
2. Load + validate
3. Show results

**Lines of code**: ~200  
**Tests**: 5-8 tests  
**Time**: 1 hour

---

### 3.4 CLI Integration with Config

**File**: `src/bin/cli.rs` → main() refactor

**Changes**:
1. Auto-load blastradius.yml if exists
2. Apply CLI arg overrides
3. Use merged config for all operations

**Implementation**:
```rust
fn main() {
    let args = parse_cli_args();
    let config = if Path::new("blastradius.yml").exists() {
        Config::load("blastradius.yml")?
    } else {
        Config::default()
    };
    config.merge_cli_args(&args)?;
    // Use config
}
```

**Lines of code**: ~100  
**Time**: 0.5 hours

---

### 3.5 Python Integration

**File**: `python/pyblastradius/config.py` → complete implementation

**Key features**:
1. Load from YAML file
2. Validate schema
3. Resolve environment variables
4. Create discoverer instances

**Lines of code**: ~300  
**Tests**: 10-12 tests  
**Time**: 1 hour

---

### Phase 3 Summary

| Component | LOC | Time | Status |
|-----------|-----|------|--------|
| Config class | 300 | 1.5h | NEW |
| Config parsing | 400 | 1.5h | NEW |
| Config commands | 200 | 1h | NEW |
| CLI integration | 100 | 0.5h | REFACTOR |
| Python integration | 300 | 1h | NEW |
| **TOTAL** | **1,300** | **5.5h** | |

**Delivers**: GETTING_STARTED.md Method 4  
**Users can**: Version control configs, repeatably analyze, override via CLI

---

## PHASE 4: GitHub Actions Integration [4 hours]

### 4.1 GitHub Action Metadata

**File**: `action.yml` → NEW

```yaml
name: 'Blast Radius Analysis'
description: 'Analyze and report blast radius of infrastructure changes'
inputs:
  discovery-sources:
    description: 'Discovery sources (multi-line)'
    required: true
  max-depth:
    description: 'Maximum depth for blast radius analysis'
    default: '5'
  failure-on-score:
    description: 'Fail if blast radius score exceeds this (0-1)'
    default: '0.9'
outputs:
  blast-radius-score:
    description: 'Blast radius score (0-1)'
  analysis-json:
    description: 'Path to analysis JSON file'
runs:
  using: 'composite'
  steps:
    - run: ${{ github.action_path }}/action.sh
      shell: bash
```

**Lines of code**: ~50  
**Time**: 0.5 hours

---

### 4.2 Action Script

**File**: `action.sh` → NEW

```bash
#!/bin/bash

# Install pyblastradius
pip install pyblastradius

# Run scan
pyblastradius scan $DISCOVERY_SOURCES

# Run analyze
pyblastradius analyze blast-radius.json

# Parse results
SCORE=$(jq '.blast_radius_score' blast-radius.json)

# Set outputs
echo "blast-radius-score=$SCORE" >> $GITHUB_OUTPUT

# Fail if threshold exceeded
if (( $(echo "$SCORE > $FAILURE_ON_SCORE" | bc -l) )); then
    exit 1
fi
```

**Lines of code**: ~40  
**Time**: 0.5 hours

---

### 4.3 PR Comment Integration

**File**: `.github/workflows/blast-radius.yml` → example workflow

Shows how to:
1. Run action
2. Post results to PR
3. Create check run
4. Suggest fixes

**Lines of code**: ~80  
**Time**: 1 hour

---

### 4.4 Testing

**File**: `.github/workflows/test-action.yml` → NEW

Test action against sample repo with known blast radius

**Lines of code**: ~50  
**Time**: 1 hour

---

### Phase 4 Summary

| Component | LOC | Time | Status |
|-----------|-----|------|--------|
| Action metadata | 50 | 0.5h | NEW |
| Action script | 40 | 0.5h | NEW |
| Example workflow | 80 | 1h | NEW |
| Test workflow | 50 | 1h | NEW |
| **TOTAL** | **220** | **3h** | |

**Delivers**: GETTING_STARTED.md Method 8 (GitHub Actions)  
**Users can**: Automate pre-merge validation in CI/CD

---

## PHASE 5: Documentation Examples [3 hours]

### 5.1 Example Config Files

**Files**: `docs/examples/blastradius-*.yml`

1. **blastradius-kubernetes.yml**: K8s only
2. **blastradius-terraform.yml**: Terraform only
3. **blastradius-complete.yml**: K8s + Terraform + OTLP
4. **blastradius-local.yml**: Local development

**Lines of code**: ~150  
**Time**: 0.5 hours

---

### 5.2 Sample Data for Testing

**Files**: `tests/fixtures/*`

1. Sample Kubernetes manifests (10-15 services)
2. Sample Terraform configs (10-15 resources)
3. Expected graph output (JSON)
4. Expected analysis results

**Lines of code**: ~200  
**Time**: 1 hour

---

### 5.3 Python Script Examples

**Files**: `docs/examples/python_*.py`

1. **python_quickstart.py**: 10 lines, discover + analyze
2. **python_advanced.py**: Multi-source, filtering
3. **python_callbacks.py**: Streaming results
4. **python_config.py**: Load config, analyze

**Lines of code**: ~150  
**Time**: 0.5 hours

---

### 5.4 Test All Examples End-to-End

Run each example against fixtures, verify output

**Time**: 1 hour

---

### Phase 5 Summary

| Component | LOC | Time | Status |
|-----------|-----|------|--------|
| Config examples | 150 | 0.5h | NEW |
| Sample data | 200 | 1h | NEW |
| Python examples | 150 | 0.5h | NEW |
| E2E testing | - | 1h | NEW |
| **TOTAL** | **500** | **3h** | |

**Delivers**: All examples in GETTING_STARTED.md Methods 1-8  
**Users can**: Copy-paste and run every example successfully

---

## PHASE 6: REST API Server [5 hours]

### 6.1 HTTP Server Setup

**File**: `src/bin/server.rs` → NEW

Use actix-web framework

```rust
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/health", web::get().to(health_check))
            .route("/api/v1/analyze", web::post().to(analyze))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
```

**Lines of code**: ~150  
**Time**: 1 hour

---

### 6.2 API Endpoints

**Endpoints**:

1. **GET /health**  
   Check server status

2. **POST /api/v1/analyze**  
   Run analysis
   ```json
   {
     "sources": [
       {"type": "kubernetes", "kubeconfig": "..."}
     ],
     "analysis": {
       "target": "payment-service",
       "max_depth": 3
     }
   }
   ```

3. **GET /api/v1/graphs/:id**  
   Fetch previous analysis

**Lines of code**: ~400  
**Time**: 2 hours

---

### 6.3 Async Processing

Support long-running analyses with webhooks

```rust
POST /api/v1/analyze
Response: {"id": "analysis-123", "status": "pending"}

(polls every few seconds)

GET /api/v1/analyses/analysis-123
Response: {"id": "...", "status": "completed", "result": {...}}
```

**Lines of code**: ~200  
**Time**: 1 hour

---

### 6.4 OpenAPI Documentation

Generate Swagger/OpenAPI spec for REST API

**Lines of code**: ~100  
**Time**: 0.5 hours

---

### 6.5 Testing

Integration tests for all endpoints

**Lines of code**: ~200  
**Time**: 0.5 hours

---

### Phase 6 Summary

| Component | LOC | Time | Status |
|-----------|-----|------|--------|
| HTTP server | 150 | 1h | NEW |
| API endpoints | 400 | 2h | NEW |
| Async processing | 200 | 1h | NEW |
| OpenAPI | 100 | 0.5h | NEW |
| Testing | 200 | 0.5h | NEW |
| **TOTAL** | **1,050** | **5h** | |

**Delivers**: GETTING_STARTED.md Method 8 (REST API variant)  
**Users can**: Embed analysis in web dashboards, external tools

---

## BUILD SUMMARY

| Phase | Component | LOC | Time | Delivers |
|-------|-----------|-----|------|----------|
| 1 | PyPI Publish | - | 0.5h | Unblocks all |
| 2 | CLI Implementation | 2,050 | 8h | Methods 1-3 |
| 3 | Config Files | 1,300 | 5.5h | Method 4 |
| 4 | GitHub Actions | 220 | 3h | Method 8 |
| 5 | Examples | 500 | 3h | All working |
| 6 | REST API | 1,050 | 5h | Method 8 API |
| **TOTAL** | | **5,120** | **24.5h** | **v0.1.0 MVP** |

---

## Build Schedule

### Day 1 (8 hours)
- [x] Phase 1: Unblock PyPI (0.5h)
- [ ] Phase 2 start: URI parser → Scan commands (7.5h)

### Day 2 (8 hours)
- [ ] Phase 2 finish: Analyze → Error handling (2.5h)
- [ ] Phase 3: Config support (5.5h)

### Day 3 (8.5 hours)
- [ ] Phase 4: GitHub Actions (3h)
- [ ] Phase 5: Examples (3h)
- [ ] Phase 6 start: REST API (2.5h)

---

## Quality Gates

### Before releasing v0.1.0:

- [ ] All CLI commands working
- [ ] All methods documented and working
- [ ] 80%+ test coverage
- [ ] All examples run successfully
- [ ] No critical bugs in error paths
- [ ] README updated
- [ ] CHANGELOG updated
- [ ] Tagged and released to PyPI

---

## Rollback Plan

If issues arise during build:

1. **Phase 1 fails (PyPI)**: Skip to Phase 2, build locally only
2. **Phase 2 fails (CLI)**: Focus on Python API only (Methods 5-6)
3. **Phase 3 fails (Config)**: Skip to Phase 2.5, implement Python config only
4. **Phase 4+ fails**: Release without those features, add in v0.2

Minimum viable release: Phase 1 + Phase 2 = 8.5 hours (Methods 1-3)

---

## Go/No-Go Checklist

- [ ] v0.1.0 wheels published to PyPI
- [ ] `pip install pyblastradius` works on all platforms
- [ ] All imports resolved (no missing modules)
- [ ] Sample Kubernetes cluster available for testing (or mock)
- [ ] GitHub Actions secrets configured if needed
- [ ] Documentation reviewed and cross-checked

**Status**: Ready to start when Phase 1 complete.

---

## Next Action

**Immediate**:
1. Verify PyPI publish workflow status
2. Confirm wheels are published
3. Test `pip install pyblastradius`
4. Then proceed to Phase 2 uninterrupted
