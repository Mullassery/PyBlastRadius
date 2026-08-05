"""Analysis engines for blast radius, criticality, and anomalies."""

from dataclasses import dataclass
from typing import List, Optional, Dict, Any
from collections import defaultdict, deque

from pyblastradius._core import PyGraph


@dataclass
class BlastRadiusResult:
    """Result of blast radius analysis"""
    service: str
    score: float
    directly_impacted: List[str]
    indirectly_impacted: List[str]
    critical_paths: List[List[str]]

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary"""
        return {
            "service": self.service,
            "score": self.score,
            "directly_impacted": self.directly_impacted,
            "indirectly_impacted": self.indirectly_impacted,
            "critical_paths": self.critical_paths,
        }


class BlastRadiusAnalyzer:
    """Analyze blast radius (impact of service failure)"""

    def __init__(self, graph: PyGraph):
        self.graph = graph

    def analyze(self, service: str, max_depth: int = 3) -> BlastRadiusResult:
        """Analyze blast radius for a service"""
        if not self.graph:
            raise ValueError("Graph is not initialized")

        # BFS to find impacted services
        visited = set()
        directly_impacted = []
        indirectly_impacted = []

        queue = deque([(service, 0)])
        visited.add(service)

        while queue:
            current, depth = queue.popleft()

            if depth == 0:
                pass  # This is the root service
            elif depth == 1:
                directly_impacted.append(current)
            elif depth <= max_depth:
                indirectly_impacted.append(current)

            if depth < max_depth:
                # Find services that depend on this one (get predecessors/incoming edges)
                try:
                    for pred in self.graph.get_neighbors(current):
                        if pred not in visited:
                            visited.add(pred)
                            queue.append((pred, depth + 1))
                except Exception:
                    pass

        # Calculate blast radius score
        total_services = len([n for n in self.graph.list_nodes()
                            if self.graph.get_node_type(n) == "Service"])
        impacted_count = len(directly_impacted) + len(indirectly_impacted)
        score = impacted_count / max(total_services, 1)

        return BlastRadiusResult(
            service=service,
            score=score,
            directly_impacted=directly_impacted,
            indirectly_impacted=indirectly_impacted,
            critical_paths=[],
        )


class CriticalityScorer:
    """Score services by criticality (how many others depend on them)"""

    def __init__(self, graph: PyGraph):
        self.graph = graph

    def score(self, service: str) -> float:
        """Score a service's criticality (0-1)"""
        if not self.graph:
            raise ValueError("Graph is not initialized")

        # Count how many things depend on this service (predecessors)
        try:
            dependents = len(self.graph.get_predecessors(service))
        except Exception:
            dependents = 0

        total_nodes = self.graph.node_count()
        criticality = dependents / max(total_nodes, 1)
        return min(1.0, criticality)

    def score_all(self) -> Dict[str, float]:
        """Score all services by criticality"""
        scores = {}
        for node in self.graph.list_nodes():
            try:
                if self.graph.get_node_type(node) == "Service":
                    scores[node] = self.score(node)
            except Exception:
                pass
        return scores


class Simulator:
    """Simulate cascading failures"""

    def __init__(self, graph: PyGraph):
        self.graph = graph

    def simulate_cascade(self, root_service: str, max_depth: int = 5) -> Dict[str, Any]:
        """Simulate cascading failure from a service"""
        if not self.graph:
            raise ValueError("Graph is not initialized")

        # BFS to find all impacted services
        visited = set()
        queue = deque([root_service])
        visited.add(root_service)
        impacted = [root_service]

        while queue:
            current = queue.popleft()
            try:
                for neighbor in self.graph.get_neighbors(current):
                    if neighbor not in visited:
                        visited.add(neighbor)
                        impacted.append(neighbor)
                        queue.append(neighbor)
            except Exception:
                pass

        return {
            "root": root_service,
            "impacted_count": len(impacted),
            "impacted": impacted,
        }
