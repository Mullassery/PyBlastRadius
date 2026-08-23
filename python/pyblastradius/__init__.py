"""
PyBlastRadius: Operational Blast Radius Intelligence Platform

Predict cascading failures and quantify business impact across infrastructure,
data pipelines, and AI systems.

Quick start:
    from pyblastradius import KubernetesDiscoverer, BlastRadiusAnalyzer

    # Discover dependencies
    discoverer = KubernetesDiscoverer()
    graph = discoverer.discover()

    # Analyze blast radius
    result = BlastRadiusAnalyzer.analyze(graph, node_id)
    print(f"Blast radius score: {result.blast_radius_score:.1f}/100")
"""

__version__ = "0.2.1"
__author__ = "Georgi Mammen Mullassery"
__email__ = "mullassery@gmail.com"
__license__ = "Proprietary License — Free to use with explicit attribution"

# Core exports - real Rust engine, via the `pyblastradius._core` pyo3
# extension module (see src/python.rs). BlastRadiusAnalyzer, CriticalityScorer
# and Simulator here are the actual petgraph-backed implementations, not
# Python reimplementations: every call crosses into Rust.
#
# NOTE: prior to this fix, this module re-exported *Python* stand-ins for
# BlastRadiusAnalyzer/CriticalityScorer/Simulator from pyblastradius.analysis
# that shadowed these real bindings and crashed on first use (they called a
# `Graph.services()` method that has never existed on the Rust `Graph`
# class). That was a wiring bug, not a Rust bug - the real engine
# (verified via src/python.rs) was reachable and correct the whole time.
try:
    from pyblastradius._core import (
        Graph,
        Node,
        Edge,
        NodeType,
        EdgeType,
        HealthStatus,
        FailureType,
        BlastRadiusResult,
        CriticalityScore,
        SimulationResult,
        BlastRadiusAnalyzer,
        CriticalityScorer,
        Simulator,
    )
except ImportError:
    # Fallback if Rust bindings not available (development mode)
    Graph = None
    Node = None
    Edge = None
    NodeType = None
    EdgeType = None
    HealthStatus = None
    FailureType = None
    BlastRadiusResult = None
    CriticalityScore = None
    SimulationResult = None
    BlastRadiusAnalyzer = None
    CriticalityScorer = None
    Simulator = None

# Python-only exports
from pyblastradius.discovery import (
    KubernetesDiscoverer,
    TerraformDiscoverer,
    AirflowDiscoverer,
    OtelDiscoverer,
)
from pyblastradius.analysis import (
    AnomalyDetector,
    HiddenDependencyDetector,
)
from pyblastradius.integrations import (
    OtelCollectorClient,
    StatGuardianClient,
    DbtManifestParser,
)

__all__ = [
    # Core (Rust engine)
    "Graph",
    "Node",
    "Edge",
    "NodeType",
    "EdgeType",
    "HealthStatus",
    "FailureType",
    "BlastRadiusResult",
    "CriticalityScore",
    "SimulationResult",
    "BlastRadiusAnalyzer",
    "CriticalityScorer",
    "Simulator",
    # Discovery
    "KubernetesDiscoverer",
    "TerraformDiscoverer",
    "AirflowDiscoverer",
    "OtelDiscoverer",
    # Analysis (Python-only, not backed by the Rust engine - see
    # pyblastradius/analysis.py docstrings for current limitations)
    "AnomalyDetector",
    "HiddenDependencyDetector",
    # Integrations
    "OtelCollectorClient",
    "StatGuardianClient",
    "DbtManifestParser",
]
