"""Discovery engines for finding dependencies in infrastructure."""

from abc import ABC, abstractmethod
from typing import Dict

try:
    from pyblastradius._core import Graph, Node, Edge, NodeType, EdgeType
except ImportError:
    Graph = Edge = NodeType = EdgeType = None
    class Node:
        def __init__(self, id: str, name: str, node_type: str, metadata: Dict = None):
            self.id = id
            self.name = name
            self.type = node_type
            self.metadata = metadata or {}


class Discoverer(ABC):
    """Base class for discovery engines"""

    @abstractmethod
    def discover(self) -> "Graph":
        """Discover dependencies and return graph"""
        pass


# KubernetesDiscoverer and OtelDiscoverer used to live here as pure-Python
# placeholders: KubernetesDiscoverer globbed for *.yaml files and did
# `pass` in the loop body, OtelDiscoverer's discover() was a comment
# followed by an empty graph. Both always returned an empty graph. They're
# now `pyblastradius.KubernetesDiscoverer` / `pyblastradius.OtelDiscoverer`,
# backed by the real Rust parsers (see src/discovery/kubernetes.rs,
# src/discovery/otlp.rs / src/python.rs's PyKubernetesDiscoverer,
# PyOtelDiscoverer) -- import them from the top-level `pyblastradius`
# package, not from this module. Same for TerraformDiscoverer, wired up
# earlier (see src/discovery/terraform.rs).


class AirflowDiscoverer(Discoverer):
    """Discover dependencies from Airflow DAGs"""

    def __init__(self, host: str = "localhost", port: int = 8080):
        self.host = host
        self.port = port

    def discover(self) -> Graph:
        """Discover from Airflow DAGs"""
        graph = Graph()

        # Try to connect to Airflow API or parse DAG files
        # For now, return empty graph
        return graph


__all__ = [
    "Discoverer",
    "AirflowDiscoverer",
]
