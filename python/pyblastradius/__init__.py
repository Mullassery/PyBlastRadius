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

__version__ = "0.1.0"
__author__ = "Georgi Mammen Mullassery"
__email__ = "mullassery@gmail.com"
__license__ = "Proprietary License — Free to use with explicit attribution"

# Core exports
try:
    from pyblastradius._core import (
        Graph,
        Node,
        Edge,
        NodeType,
        EdgeType,
    )
except ImportError:
    # Fallback if Rust bindings not available (development mode)
    Graph = None
    Node = None
    Edge = None
    NodeType = None
    EdgeType = None

# Python-only exports
from pyblastradius.discovery import (
    KubernetesDiscoverer,
    TerraformDiscoverer,
    AirflowDiscoverer,
    OtelDiscoverer,
    LocalDiscoverer,
)
from pyblastradius.analysis import (
    BlastRadiusAnalyzer,
    CriticalityScorer,
    Simulator,
    BlastRadiusResult,
)

__all__ = [
    # Core
    "Graph",
    "Node",
    "Edge",
    "NodeType",
    "EdgeType",
    # Discovery
    "KubernetesDiscoverer",
    "TerraformDiscoverer",
    "AirflowDiscoverer",
    "OtelDiscoverer",
    # Analysis
    "BlastRadiusAnalyzer",
    "CriticalityScorer",
    "Simulator",
    "AnomalyDetector",
    "HiddenDependencyDetector",
    # Integrations
    "OtelCollectorClient",
    "StatGuardianClient",
    "DbtManifestParser",
]
