"""PyBlastRadius CLI - Command-line interface for blast radius analysis."""

import json
import sys
from pathlib import Path
from typing import Optional, List, Dict, Any
import click
from rich.console import Console
from rich.table import Table
from rich.progress import Progress

from . import (
    KubernetesDiscoverer,
    TerraformDiscoverer,
    AirflowDiscoverer,
    OtelDiscoverer,
    LocalDiscoverer,
    BlastRadiusAnalyzer,
    CriticalityScorer,
)
from .discovery import Discoverer
from .trees import build_tree, TreeType, render_json
from pyblastradius._core import PyGraph

console = Console()


class UriParser:
    """Parse discovery URIs: scheme://key=value,key=value"""

    @staticmethod
    def parse(uri: str) -> tuple[str, Dict[str, str]]:
        """Parse URI into scheme and parameters"""
        if "://" not in uri:
            raise ValueError(f"Invalid URI format: {uri}")

        scheme, params_str = uri.split("://", 1)
        params = {}

        if params_str:
            for param in params_str.split(","):
                if "=" not in param:
                    raise ValueError(f"Invalid parameter format: {param}")
                key, value = param.split("=", 1)
                params[key.strip()] = value.strip()

        return scheme, params

    @staticmethod
    def create_discoverer(scheme: str, params: Dict[str, str]) -> Discoverer:
        """Create discoverer from scheme and parameters"""
        if scheme == "kubernetes":
            return KubernetesDiscoverer(
                kubeconfig=params.get("kubeconfig"),
                context=params.get("context"),
                namespaces=params.get("namespaces", "").split(";") if params.get("namespaces") else None,
            )
        elif scheme == "terraform":
            return TerraformDiscoverer(
                path=params.get("path", "."),
                backend=params.get("backend", "local"),
            )
        elif scheme == "airflow":
            return AirflowDiscoverer(
                host=params.get("host", "localhost"),
                port=int(params.get("port", "8080")),
            )
        elif scheme == "otlp":
            return OtelDiscoverer(
                endpoint=params.get("endpoint", "localhost:4317"),
                duration_minutes=int(params.get("duration", "60")),
            )
        elif scheme == "local":
            return LocalDiscoverer(
                root_path=params.get("path", "."),
                namespace_prefix=params.get("namespace"),
            )
        else:
            raise ValueError(f"Unknown discovery scheme: {scheme}")


@click.group()
def cli():
    """PyBlastRadius - Operational Blast Radius Intelligence"""
    pass


@cli.command()
@click.argument("sources", nargs=-1, required=False)
@click.option("--output", "-o", default="blast-radius.json", help="Output file")
@click.option("--verbose", "-v", is_flag=True, help="Verbose output")
def scan(sources: tuple, output: str, verbose: bool):
    """Discover dependencies from infrastructure"""
    try:
        graph = Graph()

        if not sources:
            # Auto-discover
            console.print("🔍 Auto-discovering resources in current directory...")
            discoverer = LocalDiscoverer()
            graph = discoverer.discover()
        else:
            # Explicit sources
            with Progress() as progress:
                task = progress.add_task("[cyan]Discovering...", total=len(sources))

                for source_uri in sources:
                    try:
                        scheme, params = UriParser.parse(source_uri)
                        discoverer = UriParser.create_discoverer(scheme, params)
                        sub_graph = discoverer.discover()
                        graph = Graph.merge([graph, sub_graph])

                        if verbose:
                            console.print(f"  ✓ {scheme}: {sub_graph.node_count()} nodes")
                    except Exception as e:
                        console.print(f"  ✗ {source_uri}: {str(e)}", style="red")

                    progress.advance(task)

        # Save graph
        with open(output, "w") as f:
            f.write(graph.to_json())

        console.print(f"\n✅ Discovery complete: {graph.node_count()} nodes")
        console.print(f"📁 Saved to: {output}")
        console.print(f"\n💡 Next: pyblastradius analyze {output}")

    except Exception as e:
        console.print(f"\n❌ Error: {str(e)}", style="red")
        if verbose:
            import traceback
            traceback.print_exc()
        sys.exit(1)


@cli.command()
@click.argument("graph_file", type=click.Path(exists=True))
@click.option("--type", "-t", required=True, type=click.Choice(["call", "reverse_call", "import", "database", "blast_radius"]),
              help="Tree type")
@click.option("--root", "-r", required=True, help="Root node name")
@click.option("--max-depth", "-d", default=5, type=int, help="Max depth")
@click.option("--format", "-f", default="json", type=click.Choice(["json", "rich"]))
@click.option("--output", "-o", default=None, help="Output file (default: stdout)")
def tree(graph_file: str, type: str, root: str, max_depth: int, format: str, output: Optional[str]):
    """Build and render dependency trees"""
    try:
        # Load graph from JSON file
        with open(graph_file) as f:
            graph_data = json.load(f)

        # Create a PyGraph and populate it
        graph = PyGraph()
        for node_data in graph_data.get("nodes", []):
            graph.add_node(node_data["name"], node_data.get("type", "Service"))

        for edge_data in graph_data.get("edges", []):
            try:
                graph.add_edge(edge_data["source"], edge_data["target"], edge_data.get("type", "Calls"))
            except Exception:
                pass  # Skip edges where source/target doesn't exist

        # Build the tree
        if type == "blast_radius":
            from pyblastradius.trees import build_blast_radius_tree
            tree_node = build_blast_radius_tree(graph, root, max_depth=max_depth)
        else:
            tree_type_map = {
                "call": TreeType.CALL,
                "reverse_call": TreeType.REVERSE_CALL,
                "import": TreeType.IMPORT,
                "database": TreeType.DATABASE,
            }
            tree_type = tree_type_map[type]
            tree_node = build_tree(graph, tree_type, root, max_depth=max_depth)

        if not tree_node:
            console.print(f"❌ Root node '{root}' not found", style="red")
            sys.exit(1)

        # Render
        if format == "json":
            result = render_json(tree_node)
            if output:
                with open(output, "w") as f:
                    f.write(result)
                console.print(f"✅ Tree saved to: {output}")
            else:
                click.echo(result)
        elif format == "rich":
            from pyblastradius.render.rich_renderer import print_tree
            if output:
                # Save as JSON when output file specified
                result = render_json(tree_node)
                with open(output, "w") as f:
                    f.write(result)
                console.print(f"✅ Tree saved to: {output}")
            else:
                print_tree(tree_node, console)

    except Exception as e:
        console.print(f"❌ Error: {str(e)}", style="red")
        if False:  # Set to True for debugging
            import traceback
            traceback.print_exc()
        sys.exit(1)


@cli.command()
@click.argument("graph_file", type=click.Path(exists=True))
@click.option("--service", "-s", required=True, help="Target service")
@click.option("--max-depth", "-d", default=3, type=int, help="Max depth")
@click.option("--format", "-f", default="table", type=click.Choice(["table", "json", "csv"]))
def analyze(graph_file: str, service: str, max_depth: int, format: str):
    """Analyze blast radius for a service"""
    try:
        # Load graph
        with open(graph_file) as f:
            graph = Graph.from_json(f.read())

        # Run analysis
        analyzer = BlastRadiusAnalyzer(graph)
        result = analyzer.analyze(service, max_depth=max_depth)

        # Output
        if format == "json":
            click.echo(json.dumps(result.to_dict(), indent=2))
        elif format == "csv":
            # CSV output
            click.echo(f"service,score,directly_impacted,indirectly_impacted")
            click.echo(f"{service},{result.score:.2f},{len(result.directly_impacted)},{len(result.indirectly_impacted)}")
        else:
            # Table output
            table = Table(title=f"Blast Radius Analysis: {service}")
            table.add_column("Metric", style="cyan")
            table.add_column("Value", style="green")

            table.add_row("Blast Radius Score", f"{result.score:.1%}")
            table.add_row("Risk Level", get_risk_level(result.score))
            table.add_row("Directly Impacted", str(len(result.directly_impacted)))
            table.add_row("Indirectly Impacted", str(len(result.indirectly_impacted)))

            console.print(table)

            if result.directly_impacted:
                console.print("\n[bold]Directly Impacted Services:[/bold]")
                for node in result.directly_impacted[:5]:
                    console.print(f"  • {node}")

    except Exception as e:
        console.print(f"❌ Error: {str(e)}", style="red")
        sys.exit(1)


@cli.command()
@click.argument("graph_file", type=click.Path(exists=True))
@click.option("--limit", "-l", default=10, type=int, help="Number of services to show")
@click.option("--format", "-f", default="table", type=click.Choice(["table", "json"]))
def criticality(graph_file: str, limit: int, format: str):
    """Rank services by criticality"""
    try:
        # Load graph
        with open(graph_file) as f:
            graph = Graph.from_json(f.read())

        # Score
        scorer = CriticalityScorer(graph)
        scores = [(node, scorer.score(node)) for node in graph.services()]
        scores.sort(key=lambda x: x[1], reverse=True)

        # Output
        if format == "json":
            data = [{"service": node, "score": score} for node, score in scores[:limit]]
            click.echo(json.dumps(data, indent=2))
        else:
            table = Table(title="Service Criticality Ranking")
            table.add_column("Service", style="cyan")
            table.add_column("Criticality Score", style="green")
            table.add_column("Risk Level", style="yellow")

            for node, score in scores[:limit]:
                table.add_row(node, f"{score:.1%}", get_risk_level(score))

            console.print(table)

    except Exception as e:
        console.print(f"❌ Error: {str(e)}", style="red")
        sys.exit(1)


@cli.command()
@click.argument("graph_file", type=click.Path(exists=True))
@click.option("--target", "-t", required=True, help="Service to fail")
@click.option("--scenario", "-s", default="cascade", help="Failure scenario")
def simulate(graph_file: str, target: str, scenario: str):
    """Simulate service failure and cascade"""
    try:
        # Load graph
        with open(graph_file) as f:
            graph = Graph.from_json(f.read())

        console.print(f"🔴 Simulating {target} failure...")
        console.print(f"📊 Scenario: {scenario}")

        # Simple cascade simulation
        impacted = set([target])
        queue = [target]

        while queue:
            current = queue.pop(0)
            # Find dependent services
            for edge in graph.edges():
                if edge.source == current and edge.target not in impacted:
                    impacted.add(edge.target)
                    queue.append(edge.target)

        console.print(f"\n⚠️  Impacted Services: {len(impacted) - 1}")
        for service in sorted(impacted):
            if service != target:
                console.print(f"  • {service}")

        console.print(f"\n⏱️  Estimated Recovery Time: 45-120 minutes")

    except Exception as e:
        console.print(f"❌ Error: {str(e)}", style="red")
        sys.exit(1)


@cli.command()
@click.argument("graph_file", type=click.Path(exists=True))
@click.option("--output", "-o", default="blast-radius.html", help="Output file")
@click.option("--format", "-f", default="interactive", type=click.Choice(["interactive", "dot", "json"]))
def visualize(graph_file: str, output: str, format: str):
    """Generate graph visualization"""
    try:
        # Load graph
        with open(graph_file) as f:
            graph = Graph.from_json(f.read())

        console.print(f"📊 Generating {format} visualization...")

        if format == "json":
            with open(output, "w") as f:
                f.write(graph.to_json())
        elif format == "dot":
            dot = graph.to_dot()
            with open(output, "w") as f:
                f.write(dot)
        else:
            # Generate interactive HTML
            html = generate_interactive_html(graph)
            with open(output, "w") as f:
                f.write(html)

        console.print(f"✅ Visualization saved to: {output}")

    except Exception as e:
        console.print(f"❌ Error: {str(e)}", style="red")
        sys.exit(1)


def get_risk_level(score: float) -> str:
    """Get risk level from score"""
    if score < 0.2:
        return "🟢 Low"
    elif score < 0.5:
        return "🟡 Medium"
    elif score < 0.8:
        return "🟠 High"
    else:
        return "🔴 Critical"


def generate_interactive_html(graph: PyGraph) -> str:
    """Generate interactive HTML visualization"""
    # Simple HTML with graph data
    nodes = [{"id": node, "label": node} for node in graph.list_nodes()]
    edges = []  # TODO: expose edges from PyGraph

    return f"""<!DOCTYPE html>
<html>
<head>
    <title>Blast Radius Graph</title>
    <style>
        body {{ font-family: sans-serif; margin: 20px; }}
        #graph {{ border: 1px solid #ccc; width: 100%; height: 600px; }}
        .info {{ margin-top: 20px; }}
    </style>
</head>
<body>
    <h1>Blast Radius Graph</h1>
    <p>Services: {len(nodes)} | Dependencies: {len(edges)}</p>
    <div id="graph"></div>
    <div class="info">
        <h2>Graph Data</h2>
        <pre>{json.dumps({"nodes": nodes, "edges": edges}, indent=2)}</pre>
    </div>
</body>
</html>"""


@cli.command()
@click.option("--host", "-h", default="127.0.0.1", help="Server host")
@click.option("--port", "-p", default=8080, type=int, help="Server port")
def server(host: str, port: int):
    """Start REST API server"""
    try:
        from pyblastradius.server import run_server
        console.print(f"🚀 Starting PyBlastRadius server...")
        console.print(f"📡 Listening on {host}:{port}")
        console.print(f"📚 API docs: http://{host}:{port}/docs")
        run_server(host, port)
    except ImportError:
        console.print(
            "❌ FastAPI is required for REST API. Install with:\n"
            "   pip install fastapi uvicorn",
            style="red"
        )
        sys.exit(1)
    except Exception as e:
        console.print(f"❌ Error: {str(e)}", style="red")
        sys.exit(1)


def main():
    """Entry point"""
    cli()


if __name__ == "__main__":
    main()
