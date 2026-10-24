"""Configuration loading, validation, env resolution, and CLI merging."""

from __future__ import annotations

from pathlib import Path

import pytest

from pyblastradius.config import AnalysisConfig, Config, DiscoverySource, OutputConfig


def test_default_config_is_valid() -> None:
    config = Config.default()
    assert config.version == "1"
    assert config.analysis == AnalysisConfig()
    assert config.discovery_sources == []
    assert config.outputs == []
    assert config.alerts == []
    assert config.validate() is True


def test_load_from_file_reads_all_sections(tmp_path: Path) -> None:
    pytest.importorskip("yaml")
    path = tmp_path / "blastradius.yml"
    path.write_text(
        """
version: "2"
discovery:
  sources:
    - type: terraform
      path: infra
    - type: kubernetes
      namespace: prod
      required: false
analysis:
  max_depth: 5
  include_transitive: false
outputs:
  - type: json
    path: out.json
alerts:
  - condition: blast_radius_score > 0.8
    actions:
      - type: slack
        channel: "#incidents"
""".strip()
    )

    config = Config.load_from_file(str(path))

    assert config.version == "2"
    assert [s.type for s in config.discovery_sources] == ["terraform", "kubernetes"]
    assert config.discovery_sources[0].params == {"path": "infra"}
    assert config.discovery_sources[1].params == {"namespace": "prod"}
    assert config.discovery_sources[1].required is False
    assert config.analysis.max_depth == 5
    assert config.analysis.include_transitive is False
    assert config.outputs == [OutputConfig(type="json", params={"path": "out.json"})]
    assert config.alerts[0].condition == "blast_radius_score > 0.8"
    assert config.alerts[0].actions == [{"type": "slack", "channel": "#incidents"}]


def test_missing_file_is_a_clear_error(tmp_path: Path) -> None:
    pytest.importorskip("yaml")
    with pytest.raises(FileNotFoundError):
        Config.load_from_file(str(tmp_path / "missing.yml"))


def test_environment_variables_are_resolved_recursively(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    pytest.importorskip("yaml")
    monkeypatch.setenv("PBR_NAMESPACE", "staging")
    monkeypatch.setenv("PBR_CHANNEL", "#alerts")
    path = tmp_path / "blastradius.yml"
    path.write_text(
        """
discovery:
  sources:
    - type: kubernetes
      namespace: ${PBR_NAMESPACE}
      labels:
        env: ${PBR_NAMESPACE}
alerts:
  - condition: score > 0.5
    actions:
      - type: slack
        channel: ${PBR_CHANNEL}
""".strip()
    )

    config = Config.load_from_file(str(path))

    assert config.discovery_sources[0].params == {
        "namespace": "staging",
        "labels": {"env": "staging"},
    }
    assert config.alerts[0].actions == [{"type": "slack", "channel": "#alerts"}]


def test_validate_rejects_out_of_range_depth() -> None:
    config = Config.default()
    config.analysis.max_depth = 0
    with pytest.raises(ValueError, match="max_depth"):
        config.validate()
    config.analysis.max_depth = 21
    with pytest.raises(ValueError, match="max_depth"):
        config.validate()


def test_merge_cli_args_overrides_depth_and_sources() -> None:
    config = Config.default()
    config.discovery_sources.append(DiscoverySource(type="terraform", params={"path": "old"}))

    config.merge_cli_args(
        {
            "max_depth": 7,
            "sources": ["kubernetes://namespace=prod,context=prod-ctx", "terraform://path=infra"],
        }
    )

    assert config.analysis.max_depth == 7
    assert [(s.type, s.params) for s in config.discovery_sources] == [
        ("kubernetes", {"namespace": "prod", "context": "prod-ctx"}),
        ("terraform", {"path": "infra"}),
    ]


def test_invalid_source_uri_is_rejected() -> None:
    config = Config.default()
    with pytest.raises(ValueError, match="Invalid URI"):
        config.merge_cli_args({"sources": ["not-a-uri"]})
    with pytest.raises(ValueError, match="Invalid parameter"):
        config.merge_cli_args({"sources": ["kubernetes://namespace"]})


def test_to_dict_round_trips_through_loader(tmp_path: Path) -> None:
    pytest.importorskip("yaml")
    config = Config.default()
    config.version = "3"
    config.analysis.max_depth = 4
    config.analysis.include_transitive = False
    config.discovery_sources.append(
        DiscoverySource(type="terraform", params={"path": "infra"}, required=False)
    )
    config.outputs.append(OutputConfig(type="html", params={"path": "report.html"}))

    path = tmp_path / "roundtrip.yml"
    path.write_text(__import__("yaml").safe_dump(config.to_dict()))

    loaded = Config.load_from_file(str(path))
    assert loaded.version == "3"
    assert loaded.analysis.max_depth == 4
    assert loaded.analysis.include_transitive is False
    assert loaded.discovery_sources[0].params == {"path": "infra"}
    assert loaded.discovery_sources[0].required is False
    assert loaded.outputs[0].type == "html"
    assert loaded.outputs[0].params == {"path": "report.html"}
