"""Tree models and builders for dependency visualization."""

from dataclasses import dataclass, field, asdict
from typing import Dict, List, Optional, Set, Callable
from enum import Enum
from collections import deque
import json


class TreeType(Enum):
    """Types of dependency trees"""
    CALL = "call"
    REVERSE_CALL = "reverse_call"
    IMPORT = "import"
    BLAST_RADIUS = "blast_radius"
    DATABASE = "database"
    TEST_COVERAGE = "test_coverage"


# Map tree types to edge types they traverse
TREE_EDGE_FILTERS: Dict[TreeType, Optional[Set[str]]] = {
    TreeType.CALL: {"Calls"},
    TreeType.REVERSE_CALL: {"Calls"},
    TreeType.IMPORT: {"Imports"},
    TreeType.BLAST_RADIUS: None,  # All edge types, handled specially
    TreeType.DATABASE: {"Queries"},
    TreeType.TEST_COVERAGE: {"Tests"},
}

# Default direction for each tree type
TREE_DEFAULT_DIRECTION: Dict[TreeType, str] = {
    TreeType.CALL: "forward",
    TreeType.REVERSE_CALL: "reverse",
    TreeType.IMPORT: "forward",
    TreeType.BLAST_RADIUS: "reverse",
    TreeType.DATABASE: "forward",
    TreeType.TEST_COVERAGE: "reverse",  # "what tests cover this module"
}


@dataclass
class TreeNode:
    """A node in a dependency tree"""
    id: str  # node name
    label: str = ""  # display name, defaults to id
    node_type: str = ""  # "Service", "Database", "Module", etc.
    via_edge_type: Optional[str] = None  # edge type that led here from parent
    depth: int = 0
    is_cycle: bool = False  # terminal leaf if True
    metadata: Dict = field(default_factory=dict)
    children: List["TreeNode"] = field(default_factory=list)

    def __post_init__(self):
        if not self.label:
            self.label = self.id

    def to_dict(self) -> Dict:
        """Recursively convert to dictionary"""
        return {
            "id": self.id,
            "label": self.label,
            "node_type": self.node_type,
            "via_edge_type": self.via_edge_type,
            "depth": self.depth,
            "is_cycle": self.is_cycle,
            "metadata": self.metadata,
            "children": [child.to_dict() for child in self.children],
        }

    def flatten(self) -> List["TreeNode"]:
        """Flatten tree to list (depth-first)"""
        result = [self]
        for child in self.children:
            result.extend(child.flatten())
        return result


def build_tree(
    graph,
    tree_type: TreeType,
    root: str,
    max_depth: int = 5,
    direction: str = "forward",
) -> Optional[TreeNode]:
    """
    Build a dependency tree by walking the graph.

    Args:
        graph: PyGraph instance
        tree_type: Type of tree to build (CALL, IMPORT, DATABASE, etc.)
        root: Root node name
        max_depth: Maximum depth to traverse
        direction: "forward" (follow edges out) or "reverse" (follow edges in)

    Returns:
        TreeNode at root, or None if root not found
    """
    try:
        root_type = graph.get_node_type(root)
    except Exception:
        return None

    # Track current path for cycle detection
    root_node = TreeNode(
        id=root,
        label=root,
        node_type=root_type,
        depth=0,
        via_edge_type=None,
    )

    # Special handling for reverse_call trees
    if tree_type == TreeType.REVERSE_CALL and direction == "forward":
        direction = "reverse"

    _walk_tree(
        graph=graph,
        node=root_node,
        tree_type=tree_type,
        max_depth=max_depth,
        current_path={root},
        direction=direction,
    )

    return root_node


def _walk_tree(
    graph,
    node: TreeNode,
    tree_type: TreeType,
    max_depth: int,
    current_path: Set[str],
    direction: str = "forward",
) -> None:
    """
    Recursively walk graph and build tree, detecting cycles via current path.
    """
    if node.depth >= max_depth:
        return

    next_depth = node.depth + 1
    edge_filter = TREE_EDGE_FILTERS.get(tree_type)

    # Determine which nodes to visit next
    next_nodes_to_visit = []

    if direction == "forward":
        # Follow edges outgoing from current node
        try:
            neighbors = graph.get_neighbors(node.id)
            for neighbor in neighbors:
                next_nodes_to_visit.append(neighbor)
        except Exception:
            pass
    else:  # reverse
        # Follow edges incoming to current node (predecessors)
        try:
            preds = graph.get_predecessors(node.id)
            for pred in preds:
                next_nodes_to_visit.append(pred)
        except Exception:
            pass

    # Filter by edge type if specified
    # (For now, simple implementation - ideally would check actual edge types)
    for next_node_id in next_nodes_to_visit:
        if next_node_id in current_path:
            # Cycle detected - add as terminal leaf
            try:
                next_type = graph.get_node_type(next_node_id)
            except Exception:
                next_type = ""

            child = TreeNode(
                id=next_node_id,
                label=next_node_id,
                node_type=next_type,
                depth=next_depth,
                is_cycle=True,
            )
            node.children.append(child)
        else:
            # New node - recurse
            try:
                next_type = graph.get_node_type(next_node_id)
            except Exception:
                next_type = ""

            child = TreeNode(
                id=next_node_id,
                label=next_node_id,
                node_type=next_type,
                depth=next_depth,
                is_cycle=False,
            )
            node.children.append(child)

            # Recurse with updated path
            new_path = current_path | {next_node_id}
            _walk_tree(
                graph=graph,
                node=child,
                tree_type=tree_type,
                max_depth=max_depth,
                current_path=new_path,
                direction=direction,
            )


def build_blast_radius_tree(
    graph,
    root: str,
    max_depth: int = 5,
) -> Optional[TreeNode]:
    """
    Build a blast radius tree showing impact cascade.
    Traverses all edge types in the backward direction (predecessors).
    """
    from pyblastradius.analysis import BlastRadiusAnalyzer

    try:
        root_type = graph.get_node_type(root)
    except Exception:
        return None

    # Get blast radius analysis
    analyzer = BlastRadiusAnalyzer(graph)
    result = analyzer.analyze(root, max_depth=max_depth)

    # Create root node
    root_node = TreeNode(
        id=root,
        label=root,
        node_type=root_type,
        depth=0,
        via_edge_type=None,
        metadata={
            "blast_radius_score": result.score,
            "directly_impacted": len(result.directly_impacted),
            "indirectly_impacted": len(result.indirectly_impacted),
        },
    )

    # Build tree from blast radius results
    visited = {root}

    # Add directly impacted nodes
    for impacted in result.directly_impacted:
        if impacted not in visited and len(root_node.children) < max_depth:
            visited.add(impacted)
            try:
                impacted_type = graph.get_node_type(impacted)
            except Exception:
                impacted_type = ""

            child = TreeNode(
                id=impacted,
                label=impacted,
                node_type=impacted_type,
                depth=1,
                is_cycle=False,
            )
            root_node.children.append(child)

            # Add indirectly impacted as children of directly impacted
            for secondary in result.indirectly_impacted:
                if secondary not in visited and len(child.children) < 3:
                    visited.add(secondary)
                    try:
                        secondary_type = graph.get_node_type(secondary)
                    except Exception:
                        secondary_type = ""

                    grandchild = TreeNode(
                        id=secondary,
                        label=secondary,
                        node_type=secondary_type,
                        depth=2,
                        is_cycle=False,
                    )
                    child.children.append(grandchild)

    return root_node


def render_json(tree: TreeNode, indent: int = 2) -> str:
    """Render tree as JSON string"""
    return json.dumps(tree.to_dict(), indent=indent)

