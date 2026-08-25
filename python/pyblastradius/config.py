"""Configuration management for PyBlastRadius."""

import os
import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional, List, Dict, Any

try:
    import yaml
except ImportError:
    yaml = None


@dataclass
class DiscoverySource:
    """Configuration for a discovery source"""
    type: str
    params: Dict[str, Any] = field(default_factory=dict)
    required: bool = True


@dataclass
class AnalysisConfig:
    """Configuration for analysis"""
    max_depth: int = 3
    include_transitive: bool = True


@dataclass
class OutputConfig:
    """Configuration for output"""
    type: str  # json, html, slack, etc.
    params: Dict[str, Any] = field(default_factory=dict)


@dataclass
class AlertConfig:
    """Configuration for alerts"""
    condition: str  # e.g. "blast_radius_score > 0.8"
    actions: List[Dict[str, Any]] = field(default_factory=list)


class Config:
    """PyBlastRadius configuration"""

    def __init__(self):
        self.version: str = "1"
        self.discovery_sources: List[DiscoverySource] = []
        self.analysis: AnalysisConfig = AnalysisConfig()
        self.outputs: List[OutputConfig] = []
        self.alerts: List[AlertConfig] = []

    @classmethod
    def load_from_file(cls, path: str = "blastradius.yml") -> "Config":
        """Load configuration from file"""
        if not yaml:
            raise ImportError("PyYAML is required for config file support: pip install pyyaml")

        config_path = Path(path)
        if not config_path.exists():
            raise FileNotFoundError(f"Config file not found: {path}")

        with open(config_path) as f:
            data = yaml.safe_load(f) or {}

        config = cls()
        config._load_from_dict(data)
        config._resolve_env_vars()
        return config

    @classmethod
    def default(cls) -> "Config":
        """Create default configuration"""
        return cls()

    def _load_from_dict(self, data: Dict[str, Any]) -> None:
        """Load configuration from dictionary"""
        self.version = str(data.get("version", "1"))

        # Load discovery sources
        for source_data in data.get("discovery", {}).get("sources", []):
            source_type = source_data.get("type")
            if not source_type:
                continue

            params = {k: v for k, v in source_data.items() if k != "type" and k != "required"}
            required = source_data.get("required", True)

            self.discovery_sources.append(DiscoverySource(
                type=source_type,
                params=params,
                required=required,
            ))

        # Load analysis config
        analysis_data = data.get("analysis", {})
        self.analysis = AnalysisConfig(
            max_depth=analysis_data.get("max_depth", 3),
            include_transitive=analysis_data.get("include_transitive", True),
        )

        # Load outputs
        for output_data in data.get("outputs", []):
            output_type = output_data.get("type")
            if not output_type:
                continue

            params = {k: v for k, v in output_data.items() if k != "type"}
            self.outputs.append(OutputConfig(
                type=output_type,
                params=params,
            ))

        # Load alerts
        for alert_data in data.get("alerts", []):
            condition = alert_data.get("condition")
            if not condition:
                continue

            self.alerts.append(AlertConfig(
                condition=condition,
                actions=alert_data.get("actions", []),
            ))

    def _resolve_env_vars(self) -> None:
        """Resolve environment variables in configuration"""
        def resolve_value(value):
            if isinstance(value, str):
                # Replace ${VAR_NAME} with environment variable
                def replace_env(match):
                    var_name = match.group(1)
                    return os.environ.get(var_name, match.group(0))

                return re.sub(r'\$\{(\w+)\}', replace_env, value)
            elif isinstance(value, dict):
                return {k: resolve_value(v) for k, v in value.items()}
            elif isinstance(value, list):
                return [resolve_value(item) for item in value]
            return value

        # Resolve in discovery sources
        for source in self.discovery_sources:
            source.params = resolve_value(source.params)

        # Resolve in outputs
        for output in self.outputs:
            output.params = resolve_value(output.params)

        # Resolve in alerts
        for alert in self.alerts:
            alert.actions = resolve_value(alert.actions)

    def validate(self) -> bool:
        """Validate configuration"""
        if not self.version:
            raise ValueError("Config version is required")

        if self.analysis.max_depth < 1 or self.analysis.max_depth > 20:
            raise ValueError("Analysis max_depth must be 1-20")

        return True

    def merge_cli_args(self, args: Dict[str, Any]) -> None:
        """Merge CLI arguments (overrides config file)"""
        if "max_depth" in args and args["max_depth"]:
            self.analysis.max_depth = args["max_depth"]

        if "sources" in args and args["sources"]:
            # CLI sources override config file sources
            self.discovery_sources.clear()
            for source_uri in args["sources"]:
                self._add_source_from_uri(source_uri)

    def _add_source_from_uri(self, uri: str) -> None:
        """Add discovery source from URI"""
        if "://" not in uri:
            raise ValueError(f"Invalid URI format: {uri}")

        scheme, params_str = uri.split("://", 1)
        params = {}

        if params_str:
            for param in params_str.split(","):
                if "=" not in param:
                    raise ValueError(f"Invalid parameter format: {param}")
                key, value = param.split("=", 1)
                params[key.strip()] = value.strip()

        self.discovery_sources.append(DiscoverySource(
            type=scheme,
            params=params,
        ))

    def create_discoverers(self) -> List:
        """Create discoverer instances from configuration"""
        from pyblastradius import TerraformDiscoverer
        from pyblastradius.discovery import (
            KubernetesDiscoverer,
            AirflowDiscoverer,
            OtelDiscoverer,
        )

        discoverers = []

        for source in self.discovery_sources:
            try:
                if source.type == "kubernetes":
                    discoverers.append(KubernetesDiscoverer(**source.params))
                elif source.type == "terraform":
                    discoverers.append(TerraformDiscoverer(**source.params))
                elif source.type == "airflow":
                    discoverers.append(AirflowDiscoverer(**source.params))
                elif source.type == "otlp":
                    discoverers.append(OtelDiscoverer(**source.params))
            except Exception as e:
                if source.required:
                    raise ValueError(f"Failed to create {source.type} discoverer: {str(e)}")

        return discoverers

    def to_dict(self) -> Dict[str, Any]:
        """Convert configuration to dictionary"""
        return {
            "version": self.version,
            "discovery": {
                "sources": [
                    {
                        "type": source.type,
                        "required": source.required,
                        **source.params,
                    }
                    for source in self.discovery_sources
                ]
            },
            "analysis": {
                "max_depth": self.analysis.max_depth,
                "include_transitive": self.analysis.include_transitive,
            },
            "outputs": [
                {
                    "type": output.type,
                    **output.params,
                }
                for output in self.outputs
            ],
            "alerts": [
                {
                    "condition": alert.condition,
                    "actions": alert.actions,
                }
                for alert in self.alerts
            ],
        }


__all__ = ["Config", "DiscoverySource", "AnalysisConfig", "OutputConfig", "AlertConfig"]
