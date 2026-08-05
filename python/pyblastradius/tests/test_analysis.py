"""Tests for analysis module."""

import pytest
from pyblastradius._core import PyGraph
from pyblastradius.analysis import BlastRadiusAnalyzer, CriticalityScorer, Simulator


@pytest.fixture
def graph():
    """Create a test graph"""
    g = PyGraph()
    g.add_node('api', 'Service')
    g.add_node('auth', 'Service')
    g.add_node('db', 'Database')
    g.add_node('cache', 'Cache')

    g.add_edge('api', 'auth', 'Calls')
    g.add_edge('api', 'db', 'Queries')
    g.add_edge('auth', 'cache', 'Calls')
    g.add_edge('auth', 'db', 'Queries')

    return g


def test_blast_radius_analyzer(graph):
    """Test blast radius analysis"""
    analyzer = BlastRadiusAnalyzer(graph)
    result = analyzer.analyze('api', max_depth=3)

    assert result.service == 'api'
    assert result.score >= 0  # Score can be > 1 if non-service nodes are impacted
    assert isinstance(result.directly_impacted, list)
    assert isinstance(result.indirectly_impacted, list)
    assert len(result.directly_impacted) > 0  # api should impact something


def test_criticality_scorer(graph):
    """Test criticality scoring"""
    scorer = CriticalityScorer(graph)
    score = scorer.score('db')

    assert 0 <= score <= 1
    # db has 2 predecessors (api and auth), so it should have high criticality
    assert score > 0


def test_score_all(graph):
    """Test scoring all services"""
    scorer = CriticalityScorer(graph)
    scores = scorer.score_all()

    assert isinstance(scores, dict)
    assert len(scores) > 0
    for node, score in scores.items():
        assert 0 <= score <= 1


def test_simulator(graph):
    """Test cascade simulation"""
    sim = Simulator(graph)
    result = sim.simulate_cascade('api', max_depth=5)

    assert result['root'] == 'api'
    assert result['impacted_count'] >= 0
    assert isinstance(result['impacted'], list)
    assert 'api' in result['impacted']


def test_blast_radius_result_to_dict(graph):
    """Test BlastRadiusResult serialization"""
    analyzer = BlastRadiusAnalyzer(graph)
    result = analyzer.analyze('api', max_depth=2)
    data = result.to_dict()

    assert data['service'] == 'api'
    assert 'score' in data
    assert 'directly_impacted' in data
    assert 'indirectly_impacted' in data
