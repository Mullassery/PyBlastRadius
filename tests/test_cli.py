"""End-to-end tests of the `pyblastradius` CLI commands.

These exercise the actual click commands in pyblastradius/cli.py via
click's CliRunner, against a real graph file and the real Rust bindings -
this is the layer that had multiple broken-API-usage bugs before this fix
(`Graph.merge([a, b])` instead of `a.merge(b)`, `graph.services()` which
never existed, and a `_SCENARIO_TO_FAILURE_TYPE` dict that had been spliced
in between a click decorator stack and its function, which is an outright
SyntaxError). Running these commands for real is the only way to catch
that class of bug - importing the module isn't enough since decorators and
name resolution only fail once the code path actually runs.
"""

import json

from click.testing import CliRunner

import pyblastradius as pbr
from pyblastradius.cli import cli


def _write_sample_graph(path):
    graph, ids = _build_graph()
    path.write_text(graph.to_json())
    return ids


def _build_graph():
    graph = pbr.Graph()
    api = pbr.Node("api-server", pbr.NodeType.Service)
    auth = pbr.Node("auth-service", pbr.NodeType.Service)
    db = pbr.Node("postgresql", pbr.NodeType.Database)

    ids = {
        "api-server": graph.add_node(api),
        "auth-service": graph.add_node(auth),
        "postgresql": graph.add_node(db),
    }
    graph.add_edge(pbr.Edge(pbr.EdgeType.Calls, ids["api-server"], ids["auth-service"]))
    graph.add_edge(pbr.Edge(pbr.EdgeType.Calls, ids["auth-service"], ids["postgresql"]))
    return graph, ids


def test_cli_module_imports_and_has_no_syntax_errors():
    """Regression test for the `_SCENARIO_TO_FAILURE_TYPE` dict that was
    briefly spliced between the `simulate` command's decorators and its
    `def`, which is a SyntaxError in Python and would make the whole
    module unimportable.
    """
    import pyblastradius.cli  # noqa: F401 - import success is the assertion


def test_analyze_command_json_output(tmp_path):
    graph_file = tmp_path / "graph.json"
    _write_sample_graph(graph_file)

    runner = CliRunner()
    result = runner.invoke(
        cli, ["analyze", str(graph_file), "--service", "api-server", "--format", "json"]
    )

    assert result.exit_code == 0, result.output
    data = json.loads(result.output)
    assert data["service"] == "api-server"
    assert set(data["directly_impacted"]) == {"auth-service"}
    assert data["total_impacted"] == 2


def test_analyze_command_unknown_service_fails_cleanly(tmp_path):
    graph_file = tmp_path / "graph.json"
    _write_sample_graph(graph_file)

    runner = CliRunner()
    result = runner.invoke(
        cli, ["analyze", str(graph_file), "--service", "does-not-exist"]
    )
    assert result.exit_code != 0
    assert "does-not-exist" in result.output


def test_criticality_command_json_output(tmp_path):
    graph_file = tmp_path / "graph.json"
    _write_sample_graph(graph_file)

    runner = CliRunner()
    result = runner.invoke(
        cli, ["criticality", str(graph_file), "--format", "json"]
    )
    assert result.exit_code == 0, result.output
    data = json.loads(result.output)
    names = {row["service"] for row in data}
    assert names == {"api-server", "auth-service", "postgresql"}


def test_simulate_command_known_scenario(tmp_path):
    graph_file = tmp_path / "graph.json"
    _write_sample_graph(graph_file)

    runner = CliRunner()
    result = runner.invoke(
        cli, ["simulate", str(graph_file), "--target", "postgresql", "--scenario", "outage"]
    )
    assert result.exit_code == 0, result.output
    assert "Impacted Services" in result.output
    assert "Business Impact Score" in result.output


def test_simulate_command_unknown_scenario_fails_cleanly(tmp_path):
    graph_file = tmp_path / "graph.json"
    _write_sample_graph(graph_file)

    runner = CliRunner()
    result = runner.invoke(
        cli,
        ["simulate", str(graph_file), "--target", "postgresql", "--scenario", "bogus-scenario"],
    )
    assert result.exit_code != 0
    assert "bogus-scenario" in result.output
