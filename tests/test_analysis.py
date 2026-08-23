"""End-to-end tests of the real Rust analysis engine via the Python bindings.

Every assertion here is checking output produced by the actual petgraph-
backed Rust implementations in src/analysis/ (blast_radius.rs,
criticality.rs, simulator.rs) through src/python.rs - nothing here is
computed in Python.
"""

import pytest

import pyblastradius as pbr


def test_blast_radius_analyzer_direct_and_indirect_impact(sample_graph):
    graph, ids = sample_graph

    result = pbr.BlastRadiusAnalyzer.analyze(graph, ids["api-server"])

    directly = {graph.get_node(n).name for n in result.directly_impacted}
    indirectly = {graph.get_node(n).name for n in result.indirectly_impacted}

    assert directly == {"auth-service", "redis"}
    assert indirectly == {"postgresql"}
    assert result.total_impacted == 3
    assert result.blast_radius_score > 0
    assert result.source_node == ids["api-server"]


def test_blast_radius_analyzer_leaf_node_has_no_downstream_impact(sample_graph):
    """postgresql has no outgoing "calls" edges, so a blast-radius query
    *from* it (this engine walks outgoing edges) finds nothing - this is
    the correct, if perhaps counter-intuitive, behavior of the underlying
    algorithm for a pure leaf/sink node, and is worth pinning down
    explicitly so a future change to traversal direction shows up as a
    test failure rather than a silent behavior change.
    """
    graph, ids = sample_graph
    result = pbr.BlastRadiusAnalyzer.analyze(graph, ids["postgresql"])
    assert result.total_impacted == 0


def test_blast_radius_analyzer_respects_max_depth(sample_graph):
    graph, ids = sample_graph
    result = pbr.BlastRadiusAnalyzer.analyze(graph, ids["api-server"], max_depth=1)
    # depth=1 should only reach the directly-called services, not postgresql
    assert result.total_impacted == 2
    assert result.indirectly_impacted == []


def test_blast_radius_analyzer_rejects_invalid_node_id(sample_graph):
    graph, _ids = sample_graph
    with pytest.raises(ValueError):
        pbr.BlastRadiusAnalyzer.analyze(graph, "not-a-uuid")


def test_criticality_scorer_flags_single_point_of_failure(sample_graph):
    graph, ids = sample_graph
    scores = pbr.CriticalityScorer.score_all(graph)

    by_name = {s.node_name: s for s in scores}
    assert set(by_name) == {"api-server", "auth-service", "postgresql", "redis"}

    # postgresql is only reachable through auth-service -> it's a SPOF
    assert by_name["postgresql"].is_single_point_of_failure is True

    # scores come back sorted descending by criticality_score
    ordered = [s.criticality_score for s in scores]
    assert ordered == sorted(ordered, reverse=True)


def test_criticality_scorer_spof_detection_on_branching_topology():
    """Regression test for a false-negative SPOF bug: a node was only
    flagged as a single point of failure if its sole predecessor *also* had
    no other outgoing edges. That misses real SPOFs whenever the
    predecessor branches to other nodes too.

    `sample_graph`'s linear chain (api-server -> auth-service ->
    postgresql) doesn't exercise this: auth-service has exactly one
    outgoing edge, so even the buggy implementation got it right there.
    This fixture adds a genuine branch so the predecessor fans out while
    the node under test still has only one way in.

    Topology:
        root -> a -> leaf   (leaf's only path in is through a)
        root -> b
        a    -> b           (b is reachable both via root and via a)
    """
    graph = pbr.Graph()

    root = pbr.Node("root", pbr.NodeType.Service)
    a = pbr.Node("a", pbr.NodeType.Service)
    b = pbr.Node("b", pbr.NodeType.Service)
    leaf = pbr.Node("leaf", pbr.NodeType.Service)

    ids = {
        "root": graph.add_node(root),
        "a": graph.add_node(a),
        "b": graph.add_node(b),
        "leaf": graph.add_node(leaf),
    }

    graph.add_edge(pbr.Edge(pbr.EdgeType.Calls, ids["root"], ids["a"]))
    graph.add_edge(pbr.Edge(pbr.EdgeType.Calls, ids["root"], ids["b"]))
    graph.add_edge(pbr.Edge(pbr.EdgeType.Calls, ids["a"], ids["b"]))
    graph.add_edge(pbr.Edge(pbr.EdgeType.Calls, ids["a"], ids["leaf"]))

    scores = {s.node_name: s for s in pbr.CriticalityScorer.score_all(graph)}

    # `leaf` has exactly one predecessor (a), even though `a` also branches
    # to `b` -> `leaf` is a real SPOF.
    assert scores["leaf"].is_single_point_of_failure is True

    # `b` has two independent predecessors (root and a) -> not a SPOF.
    assert scores["b"].is_single_point_of_failure is False


def test_criticality_scorer_score_node_matches_score_all(sample_graph):
    graph, ids = sample_graph
    single = pbr.CriticalityScorer.score_node(graph, ids["postgresql"])
    all_scores = {s.node_id: s.criticality_score for s in pbr.CriticalityScorer.score_all(graph)}
    assert single.criticality_score == all_scores[ids["postgresql"]]


def test_simulator_outage_produces_recovery_estimate(sample_graph):
    graph, ids = sample_graph
    result = pbr.Simulator.simulate(graph, ids["postgresql"], pbr.FailureType.Outage)

    assert result.source_node == ids["postgresql"]
    assert result.failure_type
    assert result.estimated_recovery_time_minutes >= 0
    assert result.business_impact_score >= 0
    assert "postgresql" in result.affected_services


def test_simulator_different_failure_types_produce_results(sample_graph):
    graph, ids = sample_graph
    for failure_type in (
        pbr.FailureType.Outage,
        pbr.FailureType.Degradation,
        pbr.FailureType.HighLatency,
        pbr.FailureType.HighErrorRate,
        pbr.FailureType.DataQualityIssue,
    ):
        result = pbr.Simulator.simulate(graph, ids["api-server"], failure_type)
        assert result.estimated_impact_nodes >= 1
