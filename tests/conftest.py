"""Shared fixtures for the PyBlastRadius test suite.

These tests exercise the *real* compiled `pyblastradius._core` extension
module (built via `maturin develop`/`maturin build`) - there are no mocks or
stand-ins for the Rust engine here. If the extension module was not built,
`import pyblastradius` still succeeds (see the try/except in
`pyblastradius/__init__.py`), but `pyblastradius.Graph` etc. will be `None`
and these tests will fail loudly rather than silently passing against a
fake.
"""

import pytest

import pyblastradius as pbr


@pytest.fixture()
def sample_graph():
    """A small, realistic dependency graph used across multiple tests.

    Topology (edges are "calls"):
        api-server -> auth-service -> postgresql
        api-server -> redis

    So a failure of `postgresql` should cascade up through `auth-service`
    to `api-server`, and a blast-radius query *from* `api-server` (this
    engine's blast radius walks outgoing/"calls" edges from the source) hits
    `auth-service` and `redis` directly, and `postgresql` indirectly.
    """
    graph = pbr.Graph()

    api = pbr.Node("api-server", pbr.NodeType.Service)
    auth = pbr.Node("auth-service", pbr.NodeType.Service)
    db = pbr.Node("postgresql", pbr.NodeType.Database)
    cache = pbr.Node("redis", pbr.NodeType.Cache)

    ids = {
        "api-server": graph.add_node(api),
        "auth-service": graph.add_node(auth),
        "postgresql": graph.add_node(db),
        "redis": graph.add_node(cache),
    }

    graph.add_edge(pbr.Edge(pbr.EdgeType.Calls, ids["api-server"], ids["auth-service"]))
    graph.add_edge(pbr.Edge(pbr.EdgeType.Calls, ids["auth-service"], ids["postgresql"]))
    graph.add_edge(pbr.Edge(pbr.EdgeType.Calls, ids["api-server"], ids["redis"]))

    return graph, ids
