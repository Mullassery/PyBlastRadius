"""Analysis engines for blast radius, criticality, and anomalies."""

from dataclasses import dataclass
from typing import List, Optional, Dict, Any
from abc import ABC, abstractmethod

try:
    from pyblastradius._core import Graph
except ImportError:
    Graph = None


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

    def __init__(self, graph: Graph):
        self.graph = graph

    def analyze(self, service: str, max_depth: int = 3) -> BlastRadiusResult:
        """Analyze blast radius for a service"""
        if not self.graph:
            raise ValueError("Graph is not initialized")

        # BFS to find impacted services
        visited = set()
        directly_impacted = []
        indirectly_impacted = []

        queue = [(service, 0)]
        visited.add(service)

        while queue:
            current, depth = queue.pop(0)

            if depth == 0:
                pass  # This is the root service
            elif depth == 1:
                directly_impacted.append(current)
            elif depth <= max_depth:
                indirectly_impacted.append(current)

            if depth < max_depth:
                # Find dependent services (services that depend on current)
                for edge in self.graph.edges():
                    if edge.source == current and edge.target not in visited:
                        visited.add(edge.target)
                        queue.append((edge.target, depth + 1))

        # Calculate blast radius score
        total_services = len(self.graph.services())
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

    def __init__(self, graph: Graph):
        self.graph = graph

    def score(self, service: str) -> float:
        """Score a service's criticality (0-1)"""
        if not self.graph:
            raise ValueError("Graph is not initialized")

        # Count dependencies
        dependents = 0
        for edge in self.graph.edges():
            if edge.target == service:
                dependents += 1

        total_edges = len(list(self.graph.edges()))
        score = dependents / max(total_edges, 1)

        return min(score, 1.0)

    def score_all(self) -> Dict[str, float]:
        """Score all services"""
        scores = {}
        for service in self.graph.services():
            scores[service] = self.score(service)
        return scores


class Simulator:
    """Simulate failure scenarios and cascade effects"""

    def __init__(self, graph: Graph):
        self.graph = graph

    def simulate_cascade(self, target: str, max_depth: int = 10) -> Dict[str, Any]:
        """Simulate cascade failure"""
        impacted = set([target])
        queue = [target]
        depth_map = {target: 0}

        while queue:
            current = queue.pop(0)
            current_depth = depth_map[current]

            if current_depth < max_depth:
                for edge in self.graph.edges():
                    if edge.source == current and edge.target not in impacted:
                        impacted.add(edge.target)
                        depth_map[edge.target] = current_depth + 1
                        queue.append(edge.target)

        return {
            "target": target,
            "impacted_count": len(impacted) - 1,
            "impacted_services": list(impacted - {target}),
            "recovery_time_min": 30,
            "recovery_time_max": 120,
        }


class AnomalyDetector:
    """Detect anomalies in service behavior"""

    def __init__(self, graph: Graph):
        self.graph = graph

    def detect(self, service: str) -> List[Dict[str, Any]]:
        """Detect anomalies for a service"""
        # Placeholder: would integrate with metrics/traces
        return []

    def detect_all(self) -> Dict[str, List[Dict[str, Any]]]:
        """Detect anomalies for all services"""
        anomalies = {}
        for service in self.graph.services():
            anomalies[service] = self.detect(service)
        return anomalies


class HiddenDependencyDetector:
    """Detect undocumented dependencies"""

    def __init__(self, graph: Graph):
        self.graph = graph

    def detect(self) -> List[Dict[str, Any]]:
        """Detect hidden dependencies"""
        # Placeholder: would scan logs, configs, etc.
        return []


__all__ = [
    "BlastRadiusResult",
    "BlastRadiusAnalyzer",
    "CriticalityScorer",
    "Simulator",
    "AnomalyDetector",
    "HiddenDependencyDetector",
]
