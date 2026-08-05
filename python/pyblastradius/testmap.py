"""Discover and map test coverage for Python projects."""

import ast
from pathlib import Path
from typing import Dict, List, Optional, Set, Tuple
from pyblastradius._core import PyGraph


class TestMapper:
    """Map tests to the modules they cover using AST analysis."""

    def __init__(self, test_dir: str = "tests", src_dir: str = ".", namespace_prefix: str = ""):
        self.test_dir = Path(test_dir)
        self.src_dir = Path(src_dir)
        self.namespace_prefix = namespace_prefix

    def discover_tests(self) -> PyGraph:
        """
        Discover tests and build a graph with Test nodes and Tests edges.

        Returns:
            PyGraph with Test nodes and Tests edges to covered modules
        """
        graph = PyGraph()

        if not self.test_dir.exists():
            return graph

        # Find all test files
        test_files = list(self.test_dir.glob("**/test_*.py")) + \
                    list(self.test_dir.glob("**/*_test.py"))

        if not test_files:
            return graph

        # Parse tests and find what they import/test
        test_to_targets = {}  # test_name -> set of target modules

        for test_file in test_files:
            try:
                test_name = self._path_to_module_name(test_file, is_test=True)

                # Add test node
                graph.add_node(test_name, "Test")

                # Find imports in test file
                with open(test_file, "r", encoding="utf-8") as f:
                    tree = ast.parse(f.read(), str(test_file))

                # Extract imports
                targets = self._extract_tested_modules(tree)
                test_to_targets[test_name] = targets

                # Add edges for each target
                for target in targets:
                    try:
                        graph.add_node(target, "Module")
                        graph.add_edge(test_name, target, "Tests")
                    except Exception:
                        pass

            except Exception:
                # Skip files that can't be parsed
                pass

        return graph

    def _path_to_module_name(self, path: Path, is_test: bool = False) -> str:
        """Convert file path to module name"""
        try:
            rel_path = path.relative_to(self.test_dir if is_test else self.src_dir)
        except ValueError:
            rel_path = path

        if path.name == "__init__.py":
            module_name = str(rel_path.parent).replace("/", ".").replace("\\", ".")
        else:
            module_name = str(rel_path.with_suffix("")).replace("/", ".").replace("\\", ".")

        if self.namespace_prefix:
            module_name = f"{self.namespace_prefix}.{module_name}"

        return module_name

    def _extract_tested_modules(self, tree: ast.AST) -> Set[str]:
        """Extract modules that are tested by analyzing imports and test patterns"""
        targets = set()

        for node in ast.walk(tree):
            # Collect imports
            if isinstance(node, ast.Import):
                for alias in node.names:
                    targets.add(alias.name)
            elif isinstance(node, ast.ImportFrom):
                if node.module:
                    targets.add(node.module)

            # Look for test functions that might indicate what's being tested
            # Pattern: test_<module_name> or test_<ClassName>
            if isinstance(node, ast.FunctionDef) and node.name.startswith("test_"):
                # Extract the tested name from function name
                tested_name = node.name[5:]  # Remove 'test_' prefix
                if tested_name:
                    targets.add(tested_name)

        return targets

    def build_coverage_graph(self, full_graph: Optional[PyGraph] = None) -> PyGraph:
        """
        Build a combined graph with both source modules and tests.

        Args:
            full_graph: Optional full project graph to merge with test mapping

        Returns:
            PyGraph with source modules, test nodes, and Tests edges
        """
        test_graph = self.discover_tests()

        if full_graph and hasattr(full_graph, 'merge'):
            # Merge test graph into full graph
            # Note: PyGraph doesn't expose merge yet, so we return test_graph for now
            pass

        return test_graph


def create_test_coverage_graph(
    test_dir: str = "tests",
    src_dir: str = ".",
    full_graph: Optional[PyGraph] = None,
) -> PyGraph:
    """
    Convenience function to discover tests and create coverage graph.

    Args:
        test_dir: Directory containing tests
        src_dir: Source directory
        full_graph: Optional full project graph to merge with

    Returns:
        PyGraph with test coverage information
    """
    mapper = TestMapper(test_dir, src_dir)
    return mapper.build_coverage_graph(full_graph)
