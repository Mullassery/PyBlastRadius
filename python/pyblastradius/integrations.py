"""Integration clients for external systems."""

from typing import Optional, List, Dict, Any
import json


class OtelCollectorClient:
    """Connect to OpenTelemetry collector"""

    def __init__(self, endpoint: str = "localhost:4317", protocol: str = "grpc"):
        self.endpoint = endpoint
        self.protocol = protocol

    def discover_dependencies(self, time_window_minutes: int = 60) -> "Graph":
        """Discover dependencies from traces"""
        # Placeholder: would connect to OTLP and extract traces
        from pyblastradius import Graph
        return Graph()


class StatGuardianClient:
    """Connect to StatGuardian for data quality"""

    def __init__(self, endpoint: str, api_key: str):
        self.endpoint = endpoint
        self.api_key = api_key

    def get_quality_rules(self) -> List[Dict[str, Any]]:
        """Get quality rules"""
        return []

    def track_quality_incident(self, incident: Dict[str, Any]) -> Dict[str, Any]:
        """Track a quality incident"""
        return {"incident_id": "sg-001"}

    def get_quality_events(self) -> List[Dict[str, Any]]:
        """Get quality events"""
        return []


class DbtManifestParser:
    """Parse dbt manifest for lineage"""

    def __init__(self, manifest_path: str):
        self.manifest_path = manifest_path

    def parse_manifest(self) -> Dict[str, Any]:
        """Parse manifest file"""
        try:
            with open(self.manifest_path) as f:
                return json.load(f)
        except Exception:
            return {}

    def extract_column_lineage(self) -> Dict[str, List[str]]:
        """Extract column-level lineage"""
        manifest = self.parse_manifest()
        lineage = {}

        for model_id, model in manifest.get("nodes", {}).items():
            if model.get("resource_type") == "model":
                model_name = model.get("name")
                columns = model.get("columns", {})
                lineage[model_name] = list(columns.keys())

        return lineage


__all__ = [
    "OtelCollectorClient",
    "StatGuardianClient",
    "DbtManifestParser",
]
