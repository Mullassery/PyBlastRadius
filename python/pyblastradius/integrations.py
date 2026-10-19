"""Integration clients for external systems."""

import json
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from pyblastradius import Graph


class OtelCollectorClient:
    """Connect to OpenTelemetry collector"""

    def __init__(self, endpoint: str = "localhost:4317", protocol: str = "grpc"):
        self.endpoint = endpoint
        self.protocol = protocol

    def discover_dependencies(self, time_window_minutes: int = 60) -> "Graph":
        """Discover dependencies from traces.

        Not implemented: no OTLP client is wired into this package yet. Raising
        is deliberate -- returning an empty Graph would look like a successful
        discovery that found nothing.
        """
        raise NotImplementedError(
            "OTel trace discovery is not wired into pyblastradius yet. "
            "Feed a Graph explicitly or use the Rust core."
        )


class StatGuardianClient:
    """Connect to StatGuardian for data quality"""

    def __init__(self, endpoint: str, api_key: str):
        self.endpoint = endpoint
        self.api_key = api_key

    def get_quality_rules(self) -> list[dict[str, Any]]:
        """Get quality rules (not implemented)."""
        raise NotImplementedError(
            "StatGuardian quality rules are not wired into pyblastradius yet; "
            "no request is sent and no placeholder rules are fabricated."
        )

    def track_quality_incident(self, incident: dict[str, Any]) -> dict[str, Any]:
        """Track a quality incident (not implemented)."""
        raise NotImplementedError(
            "StatGuardian incident tracking is not wired into pyblastradius yet; "
            "no fabricated incident id is returned."
        )

    def get_quality_events(self) -> list[dict[str, Any]]:
        """Get quality events (not implemented)."""
        raise NotImplementedError(
            "StatGuardian quality events are not wired into pyblastradius yet."
        )


class DbtManifestParser:
    """Parse dbt manifest for lineage"""

    def __init__(self, manifest_path: str):
        self.manifest_path = manifest_path

    def parse_manifest(self) -> dict[str, Any]:
        """Parse manifest file"""
        try:
            with open(self.manifest_path) as f:
                return json.load(f)
        except Exception:
            return {}

    def extract_column_lineage(self) -> dict[str, list[str]]:
        """Extract column-level lineage"""
        manifest = self.parse_manifest()
        lineage = {}

        for model in manifest.get("nodes", {}).values():
            if model.get("resource_type") == "model":
                model_name = model.get("name")
                columns = model.get("columns", {})
                lineage[model_name] = list(columns.keys())

        return lineage


__all__ = [
    "DbtManifestParser",
    "OtelCollectorClient",
    "StatGuardianClient",
]
