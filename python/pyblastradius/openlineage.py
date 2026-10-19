"""OpenLineage runtime lineage ingestion.

OpenLineage (Apache-2.0) is the cross-orchestrator standard emitted by
Airflow, dbt, Spark and others. This adapter converts run events into a
PyBlastRadius graph:

    input datasets --Feeds--> job --Feeds--> output datasets

It merges into the same node model used by Terraform/Kubernetes/Airflow/OTLP
discovery, so blast-radius and criticality analysis work unchanged.
"""

from __future__ import annotations

import json
from collections.abc import Iterable
from pathlib import Path
from typing import Any

try:
    from ._core import Edge, EdgeType, Graph, Node, NodeType
except ImportError:  # pragma: no cover - extension not built
    Edge = None
    EdgeType = None
    Graph = None
    Node = None
    NodeType = None

from .discovery import Discoverer


class OpenLineageUnavailable(RuntimeError):
    """Raised when the Rust graph bindings are unavailable."""


def _require_graph():
    if Graph is None:
        raise OpenLineageUnavailable(
            "PyBlastRadius's compiled Rust bindings are not available; build with "
            "`maturin develop` or install a wheel."
        )
    return Graph


def _dataset_name(dataset: dict[str, Any]) -> str:
    namespace = str(dataset.get("namespace") or "").strip()
    name = str(dataset.get("name") or "").strip()
    if namespace and name:
        return f"{namespace}:{name}"
    return name or namespace or "unknown-dataset"


def _job_name(job: dict[str, Any]) -> str:
    namespace = str(job.get("namespace") or "").strip()
    name = str(job.get("name") or "").strip()
    if namespace and name:
        return f"{namespace}:{name}"
    return name or namespace or "unknown-job"


class _GraphAccumulator:
    def __init__(self) -> None:
        self.graph = _require_graph()()
        self.ids: dict[tuple[str, str], str] = {}
        self.edges: set[tuple[str, str, str]] = set()

    def node(self, name: str, node_type: Any, *, source: str = "openlineage", **tags: str) -> str:
        key = (str(node_type), name)
        existing = self.ids.get(key)
        if existing is not None:
            return existing
        node = Node(name, node_type)
        for tag_key, tag_value in tags.items():
            if tag_value:
                node.add_tag(tag_key, str(tag_value))
        node.add_tag("discovered_via", source)
        node_id = self.graph.add_node(node)
        self.ids[key] = node_id
        return node_id

    def edge(self, source: str, target: str, edge_type: Any, **tags: str) -> None:
        key = (source, target, str(edge_type))
        if key in self.edges or source == target:
            return
        edge = Edge(edge_type, source, target)
        self.graph.add_edge(edge)
        self.edges.add(key)


def events_to_graph(events: Iterable[dict[str, Any]]) -> Any:
    """Convert OpenLineage run events to a graph."""
    acc = _GraphAccumulator()
    for event in events:
        if not isinstance(event, dict):
            continue
        job = event.get("job") or {}
        run = event.get("run") or {}
        job_name = _job_name(job)
        job_id = acc.node(
            job_name,
            NodeType.Pipeline,
            source="openlineage",
            job_namespace=str(job.get("namespace") or ""),
            job_name=str(job.get("name") or ""),
        )
        for dataset in event.get("inputs") or []:
            if not isinstance(dataset, dict):
                continue
            dataset_id = acc.node(
                _dataset_name(dataset),
                NodeType.Table,
                source="openlineage",
                dataset_namespace=str(dataset.get("namespace") or ""),
                dataset_name=str(dataset.get("name") or ""),
            )
            acc.edge(
                dataset_id,
                job_id,
                EdgeType.Feeds,
                lineage="openlineage",
                run_id=str(run.get("runId") or ""),
            )
        for dataset in event.get("outputs") or []:
            if not isinstance(dataset, dict):
                continue
            dataset_id = acc.node(
                _dataset_name(dataset),
                NodeType.Table,
                source="openlineage",
                dataset_namespace=str(dataset.get("namespace") or ""),
                dataset_name=str(dataset.get("name") or ""),
            )
            acc.edge(
                job_id,
                dataset_id,
                EdgeType.Feeds,
                lineage="openlineage",
                run_id=str(run.get("runId") or ""),
            )
    return acc.graph


def load_events(path: str | Path) -> list[dict[str, Any]]:
    """Load one JSON event, a JSON array, or newline-delimited JSON."""
    text = Path(path).read_text(encoding="utf-8")
    stripped = text.strip()
    if not stripped:
        return []
    if stripped.startswith("["):
        data = json.loads(stripped)
        if not isinstance(data, list):
            raise ValueError("OpenLineage JSON array expected")
        return [item for item in data if isinstance(item, dict)]
    if stripped.startswith("{"):
        try:
            data = json.loads(stripped)
        except json.JSONDecodeError:
            # NDJSON fallback
            return [json.loads(line) for line in stripped.splitlines() if line.strip()]
        return [data] if isinstance(data, dict) else []
    return [json.loads(line) for line in stripped.splitlines() if line.strip()]


def discover_openlineage(path: str | Path) -> Any:
    """Load an OpenLineage file and build a graph."""
    return events_to_graph(load_events(path))


class OpenLineageDiscoverer(Discoverer):
    """PyBlastRadius discoverer for OpenLineage JSON/NDJSON files."""

    def __init__(self, path: str = ".", recursive: bool = True):
        self.path = Path(path)
        self.recursive = recursive

    def discover(self) -> Any:
        if self.path.is_file():
            return discover_openlineage(self.path)
        events: list[dict[str, Any]] = []
        pattern = "**/*.json*" if self.recursive else "*.json*"
        for file_path in sorted(self.path.glob(pattern)):
            if file_path.is_file():
                try:
                    events.extend(load_events(file_path))
                except (OSError, ValueError, json.JSONDecodeError):
                    continue
        return events_to_graph(events)


__all__ = [
    "OpenLineageDiscoverer",
    "OpenLineageUnavailable",
    "discover_openlineage",
    "events_to_graph",
    "load_events",
]
