"""In-process REST API workflow (counts toward coverage)."""

from __future__ import annotations

import importlib
import json

import pytest

pytest.importorskip("fastapi")
pytest.importorskip("uvicorn")


@pytest.fixture()
def server(monkeypatch):
    monkeypatch.setenv("PYBLASTRADIUS_API_KEY", "test-key-123")
    import pyblastradius.server as module

    return importlib.reload(module)


@pytest.fixture()
def client(server):
    from fastapi.testclient import TestClient

    return TestClient(server.app)


def _headers():
    return {"Authorization": "Bearer test-key-123"}


def test_health_is_public(client):
    assert client.get("/health").status_code == 200


def test_protected_routes_require_key(client):
    assert client.get("/api/v1/status").status_code == 401
    assert client.get("/api/v1/status", headers={"Authorization": "wrong"}).status_code == 401


def test_scan_empty_sources(client):
    response = client.post("/api/v1/scan", headers=_headers(), json={"sources": []})
    assert response.status_code == 200, response.text
    assert response.json()["nodes_count"] == 0


def test_load_analyze_fetch_and_criticality(client):
    import pyblastradius as pbr

    graph = pbr.Graph()
    api = pbr.Node("api", pbr.NodeType.Service)
    db = pbr.Node("db", pbr.NodeType.Database)
    api_id = graph.add_node(api)
    db_id = graph.add_node(db)
    graph.add_edge(pbr.Edge(pbr.EdgeType.Calls, api_id, db_id))

    loaded = client.post(
        "/api/v1/graphs/load", headers=_headers(), json=json.loads(graph.to_json())
    )
    assert loaded.status_code == 200, loaded.text
    assert loaded.json()["nodes"] == 2

    analyzed = client.post("/api/v1/analyze", headers=_headers(), json={"service": "api"})
    assert analyzed.status_code == 200, analyzed.text
    analysis_id = analyzed.json()["id"]

    fetched = client.get(f"/api/v1/analyses/{analysis_id}", headers=_headers())
    assert fetched.status_code == 200, fetched.text
    assert fetched.json()["result"]["service"] == "api"
    assert client.get("/api/v1/analyses/missing", headers=_headers()).status_code == 404

    criticality = client.post("/api/v1/criticality", headers=_headers(), json={"limit": 5})
    assert criticality.status_code == 200, criticality.text
    assert criticality.json()["count"] == 2

    listed = client.get("/api/v1/graphs", headers=_headers())
    assert listed.status_code == 200
    assert listed.json()["current"]["nodes"] == 2


def test_openapi_document_contains_core_paths(client):
    response = client.get("/api/v1/openapi.json", headers=_headers())
    assert response.status_code == 200
    paths = response.json()["paths"]
    assert "/api/v1/scan" in paths
    assert "/api/v1/analyze" in paths


def test_analyze_without_graph_is_a_clean_400(client):
    response = client.post("/api/v1/analyze", headers=_headers(), json={"service": "api"})
    assert response.status_code == 400
    assert "No graph loaded" in response.json()["detail"]
