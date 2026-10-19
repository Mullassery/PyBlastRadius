"""OpenLineage event conversion tests (no external collector)."""

from __future__ import annotations

import json

from pyblastradius.openlineage import discover_openlineage, events_to_graph, load_events

EVENT = {
    "eventType": "COMPLETE",
    "job": {"namespace": "airflow", "name": "etl_orders"},
    "inputs": [{"namespace": "snowflake", "name": "raw.orders"}],
    "outputs": [{"namespace": "snowflake", "name": "analytics.orders"}],
    "run": {"runId": "run-1"},
}


def test_event_builds_dataset_to_job_graph():
    graph = events_to_graph([EVENT])
    assert graph.node_count() == 3
    assert graph.edge_count() == 2
    names = {node.name for node in graph.nodes()}
    assert names == {
        "airflow:etl_orders",
        "snowflake:raw.orders",
        "snowflake:analytics.orders",
    }


def test_multiple_events_merge_and_dedupe():
    second = dict(EVENT)
    second["outputs"] = [{"namespace": "snowflake", "name": "analytics.orders_v2"}]
    graph = events_to_graph([EVENT, second])
    assert graph.node_count() == 4  # job + 1 input + 2 outputs
    assert graph.edge_count() == 3


def test_load_json_array_and_ndjson(tmp_path):
    array = tmp_path / "events.json"
    array.write_text(json.dumps([EVENT]))
    assert len(load_events(array)) == 1

    ndjson = tmp_path / "events.ndjson"
    ndjson.write_text(json.dumps(EVENT) + "\n" + json.dumps(EVENT) + "\n")
    assert len(load_events(ndjson)) == 2


def test_discover_file(tmp_path):
    path = tmp_path / "lineage.json"
    path.write_text(json.dumps(EVENT))
    graph = discover_openlineage(path)
    assert graph.node_count() == 3
