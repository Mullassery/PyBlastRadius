"""Tests for the real Kubernetes/OTLP discovery bindings.

KubernetesDiscoverer/OtelDiscoverer in `pyblastradius.discovery` used to be
pure-Python placeholders that always returned an empty graph (KubernetesDiscoverer
globbed for *.yaml and did `pass` in the loop body; OtelDiscoverer's discover()
was a comment followed by an empty graph), even though the real Rust parsers in
src/discovery/kubernetes.rs and src/discovery/otlp.rs existed the whole time --
they just weren't wired into src/python.rs's #[pymodule]. These tests exercise
the real compiled bindings (pyblastradius.KubernetesDiscoverer /
pyblastradius.OtelDiscoverer), not the old stubs.
"""

import time

import pytest

import pyblastradius as pbr


def _write(path, name, content):
    (path / name).write_text(content)


def test_kubernetes_discoverer_parses_real_manifests(tmp_path):
    _write(
        tmp_path,
        "service.yaml",
        """
apiVersion: v1
kind: Service
metadata:
  name: api-gateway
  namespace: default
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: api-gateway
  namespace: default
spec:
  template:
    spec:
      containers:
        - name: api
          env:
            - name: DOWNSTREAM_URL
              value: "http://auth-service.default.svc.cluster.local"
---
apiVersion: v1
kind: Service
metadata:
  name: auth-service
  namespace: default
""",
    )

    discoverer = pbr.KubernetesDiscoverer(manifests_dir=str(tmp_path))
    graph = discoverer.discover()

    # 2 real nodes (api-gateway, auth-service) and 1 real edge inferred from
    # the env var reference -- this can only be correct if the YAML was
    # actually parsed, not discarded.
    assert graph.node_count() == 2
    assert graph.edge_count() == 1
    names = {n.name for n in graph.nodes()}
    assert names == {"api-gateway.default", "auth-service.default"}


def test_kubernetes_discoverer_ignores_non_workload_manifests(tmp_path):
    _write(
        tmp_path,
        "configmap.yaml",
        """
apiVersion: v1
kind: ConfigMap
metadata:
  name: some-config
""",
    )

    graph = pbr.KubernetesDiscoverer(manifests_dir=str(tmp_path)).discover()
    assert graph.node_count() == 0


def test_kubernetes_discoverer_kubeconfig_and_context_accepted_but_inert(tmp_path):
    """kubeconfig/context are accepted for interface compatibility with the
    old stub's constructor and existing config/REST payloads, but have no
    effect -- this discoverer reads local manifests, not a live cluster."""
    _write(
        tmp_path,
        "service.yaml",
        "apiVersion: v1\nkind: Service\nmetadata:\n  name: svc\n  namespace: default\n",
    )

    discoverer = pbr.KubernetesDiscoverer(
        manifests_dir=str(tmp_path),
        kubeconfig="/nonexistent/kubeconfig",
        context="some-context",
    )
    graph = discoverer.discover()
    assert graph.node_count() == 1


def test_kubernetes_discoverer_no_manifests_dir_found_returns_empty_graph(
    tmp_path, monkeypatch
):
    # No manifests_dir given, and none of the conventional k8s/kubernetes
    # directories exist under cwd -- an honest empty graph, not an error.
    monkeypatch.chdir(tmp_path)
    graph = pbr.KubernetesDiscoverer().discover()
    assert graph.node_count() == 0


def test_otel_discoverer_empty_collection_window_produces_empty_graph():
    discoverer = pbr.OtelDiscoverer(endpoint="127.0.0.1:0", timeout_seconds=1)
    start = time.time()
    graph = discoverer.discover()
    elapsed = time.time() - start

    assert graph.node_count() == 0
    assert graph.edge_count() == 0
    # Should return once the real 1-second collection window elapses, not
    # hang indefinitely.
    assert elapsed < 5


def test_otel_discoverer_duration_minutes_converts_to_timeout_seconds():
    # duration_minutes is kept for interface compatibility with the old
    # stub's constructor; it must still reach the real Rust timeout_seconds
    # field (verified via __repr__, since there's no separate getter -- same
    # minimal-surface convention TerraformDiscoverer's root_dir uses).
    discoverer = pbr.OtelDiscoverer(endpoint="127.0.0.1:0", duration_minutes=2)
    assert "timeout_seconds=120" in repr(discoverer)


def test_otel_discoverer_timeout_seconds_overrides_duration_minutes():
    discoverer = pbr.OtelDiscoverer(
        endpoint="127.0.0.1:0", duration_minutes=5, timeout_seconds=7
    )
    assert "timeout_seconds=7" in repr(discoverer)
