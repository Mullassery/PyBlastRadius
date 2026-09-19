# PyBlastRadius CLI Implementation Plan

**Reference**: [INPUT_DESIGN.md](INPUT_DESIGN.md) — All CLI commands map to methods 1-3, 4, 7, 9, 10

---

## CLI Command Structure

```
pyblastradius
├── scan          [METHOD 1, 2]   Auto-discovery or explicit sources
├── analyze       [METHOD 3]      Blast radius analysis
├── criticality   [METHOD 3]      Criticality ranking
├── simulate      [METHOD 3]      Failure simulation
├── visualize     [METHOD 3]      Graph visualization
├── config        [METHOD 4]      Config file management
├── server        [METHOD 8]      REST API server
└── plugin        [v1.0+]         Plugin management
```

---

## Level 1: Auto-Discovery (Simplest)

**Related to**: INPUT_DESIGN.md § Method 1: CLI - Auto-Discovery

```bash
pyblastradius scan
```

### Implementation Checklist

- [ ] Auto-detect `blastradius.yml` in current directory
- [ ] Scan for Kubernetes manifests (recursive)
  - [ ] Find `*.yml`, `*.yaml` in typical locations (k8s/, kubernetes/)
  - [ ] Parse and validate as K8s resources
- [ ] Scan for Terraform files (recursive)
  - [ ] Find `*.tf` files in typical locations (infra/, infrastructure/)
  - [ ] Parse HCL (use existing terraform parser)
- [ ] Scan for docker-compose.yml
  - [ ] Parse services and dependencies
- [ ] Scan for Airflow DAGs
  - [ ] Find dags folder or dags.yaml
- [ ] Report what was discovered
  ```
  Detected resources:
    • Kubernetes: 127 manifests in k8s/
    • Terraform: 43 files in infrastructure/
    • Docker Compose: 1 file
    • Airflow: No DAGs found
  ```
- [ ] Merge graphs from all sources
- [ ] Output to `blast-radius.json` (default)
- [ ] Show next steps:
  ```
  Ready to analyze. Run:
    pyblastradius analyze
  ```

### Code Structure

```rust
// src/bin/cli.rs
mod scan {
    pub fn auto_discover() -> Result<Graph> {
        let mut graph = Graph::new();
        
        // Discover each source
        if let Ok(k8s_graph) = discover_kubernetes(".", recursive=true) {
            graph.merge(k8s_graph)?;
        }
        if let Ok(tf_graph) = discover_terraform(".", recursive=true) {
            graph.merge(tf_graph)?;
        }
        if let Ok(compose) = load_docker_compose(".") {
            graph.merge(compose_to_graph(compose)?)?;
        }
        // ... more sources
        
        Ok(graph)
    }
}
```

---

## Level 2: Explicit Sources (URI-based)

**Related to**: INPUT_DESIGN.md § Method 2: CLI - Discovery Sources

```bash
pyblastradius scan kubernetes://kubeconfig=~/.kube/config
pyblastradius scan terraform://path=./infra
pyblastradius scan \
  kubernetes://... \
  terraform://... \
  otlp://endpoint=localhost:4317
```

### Implementation Checklist

- [ ] Parse source URIs: `scheme://key1=value1,key2=value2`
- [ ] Support schemes:
  - [ ] `kubernetes://`: kubeconfig, context, namespaces
  - [ ] `terraform://`: path, backend, vars
  - [ ] `airflow://`: host, port, auth
  - [ ] `otlp://`: endpoint, duration, sample-rate
  - [ ] `local://`: path, recursive
  - [ ] `compose://`: file, project
  - [ ] `git://`: repo, branch, commit
  - [ ] `github://`: owner, repo, pr, commit
- [ ] Validate each URI before processing
- [ ] Error handling: Clear messages for invalid URIs
  ```
  Error: Invalid URI: kubernetes://kubeconfig=/nonexistent
  
  Suggestions:
  1. Check path exists: /nonexistent
  2. Use default: kubernetes:// (uses KUBECONFIG env var)
  ```
- [ ] Support multiple sources (merge graphs)
- [ ] Parallel source discovery (if safe)

### Code Structure

```rust
// src/bin/cli.rs
mod uri_parser {
    #[derive(Debug)]
    pub struct SourceUri {
        pub scheme: String,
        pub params: HashMap<String, String>,
    }
    
    pub fn parse(uri: &str) -> Result<SourceUri> {
        // kubernetes://kubeconfig=~/.kube/config,context=prod
        // → SourceUri { scheme: "kubernetes", params: {...} }
    }
    
    pub fn discover(uri: SourceUri) -> Result<Graph> {
        match uri.scheme.as_str() {
            "kubernetes" => discover_kubernetes_from_params(&uri.params),
            "terraform" => discover_terraform_from_params(&uri.params),
            _ => Err(Error::UnknownScheme(uri.scheme)),
        }
    }
}
```

---

## Level 3: Analysis Workflows

**Related to**: INPUT_DESIGN.md § Method 3: CLI - Analysis Workflows

```bash
pyblastradius analyze graph.json --service payment --max-depth 3
pyblastradius criticality graph.json --limit 10 --format table
pyblastradius simulate graph.json --target postgres
pyblastradius visualize graph.json --output graph.html
```

### Implementation Checklist

#### analyze command
- [ ] Load graph from JSON file
- [ ] Specify target service (required)
- [ ] `--max-depth N` option (default: 3)
- [ ] `--format` option (json, table, csv)
- [ ] Output blast radius results
  ```
  Service: payment-service
  Blast Radius Score: 0.82 (82%)
  
  Directly Impacted (1 hop):
    • api-gateway (critical)
    • payment-processor (critical)
    
  Indirectly Impacted (2 hops):
    • user-service
    • notification-service (9 services)
  ```

#### criticality command
- [ ] Load graph
- [ ] Rank all services by criticality
- [ ] `--limit N` (default: 10)
- [ ] `--format` (table, json, csv)
- [ ] Show single points of failure

#### simulate command
- [ ] Load graph
- [ ] Specify target service to fail
- [ ] `--scenario` option (cascade, cascade-with-fallback, etc.)
- [ ] Estimate recovery time
- [ ] Show failure propagation paths

#### visualize command
- [ ] Load graph
- [ ] `--output` file path
- [ ] Generate interactive HTML (default)
- [ ] `--format` option (html, dot, graphml, json)
- [ ] Color nodes by criticality
- [ ] Show edge weights/dependencies

---

## Level 4: Config File Support

**Related to**: INPUT_DESIGN.md § Method 4: Config File (blastradius.yml)

```bash
pyblastradius analyze  # Uses blastradius.yml in current dir
pyblastradius analyze --config custom.yml
```

### Implementation Checklist

- [ ] **Config parsing**: Load and validate YAML
  - [ ] Schema validation (see INPUT_DESIGN.md)
  - [ ] Required vs optional fields
  - [ ] Type checking (strings, numbers, booleans, arrays)

- [ ] **Config structure support**:
  ```yaml
  discovery:
    sources:
      - type: kubernetes
      - type: terraform
  analysis:
    max-depth: 5
  outputs:
    - type: json
    - type: html
  alerts:
    - condition: "blast_radius_score > 0.8"
  ```

- [ ] **Environment variable substitution**:
  - [ ] Support `${VAR_NAME}` or `$VAR_NAME` in config
  - [ ] `${SLACK_WEBHOOK}` → read from env
  - [ ] Error if required env var missing

- [ ] **CLI config override**:
  - [ ] CLI args override config file
  - [ ] `pyblastradius analyze --max-depth 10` overrides config

- [ ] **Config validation command**:
  ```bash
  pyblastradius config validate
  pyblastradius config show  # Show resolved config with env vars
  ```

- [ ] **Example config file**:
  - [ ] Create `.pyblastradius.example` in repo
  - [ ] Suggest during first run if config missing

### Code Structure

```rust
// src/config.rs
#[derive(Debug, Deserialize)]
pub struct BlastRadiusConfig {
    pub version: String,
    pub discovery: DiscoveryConfig,
    pub analysis: AnalysisConfig,
    pub outputs: Vec<OutputConfig>,
    pub alerts: Option<Vec<AlertConfig>>,
}

impl BlastRadiusConfig {
    pub fn load_from_file(path: &str) -> Result<Self> {
        let yaml = std::fs::read_to_string(path)?;
        let config: BlastRadiusConfig = serde_yaml::from_str(&yaml)?;
        config.validate()?;
        config.resolve_env_vars()?;
        Ok(config)
    }
    
    pub fn validate(&self) -> Result<()> {
        // Check required fields, types, etc.
    }
    
    pub fn resolve_env_vars(&mut self) -> Result<()> {
        // Replace ${VAR} with environment variables
    }
}
```

---

## Level 5: CI/CD Integration

**Related to**: INPUT_DESIGN.md § Method 7: CI/CD Integration

### GitHub Actions

```yaml
# .github/workflows/blast-radius.yml
- uses: Mullassery/pyblastradius-action@v1
  with:
    discovery-sources: |
      kubernetes://
      terraform://path=./infra
    max-depth: 5
```

### Implementation Checklist

- [ ] **GitHub Action** (`action.yml`):
  - [ ] Input: `discovery-sources` (multi-line)
  - [ ] Input: `max-depth`
  - [ ] Input: `failure-on-score` threshold
  - [ ] Output: `blast-radius-score`
  - [ ] Output: `analysis-json`

- [ ] **Action script**:
  - [ ] Install pyblastradius
  - [ ] Run scan + analyze
  - [ ] Parse results
  - [ ] Set outputs
  - [ ] Exit with appropriate code

- [ ] **PR comment support**:
  - [ ] Generate summary markdown
  - [ ] Post as PR comment
  - [ ] Include visualization link
  - [ ] Flag if score > threshold

---

## Level 6: REST API Server

**Related to**: INPUT_DESIGN.md § Method 8: REST API

```bash
pyblastradius server --port 8080
```

### Implementation Checklist

- [ ] Start HTTP server (actix-web or similar)
- [ ] Health check endpoint: `GET /health`
- [ ] Analysis endpoint: `POST /api/v1/analyze`
  - [ ] Accept JSON with sources + analysis config
  - [ ] Return analysis results
- [ ] Graph endpoint: `GET /api/v1/graphs/:id`
- [ ] Async processing support (for long analyses)
- [ ] WebSocket for streaming results
- [ ] OpenAPI/Swagger documentation

---

## Error Handling & UX

### Comprehensive Error Messages

Every error should include:
1. **What went wrong** (clear message)
2. **Where it happened** (file, line, config)
3. **Why it failed** (root cause)
4. **How to fix it** (actionable suggestions)

```bash
$ pyblastradius scan kubernetes://kubeconfig=/nonexistent

❌ Error: Kubernetes discovery failed

  Location: kubernetes://kubeconfig=/nonexistent
  Error: No such file or directory
  
  Suggestions:
  1. Verify kubeconfig path exists
     ls -la /nonexistent
  
  2. Use default kubeconfig location
     pyblastradius scan kubernetes://
  
  3. Check KUBECONFIG environment variable
     echo $KUBECONFIG
  
  Docs: https://docs.pyblastradius.io/discovery/kubernetes
```

### Progress Reporting

```bash
$ pyblastradius scan --verbose

[1/5] Loading config... ✅
[2/5] Discovering Kubernetes... ⏳ (127 manifests found)
[3/5] Discovering Terraform... ⏳ (43 files found)
[4/5] Discovering OTLP traces... ⏳ (this may take a minute)
[5/5] Merging graphs...

✅ Discovery complete: 170 nodes
Ready to analyze: pyblastradius analyze
```

---

## Testing Plan

### Unit Tests
- [ ] URI parsing (all schemes)
- [ ] Config loading + validation
- [ ] Config override logic
- [ ] Environment variable substitution

### Integration Tests
- [ ] Auto-discover in sample directory
- [ ] Explicit source discovery (mock each source)
- [ ] Config file with CLI override
- [ ] All analysis commands on sample graph
- [ ] CI/CD workflow simulation

### E2E Tests
- [ ] Full workflow: scan → analyze → visualize
- [ ] Real Kubernetes cluster (if available)
- [ ] Error scenarios (missing config, invalid URIs, etc.)

---

## Version Progression

### v0.1 (Current)
- ✅ `pyblastradius scan` (auto-discover)
- ✅ `pyblastradius analyze` (basic)
- ✅ Basic JSON output

### v0.2 (Target)
- [ ] All URI schemes working
- [ ] blastradius.yml support
- [ ] All analysis commands (criticality, simulate, visualize)
- [ ] Config validation + examples

### v0.3
- [ ] GitHub Actions integration
- [ ] REST API server
- [ ] Better error messages
- [ ] Progress reporting

### v1.0
- [ ] Plugin system for custom sources
- [ ] GraphQL API
- [ ] Production monitoring

---

## Documentation Map

**For each CLI feature, document:**

| Feature | Doc | Example | Reference |
|---------|-----|---------|-----------|
| `scan` auto | QUICK_START.md | `pyblastradius scan` | INPUT_DESIGN § Method 1 |
| `scan` URI | CLI_REFERENCE.md | `pyblastradius scan kubernetes://` | INPUT_DESIGN § Method 2 |
| `analyze` | CLI_REFERENCE.md | `pyblastradius analyze graph.json` | INPUT_DESIGN § Method 3 |
| `blastradius.yml` | CLI_REFERENCE.md | Config schema + examples | INPUT_DESIGN § Method 4 |
| CI/CD | CI_CD_INTEGRATION.md | GitHub Actions example | INPUT_DESIGN § Method 7 |
| REST API | API_REFERENCE.md | POST /api/v1/analyze | INPUT_DESIGN § Method 8 |

---

## Success Criteria

✅ User can run `pyblastradius scan` in any directory and get useful results
✅ All 10 input methods are documented with working examples
✅ Error messages point to solutions, not just problems
✅ Config file support works end-to-end
✅ CI/CD integration example is copy-paste ready
✅ All commands are discoverable via `--help`
