"""Tests for the `pyblastradius._core` pyo3 extension module itself.

Before this fix, `import pyblastradius` could never succeed at all because
no `#[pymodule]` existed in the Rust crate (see src/python.rs). These tests
confirm the module actually loads and that the Graph/Node/Edge bindings are
thin, correct wrappers around the real Rust `Graph` (not Python
reimplementations).
"""

import json

import pyblastradius as pbr


def test_core_extension_module_imports():
    """The core regression this whole effort exists to fix."""
    from pyblastradius import _core

    assert _core.Graph is not None
    assert _core.BlastRadiusAnalyzer is not None
    # __version__ on the compiled extension is derived from Cargo.toml's
    # CARGO_PKG_VERSION at build time, independent of the Python package's
    # __init__.py string.
    assert _core.__version__


def test_package_level_reexports_are_the_real_bindings():
    """pyblastradius.BlastRadiusAnalyzer etc. must be the Rust bindings, not
    the old Python stand-ins that used to shadow them (see git history:
    those called Graph.services(), which never existed on the Rust type).
    """
    from pyblastradius import _core

    assert pbr.Graph is _core.Graph
    assert pbr.BlastRadiusAnalyzer is _core.BlastRadiusAnalyzer
    assert pbr.CriticalityScorer is _core.CriticalityScorer
    assert pbr.Simulator is _core.Simulator


def test_node_and_edge_basic_construction():
    node = pbr.Node("api-server", pbr.NodeType.Service)
    assert node.name == "api-server"
    assert node.node_type == pbr.NodeType.Service
    # UUID string, not empty/placeholder
    assert len(node.id) == 36 and node.id.count("-") == 4


def test_graph_add_node_and_get_node_roundtrip(sample_graph):
    graph, ids = sample_graph
    assert graph.node_count() == 4
    assert graph.edge_count() == 3

    node = graph.get_node(ids["postgresql"])
    assert node is not None
    assert node.name == "postgresql"
    assert node.node_type == pbr.NodeType.Database


def test_graph_get_node_missing_id_returns_none(sample_graph):
    graph, _ids = sample_graph
    import uuid

    assert graph.get_node(str(uuid.uuid4())) is None


def test_graph_find_node_by_name(sample_graph):
    graph, ids = sample_graph
    found = graph.find_node_by_name("auth-service")
    assert found is not None
    assert found.id == ids["auth-service"]

    assert graph.find_node_by_name("does-not-exist") is None


def test_graph_neighbors_and_predecessors(sample_graph):
    graph, ids = sample_graph
    # api-server calls auth-service and redis
    neighbors = graph.neighbors(ids["api-server"])
    assert sorted(graph.get_node(n).name for n in neighbors) == ["auth-service", "redis"]

    # auth-service is called only by api-server
    preds = graph.predecessors(ids["auth-service"])
    assert [graph.get_node(p).name for p in preds] == ["api-server"]


def test_graph_has_path(sample_graph):
    graph, ids = sample_graph
    assert graph.has_path(ids["api-server"], ids["postgresql"]) is True
    assert graph.has_path(ids["postgresql"], ids["api-server"]) is False


def test_graph_metrics(sample_graph):
    graph, _ids = sample_graph
    metrics = graph.metrics()
    assert metrics["node_count"] == 4
    assert metrics["edge_count"] == 3
    assert metrics["density"] > 0


def test_graph_json_roundtrip(sample_graph):
    graph, ids = sample_graph
    raw = graph.to_json()
    parsed = json.loads(raw)  # must be real, valid JSON
    assert isinstance(parsed, dict)

    restored = pbr.Graph.from_json(raw)
    assert restored.node_count() == graph.node_count()
    assert restored.edge_count() == graph.edge_count()
    assert restored.get_node(ids["postgresql"]).name == "postgresql"


def test_graph_merge_is_instance_method_not_static(sample_graph):
    """Regression test: cli.py used to call the non-existent
    `Graph.merge([graph, sub_graph])` (a static/list-taking call). The real
    binding is an instance method: `graph.merge(other)`.
    """
    graph, _ids = sample_graph
    other = pbr.Graph()
    other.add_node(pbr.Node("standalone-service", pbr.NodeType.Service))

    graph.merge(other)
    assert graph.node_count() == 5


def test_graph_to_dot_contains_node_names(sample_graph):
    graph, _ids = sample_graph
    dot = graph.to_dot()
    assert "digraph" in dot
    assert "api-server" in dot
