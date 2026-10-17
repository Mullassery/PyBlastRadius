"""Discovery engines for finding dependencies in infrastructure."""

from abc import ABC, abstractmethod

try:
    from pyblastradius._core import Edge, EdgeType, Graph, Node, NodeType
except ImportError:
    Graph = Edge = NodeType = EdgeType = None
    class Node:
        def __init__(self, id: str, name: str, node_type: str, metadata: dict | None = None):
            self.id = id
            self.name = name
            self.type = node_type
            self.metadata = metadata or {}


class Discoverer(ABC):
    """Base class for discovery engines"""

    @abstractmethod
    def discover(self) -> "Graph":
        """Discover dependencies and return graph"""


# KubernetesDiscoverer, OtelDiscoverer and AirflowDiscoverer used to live
# here as pure-Python placeholders that always returned an empty graph:
# KubernetesDiscoverer globbed for *.yaml files and did `pass` in the loop
# body, OtelDiscoverer's discover() was a comment followed by an empty graph,
# and AirflowDiscoverer stored a host/port it never used. All three are now
# `pyblastradius.KubernetesDiscoverer` / `.OtelDiscoverer` /
# `.AirflowDiscoverer`, backed by the real Rust parsers (see
# src/discovery/kubernetes.rs, src/discovery/otlp.rs,
# src/discovery/airflow.rs, and src/python.rs's PyKubernetesDiscoverer,
# PyOtelDiscoverer, PyAirflowDiscoverer) -- import them from the top-level
# `pyblastradius` package, not from this module. Same for
# TerraformDiscoverer, wired up earlier (see src/discovery/terraform.rs).


__all__ = [
    "Discoverer",
]
