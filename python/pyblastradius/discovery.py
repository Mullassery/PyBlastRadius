"""Discovery engines for finding dependencies in infrastructure."""

from abc import ABC, abstractmethod
from typing import Optional, List, Dict, Any
import json
from pathlib import Path
import ast
import sys
from collections import defaultdict

try:
    from pyblastradius._core import Graph, Node, Edge, NodeType, EdgeType
except ImportError:
    Graph = Edge = NodeType = EdgeType = None
    class Node:
        def __init__(self, id: str, name: str, node_type: str, metadata: Dict = None):
            self.id = id
            self.name = name
            self.type = node_type
            self.metadata = metadata or {}


class Discoverer(ABC):
    """Base class for discovery engines"""

    @abstractmethod
    def discover(self) -> "Graph":
        """Discover dependencies and return graph"""
        pass


class KubernetesDiscoverer(Discoverer):
    """Discover dependencies from Kubernetes cluster"""

    def __init__(self, kubeconfig: Optional[str] = None, context: Optional[str] = None,
                 namespaces: Optional[List[str]] = None):
        self.kubeconfig = kubeconfig or self._default_kubeconfig()
        self.context = context
        self.namespaces = namespaces or ["default"]

    @staticmethod
    def _default_kubeconfig() -> str:
        """Get default kubeconfig path"""
        home = Path.home()
        default = home / ".kube" / "config"
        if default.exists():
            return str(default)
        return ""

    def discover(self) -> Graph:
        """Discover from Kubernetes manifests"""
        graph = Graph()

        if not self.kubeconfig or not Path(self.kubeconfig).exists():
            raise ValueError(f"Kubeconfig not found: {self.kubeconfig}")

        # Simple mock: read manifests from default locations
        # In real implementation, would parse YAML and call K8s API
        manifest_dirs = ["k8s", "kubernetes", "."]

        for manifest_dir in manifest_dirs:
            manifest_path = Path(manifest_dir)
            if manifest_path.exists() and manifest_path.is_dir():
                for yaml_file in manifest_path.glob("**/*.yaml"):
                    try:
                        with open(yaml_file) as f:
                            # Parse YAML and extract services
                            # For now, just create mock nodes
                            pass
                    except Exception:
                        pass

        return graph


class TerraformDiscoverer(Discoverer):
    """Discover dependencies from Terraform"""

    def __init__(self, path: str = ".", backend: str = "local"):
        self.path = Path(path)
        self.backend = backend

    def discover(self) -> Graph:
        """Discover from Terraform files"""
        graph = Graph()

        if not self.path.exists():
            raise ValueError(f"Path not found: {self.path}")

        # Find .tf files and parse
        for tf_file in self.path.glob("**/*.tf"):
            try:
                with open(tf_file) as f:
                    # Parse HCL and extract resources
                    # For now, just create mock nodes
                    pass
            except Exception:
                pass

        return graph


class AirflowDiscoverer(Discoverer):
    """Discover dependencies from Airflow DAGs"""

    def __init__(self, host: str = "localhost", port: int = 8080):
        self.host = host
        self.port = port

    def discover(self) -> Graph:
        """Discover from Airflow DAGs"""
        graph = Graph()

        # Try to connect to Airflow API or parse DAG files
        # For now, return empty graph
        return graph


class OtelDiscoverer(Discoverer):
    """Discover dependencies from OpenTelemetry traces"""

    def __init__(self, endpoint: str = "localhost:4317", duration_minutes: int = 60):
        self.endpoint = endpoint
        self.duration_minutes = duration_minutes

    def discover(self) -> Graph:
        """Discover from OTLP traces"""
        graph = Graph()

        # Connect to OTLP collector and extract traces
        # For now, return empty graph
        return graph


class LocalDiscoverer(Discoverer):
    """Discover Python module imports in a local directory"""

    def __init__(self, root_path: str = ".", namespace_prefix: Optional[str] = None):
        self.root_path = Path(root_path).resolve()
        self.namespace_prefix = namespace_prefix

    def discover(self) -> Graph:
        """Discover Python imports within the codebase"""
        from pyblastradius._core import PyGraph

        graph = PyGraph()

        # Find all Python files
        py_files = list(self.root_path.glob("**/*.py"))
        if not py_files:
            return graph

        # First pass: create Module nodes for all Python files
        module_names = {}  # path -> module name
        for py_file in py_files:
            try:
                rel_path = py_file.relative_to(self.root_path)
                # Convert path to module name (remove .py, convert / to .)
                if rel_path.name == "__init__.py":
                    module_name = str(rel_path.parent).replace("/", ".").replace("\\", ".")
                else:
                    module_name = str(rel_path.with_suffix("")).replace("/", ".").replace("\\", ".")

                if self.namespace_prefix:
                    module_name = f"{self.namespace_prefix}.{module_name}"

                module_names[py_file] = module_name
                graph.add_node(module_name, "Module")
            except Exception:
                pass

        # Second pass: find imports
        for py_file, module_name in module_names.items():
            try:
                with open(py_file, "r", encoding="utf-8") as f:
                    tree = ast.parse(f.read(), str(py_file))

                # Extract imports
                imports = self._extract_imports(tree)

                # Add edges for each imported module that's in our codebase
                for imported_module in imports:
                    target_module = self._resolve_import(imported_module, module_name, module_names)
                    if target_module and target_module in module_names.values():
                        graph.add_edge(module_name, target_module, "Imports")
            except Exception:
                # Skip files that can't be parsed
                pass

        return graph

    def _extract_imports(self, tree: ast.AST) -> List[str]:
        """Extract all module imports from AST"""
        imports = []

        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                # import x, import x.y, import x as z
                for alias in node.names:
                    imports.append(alias.name)
            elif isinstance(node, ast.ImportFrom):
                # from x import y, from .x import y
                if node.module:
                    imports.append(node.module)

        return list(set(imports))  # Deduplicate

    def _resolve_import(self, import_name: str, importing_module: str,
                       module_names: Dict[Path, str]) -> Optional[str]:
        """Try to resolve an import to a local module name"""
        # Handle relative imports
        if import_name.startswith("."):
            parts = importing_module.split(".")
            # Count leading dots
            level = len(import_name) - len(import_name.lstrip("."))
            # Go up level-1 levels
            if level <= len(parts):
                parent = ".".join(parts[:len(parts) - level + 1])
                return parent + "." + import_name.lstrip(".") if import_name.lstrip(".") else parent

        # Handle absolute imports - check if it starts with any known module
        for module_path_name in module_names.values():
            if import_name == module_path_name or import_name.startswith(module_path_name + "."):
                return import_name

        return None


__all__ = [
    "Discoverer",
    "KubernetesDiscoverer",
    "TerraformDiscoverer",
    "AirflowDiscoverer",
    "OtelDiscoverer",
    "LocalDiscoverer",
]
