"""Rich terminal renderer for dependency trees."""

from rich.tree import Tree
from rich.console import Console
from pyblastradius.trees import TreeNode


# Color mapping by node type
NODE_TYPE_COLORS = {
    "Service": "cyan",
    "Database": "magenta",
    "Module": "blue",
    "Pipeline": "yellow",
    "Topic": "yellow",
    "Queue": "yellow",
    "Cache": "yellow",
    "VectorDb": "magenta",
    "Dashboard": "green",
    "Agent": "cyan",
    "ExternalService": "red",
    "Table": "magenta",
    "Test": "green",
}


def render_rich(tree: TreeNode) -> Tree:
    """Render a tree as a Rich Tree object"""
    color = NODE_TYPE_COLORS.get(tree.node_type, "white")

    label = f"[{color}]{tree.label}[/{color}]"
    if tree.node_type:
        label += f" [{color}dim]({tree.node_type})[/{color}dim]"
    if tree.is_cycle:
        label += " [yellow]↺ (cycle)[/yellow]"

    rich_tree = Tree(label)

    # Add children
    for child in tree.children:
        _add_child_to_rich_tree(rich_tree, child)

    return rich_tree


def _add_child_to_rich_tree(parent: Tree, child: TreeNode) -> None:
    """Recursively add children to Rich tree"""
    color = NODE_TYPE_COLORS.get(child.node_type, "white")

    label = f"[{color}]{child.label}[/{color}]"
    if child.node_type:
        label += f" [{color}dim]({child.node_type})[/{color}dim]"
    if child.is_cycle:
        label += " [yellow]↺ (cycle)[/yellow]"

    child_tree = parent.add(label)

    for grandchild in child.children:
        _add_child_to_rich_tree(child_tree, grandchild)


def print_tree(tree: TreeNode, console: Console = None) -> None:
    """Print tree to console using Rich"""
    if console is None:
        console = Console()

    rich_tree = render_rich(tree)
    console.print(rich_tree)
