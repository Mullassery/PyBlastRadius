# Contributing to PyBlastRadius

Thank you for your interest in contributing! This document provides guidelines and setup instructions.

---

## Development Setup

### Prerequisites

- **Rust 1.70+**: Install from [rustup.rs](https://rustup.rs/)
- **Git**: Version control
- **Python 3.10-3.13**: For building/testing the `pyblastradius._core` PyO3
  extension module and running the pytest suite (PyO3 0.23.5 does not
  support Python 3.14+ as the build interpreter)
- **Protobuf compiler**: For OTLP support (`brew install protobuf`)

### Clone & Build

```bash
git clone https://github.com/Mullassery/PyBlastRadius.git
cd PyBlastRadius

# Build in dev mode
cargo build

# Run tests
cargo test

# Run CLI
cargo run -- --version
```

### IDE Setup

#### VS Code

Install extensions:
- rust-analyzer
- Rust Syntax
- Even Better TOML

`.vscode/settings.json`:
```json
{
  "[rust]": {
    "editor.formatOnSave": true,
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  },
  "rust-analyzer.check.command": "clippy"
}
```

#### IntelliJ IDEA / RustRover

- Install Rust plugin
- Enable Cargo build tool window

---

## Code Style

### Rust Conventions

We follow standard Rust conventions:

- **Naming**: `snake_case` for functions/variables, `PascalCase` for types
- **Formatting**: `cargo fmt` (run before commit)
- **Linting**: `cargo clippy` (fix warnings)
- **Documentation**: Doc comments on public items

### Example

```rust
/// Analyzes blast radius for a service failure.
///
/// # Arguments
///
/// * `graph` - The dependency graph
/// * `node_id` - Service to analyze
///
/// # Returns
///
/// Blast radius analysis result
pub fn analyze(graph: &Graph, node_id: NodeId) -> Result<BlastRadiusResult> {
    // Implementation
}
```

### Running Checks

```bash
# Format code
cargo fmt

# Lint with Clippy
cargo clippy -- -D warnings

# Run all checks
cargo test && cargo fmt && cargo clippy
```

---

## Testing

### Unit Tests

Add tests in the same file as the code:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        // Arrange
        let input = "test";

        // Act
        let result = function_under_test(input);

        // Assert
        assert_eq!(result, expected_value);
    }
}
```

### Integration Tests

Add in `tests/integration/`:

```rust
#[test]
fn test_end_to_end() {
    let graph = load_test_graph("tests/fixtures/sample.json").unwrap();
    let result = analyze(&graph).unwrap();
    assert!(!result.nodes.is_empty());
}
```

### Running Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_blast_radius

# Run tests in a file
cargo test --lib graph::tests

# Run with coverage
cargo tarpaulin --out Html
```

### Testing the Python Bindings

The Python package (`python/pyblastradius/`) wraps the compiled
`pyblastradius._core` extension module. To build it and run the pytest
suite that exercises it:

```bash
python3.11 -m venv .venv   # any 3.10-3.13 interpreter; PyO3 0.23.5 rejects 3.14+
source .venv/bin/activate
pip install maturin
maturin develop --release          # builds src/ and installs it as pyblastradius
pip install -e ".[dev,server]"     # test/dev tooling + FastAPI server extras
pytest tests/ -q
```

Verified 2026-09-19: this sequence builds cleanly and all 35 tests in
`tests/` pass. Note `tests/test_server.py` needs `httpx` (pulled in via the
`dev` extra) because `fastapi.testclient.TestClient` requires it at import
time -- without it those 4 tests fail with an import error, not a real
regression in the code under test.

### Test Fixtures

Store test data in `tests/fixtures/`:

```
tests/
└── fixtures/
    ├── simple_graph.json
    ├── large_graph.json
    └── kubernetes_manifest.yaml
```

---

## Commit Guidelines

### Commit Message Format

```
<type>: <subject>

<body>

<footer>
```

**Type**: `feat`, `fix`, `docs`, `refactor`, `perf`, `test`, `chore`

**Subject**: Imperative, present tense ("add" not "added")

**Body**: Explain WHY, not WHAT (git diff shows what)

**Footer**: Reference issues: `Closes #123`

### Examples

```
feat: implement blast radius analysis

Add BFS-based algorithm to analyze failure cascade paths.
Supports depth-limited analysis for large graphs.

Closes #42
```

```
fix: correct criticality score calculation

The recovery_time weight was inverted. Fixed formula to match spec.

Closes #85
```

---

## Pull Request Process

1. **Fork** the repository
2. **Create** a feature branch: `git checkout -b feat/my-feature`
3. **Develop** with tests: `cargo test`
4. **Format** code: `cargo fmt`
5. **Lint**: `cargo clippy`
6. **Commit** with clear messages
7. **Push** to your fork
8. **Create PR** with description
9. **Address** review feedback

### PR Template

```markdown
## Description
Brief summary of changes

## Related Issues
Closes #123

## Testing
- [ ] Added unit tests
- [ ] Added integration tests
- [ ] Tested locally
- [ ] CI passes

## Checklist
- [ ] Code formatted (`cargo fmt`)
- [ ] Clippy passes (`cargo clippy`)
- [ ] All tests pass (`cargo test`)
- [ ] Documentation updated
- [ ] Commits are atomic and well-message
```

---

## Adding a New Discovery Engine

To add support for a new infrastructure/config format:

### 1. Create Module

`src/discovery/myservice.rs`:

```rust
use crate::error::Result;

pub struct MyServiceDiscoverer {
    pub config_path: String,
}

impl MyServiceDiscoverer {
    pub fn new(config_path: String) -> Self {
        MyServiceDiscoverer { config_path }
    }

    pub fn discover(&self) -> Result<crate::Graph> {
        // 1. Parse config file
        let config = self.parse_config()?;

        // 2. Create graph
        let mut graph = crate::Graph::new();

        // 3. Add nodes from config
        for item in config.items {
            let node = crate::Node::new(item.name, item.node_type);
            graph.add_node(node)?;
        }

        // 4. Add edges for relationships
        // ...

        Ok(graph)
    }

    fn parse_config(&self) -> Result<Config> {
        // Parsing logic
    }
}
```

### 2. Register in Module

`src/discovery/mod.rs`:

```rust
pub mod myservice;
pub use myservice::MyServiceDiscoverer;
```

### 3. Add CLI Support

`src/bin/cli.rs`:

```rust
#[derive(Parser)]
struct ScanArgs {
    #[arg(short, long)]
    myservice: Option<PathBuf>,
    // ...
}

// In scan command handler:
if let Some(path) = args.myservice {
    let discoverer = MyServiceDiscoverer::new(path.to_string_lossy().to_string());
    let graph = discoverer.discover()?;
    // Merge into main graph
}
```

### 4. Add Tests

`tests/integration/discovery_myservice.rs`:

```rust
#[test]
fn test_myservice_discovery() {
    let discoverer = MyServiceDiscoverer::new("tests/fixtures/myservice-config.yaml");
    let graph = discoverer.discover().unwrap();

    assert!(graph.node_count() > 0);
    assert!(graph.edge_count() > 0);
}
```

---

## Reporting Issues

Use [GitHub Issues](https://github.com/Mullassery/PyBlastRadius/issues).

**Bug Report**:
```markdown
### Description
Brief summary

### Steps to Reproduce
1. Run...
2. Observe...

### Expected Behavior
...

### Actual Behavior
...

### Environment
- OS: Linux/macOS/Windows
- Rust version: 1.72
- PyBlastRadius version: 0.1.0
```

**Feature Request**:
```markdown
### Summary
Brief description of feature

### Use Case
Why this is needed

### Proposed Solution
How it should work

### Alternatives
Other approaches considered
```

---

## Release Process

> For maintainers only

### Version Bump

Update version in:
- `Cargo.toml`
- `pyproject.toml`
- `README.md`

```bash
# Update version
sed -i 's/version = "0.1.0"/version = "0.2.0"/g' Cargo.toml

# Build & test
cargo build && cargo test

# Commit
git commit -am "chore: bump version to 0.2.0"

# Tag
git tag -a v0.2.0 -m "Release v0.2.0"

# Push
git push origin main
git push origin v0.2.0
```

### PyPI Publishing

```bash
# Build wheels
pip install maturin
maturin build --release

# Publish
twine upload target/wheels/*
```

---

## Code of Conduct

Be respectful and inclusive. All contributors are expected to follow the [Rust Code of Conduct](https://www.rust-lang.org/en-US/conduct.html).

---

## Getting Help

- **Documentation**: Check [docs/](docs/)
- **Discussions**: [GitHub Discussions](https://github.com/Mullassery/PyBlastRadius/discussions)
- **Email**: mullassery@gmail.com

---

**Happy Contributing!**
