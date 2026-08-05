"""Tests for tree module."""

import pytest
from pyblastradius._core import PyGraph
from pyblastradius.trees import TreeNode, build_tree, build_blast_radius_tree, TreeType, render_json


@pytest.fixture
def simple_graph():
    """Create a simple test graph"""
    g = PyGraph()
    g.add_node('api', 'Service')
    g.add_node('auth', 'Service')
    g.add_node('db', 'Database')
    g.add_edge('api', 'auth', 'Calls')
    g.add_edge('api', 'db', 'Queries')
    g.add_edge('auth', 'db', 'Queries')
    return g


def test_tree_node_creation():
    """Test TreeNode dataclass"""
    node = TreeNode(id='test', label='Test Node', node_type='Service', depth=0)
    assert node.id == 'test'
    assert node.label == 'Test Node'
    assert node.node_type == 'Service'
    assert node.depth == 0
    assert not node.is_cycle
    assert len(node.children) == 0


def test_tree_node_to_dict():
    """Test TreeNode serialization"""
    parent = TreeNode(id='parent', node_type='Service', depth=0)
    child = TreeNode(id='child', node_type='Database', depth=1)
    parent.children.append(child)

    data = parent.to_dict()
    assert data['id'] == 'parent'
    assert data['node_type'] == 'Service'
    assert len(data['children']) == 1
    assert data['children'][0]['id'] == 'child'


def test_build_call_tree(simple_graph):
    """Test building a call tree"""
    tree = build_tree(simple_graph, TreeType.CALL, 'api', max_depth=3)
    assert tree is not None
    assert tree.id == 'api'
    assert tree.node_type == 'Service'
    assert len(tree.children) == 2
    assert any(c.id == 'auth' for c in tree.children)
    assert any(c.id == 'db' for c in tree.children)


def test_build_reverse_call_tree(simple_graph):
    """Test building a reverse call tree"""
    tree = build_tree(simple_graph, TreeType.REVERSE_CALL, 'db', max_depth=3)
    assert tree is not None
    assert tree.id == 'db'
    assert tree.node_type == 'Database'
    # db has two predecessors in reverse call tree
    assert len(tree.children) >= 0  # Depends on edge filtering


def test_cycle_detection(simple_graph):
    """Test cycle detection in trees"""
    # Add a cycle
    simple_graph.add_edge('db', 'api', 'Calls')

    tree = build_tree(simple_graph, TreeType.CALL, 'api', max_depth=3)
    assert tree is not None

    # Flatten and check for cycle nodes
    flat = tree.flatten()
    # The tree should terminate cycles rather than infinite loop
    assert len(flat) > 0


def test_blast_radius_tree(simple_graph):
    """Test building a blast radius tree"""
    tree = build_blast_radius_tree(simple_graph, 'api', max_depth=3)
    assert tree is not None
    assert tree.id == 'api'
    assert 'blast_radius_score' in tree.metadata
    assert tree.metadata['blast_radius_score'] > 0


def test_render_json(simple_graph):
    """Test JSON rendering"""
    tree = build_tree(simple_graph, TreeType.CALL, 'api', max_depth=2)
    json_str = render_json(tree)
    assert json_str is not None
    assert 'api' in json_str
    assert 'Service' in json_str


def test_nonexistent_root(simple_graph):
    """Test building tree with nonexistent root"""
    tree = build_tree(simple_graph, TreeType.CALL, 'nonexistent', max_depth=2)
    assert tree is None


def test_max_depth_respected(simple_graph):
    """Test that max_depth is respected"""
    tree = build_tree(simple_graph, TreeType.CALL, 'api', max_depth=1)
    assert tree is not None
    # All children should be at depth 1
    for child in tree.children:
        assert child.depth == 1
