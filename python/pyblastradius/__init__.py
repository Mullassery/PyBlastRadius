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

__version__ = "0.4.1"
__author__ = "Georgi Mammen Mullassery"
__email__ = "mullassery@gmail.com"
__license__ = "Apache-2.0"

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
        AirflowDiscoverer,
        BlastRadiusAnalyzer,
        BlastRadiusResult,
        CriticalityScore,
        CriticalityScorer,
        Edge,
        EdgeType,
        FailureType,
        Graph,
        HealthStatus,
        # Real Kubernetes manifest discovery (local YAML parsing + env-var
        # reference inference - see src/discovery/kubernetes.rs), real
        # OTLP/gRPC trace-based discovery (see src/discovery/otlp.rs), and
        # real Airflow DAG discovery (source-level parsing of the DAG
        # Python files - see src/discovery/airflow.rs). Import these instead
        # of pyblastradius.discovery's KubernetesDiscoverer/OtelDiscoverer/
        # AirflowDiscoverer, which were pure-Python placeholders that always
        # returned an empty graph (same situation TerraformDiscoverer was in
        # before it was wired up).
        KubernetesDiscoverer,
        Node,
        NodeType,
        OtelDiscoverer,
        SimulationResult,
        Simulator,
        # Real Terraform discovery (hcl-rs parsing + reference-based
        # dependency detection - see src/discovery/terraform.rs). Import
        # this instead of pyblastradius.discovery's TerraformDiscoverer,
        # which was a pure-Python placeholder that discarded every .tf
        # file's contents and always returned an empty graph.
        TerraformDiscoverer,
        # Data-driven list of the discovery schemes this build supports,
        # straight from the Rust ProviderRegistry (no hand-maintained
        # Python list to drift out of sync).
        available_providers,
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
    TerraformDiscoverer = None
    KubernetesDiscoverer = None
    OtelDiscoverer = None
    AirflowDiscoverer = None
    available_providers = None

# Python-only exports (analysis helpers not backed by the Rust engine).
from pyblastradius.analysis import (
    AnomalyDetector,
    HiddenDependencyDetector,
)
from pyblastradius.integrations import (
    DbtManifestParser,
    OtelCollectorClient,
    StatGuardianClient,
)

__all__ = [
    "AirflowDiscoverer",
    # Analysis (Python-only, not backed by the Rust engine - see
    # pyblastradius/analysis.py docstrings for current limitations)
    "AnomalyDetector",
    "BlastRadiusAnalyzer",
    "BlastRadiusResult",
    "CriticalityScore",
    "CriticalityScorer",
    "DbtManifestParser",
    "Edge",
    "EdgeType",
    "FailureType",
    # Core (Rust engine)
    "Graph",
    "HealthStatus",
    "HiddenDependencyDetector",
    # Discovery
    "KubernetesDiscoverer",
    "Node",
    "NodeType",
    # Integrations
    "OtelCollectorClient",
    "OtelDiscoverer",
    "SimulationResult",
    "Simulator",
    "StatGuardianClient",
    "TerraformDiscoverer",
    "available_providers",
]
