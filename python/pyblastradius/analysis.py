"""Python-only analysis helpers that are NOT backed by the Rust engine.

`BlastRadiusAnalyzer`, `CriticalityScorer`, and `Simulator` used to be
defined in this module as pure-Python reimplementations. They have been
removed: the real, tested, petgraph-backed implementations are now reachable
from Python via the `pyblastradius._core` extension module and are
re-exported at the top of the `pyblastradius` package (`from pyblastradius
import BlastRadiusAnalyzer, CriticalityScorer, Simulator`). The old versions
here were dead weight that shadowed the real bindings and crashed on first
use (they called `Graph.services()`, which has never existed on the Rust
`Graph` class - only `Graph.nodes()`/`Graph.edges()` do).

What remains below (`AnomalyDetector`, `HiddenDependencyDetector`) has no
equivalent exposed from the Rust core yet - the Rust crate does have
`analysis::anomaly_detection` and `analysis::hidden_dependencies` modules,
but they are not wired into `src/python.rs`, and their current
implementations are themselves mostly heuristic placeholders (two of the
four hidden-dependency detection methods are literal no-ops that document
"For MVP: Return empty"). These Python classes are kept as explicit,
honest placeholders - they return empty results rather than fabricating
findings - so that code depending on them does not crash, but they should
not be relied on for real anomaly/hidden-dependency detection.
"""

from typing import List, Dict, Any

try:
    from pyblastradius._core import Graph
except ImportError:
    Graph = None


class AnomalyDetector:
    """Placeholder anomaly detector.

    Not backed by the Rust engine or by any real metrics/trace source yet.
    `detect()`/`detect_all()` always return empty results.
    """

    def __init__(self, graph: Graph):
        self.graph = graph

    def detect(self, service: str) -> List[Dict[str, Any]]:
        """Detect anomalies for a service. Placeholder: always returns []."""
        # Placeholder: would integrate with metrics/traces
        return []

    def detect_all(self) -> Dict[str, List[Dict[str, Any]]]:
        """Detect anomalies for all nodes in the graph. Placeholder: always empty per node."""
        anomalies = {}
        if self.graph is not None:
            for node in self.graph.nodes():
                anomalies[node.name] = self.detect(node.name)
        return anomalies


class HiddenDependencyDetector:
    """Placeholder hidden-dependency detector.

    Not backed by the Rust `analysis::hidden_dependencies` module (which is
    itself not wired into the Python bindings). `detect()` always returns [].
    """

    def __init__(self, graph: Graph):
        self.graph = graph

    def detect(self) -> List[Dict[str, Any]]:
        """Detect hidden dependencies. Placeholder: always returns []."""
        # Placeholder: would scan logs, configs, etc.
        return []


__all__ = [
    "AnomalyDetector",
    "HiddenDependencyDetector",
]
