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


def test_visualize_writes_interactive_html(tmp_path):
    graph_file = tmp_path / "graph.json"
    _write_sample_graph(graph_file)
    output = tmp_path / "graph.html"

    result = CliRunner().invoke(
        cli, ["visualize", str(graph_file), "--output", str(output)]
    )

    assert result.exit_code == 0, result.output
    html = output.read_text()
    assert "<!DOCTYPE html>" in html
    assert "api-server" in html
    assert "auth-service" in html


def test_visualize_writes_dot_and_json(tmp_path):
    graph_file = tmp_path / "graph.json"
    _write_sample_graph(graph_file)

    dot_out = tmp_path / "graph.dot"
    json_out = tmp_path / "graph-out.json"
    runner = CliRunner()

    dot_result = runner.invoke(
        cli, ["visualize", str(graph_file), "--format", "dot", "--output", str(dot_out)]
    )
    assert dot_result.exit_code == 0, dot_result.output
    assert "digraph" in dot_out.read_text().lower()

    json_result = runner.invoke(
        cli, ["visualize", str(graph_file), "--format", "json", "--output", str(json_out)]
    )
    assert json_result.exit_code == 0, json_result.output
    assert json.loads(json_out.read_text())["nodes"]


def test_scan_local_source_writes_graph(tmp_path):
    source = tmp_path / "infra"
    source.mkdir()
    (source / "main.tf").write_text(
        'resource "aws_instance" "api" {\n  ami = "ami-123"\n}\n'
    )
    output = tmp_path / "blast-radius.json"

    result = CliRunner().invoke(
        cli, ["scan", f"local://path={source}", "--output", str(output)]
    )

    assert result.exit_code == 0, result.output
    assert output.exists()
    assert "Discovery complete" in result.output


def test_scan_rejects_unknown_scheme_without_crashing(tmp_path):
    output = tmp_path / "graph.json"
    result = CliRunner().invoke(
        cli, ["scan", "not-a-uri", "--output", str(output)]
    )
    # The command reports the bad source and still writes an (empty) graph.
    assert result.exit_code == 0, result.output
    assert output.exists()
    assert "not-a-uri" in result.output


def test_server_command_starts_and_reports_bind(monkeypatch):
    import importlib

    monkeypatch.setenv("PYBLASTRADIUS_API_KEY", "test-key")
    import pyblastradius.server as server_module

    importlib.reload(server_module)
    called = {}

    def fake_run(host, port):
        called["host"] = host
        called["port"] = port

    monkeypatch.setattr(server_module, "run_server", fake_run)
    result = CliRunner().invoke(cli, ["server", "--host", "0.0.0.0", "--port", "9999"])

    assert result.exit_code == 0, result.output
    assert called == {"host": "0.0.0.0", "port": 9999}
    assert "Listening on 0.0.0.0:9999" in result.output
