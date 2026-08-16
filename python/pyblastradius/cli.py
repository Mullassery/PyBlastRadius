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
    BlastRadiusAnalyzer,
    CriticalityScorer,
    Simulator,
    FailureType,
    Graph,
)
from .discovery import Discoverer

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
                path=params.get("path", "."),
                recursive=params.get("recursive", "true").lower() == "true",
            )
        else:
            raise ValueError(f"Unknown discovery scheme: {scheme}")


class LocalDiscoverer(Discoverer):
    """Discover from local filesystem"""

    def __init__(self, path: str = ".", recursive: bool = True):
        self.path = Path(path)
        self.recursive = recursive

    def discover(self) -> Graph:
        """Auto-discover dependencies in local directory"""
        graph = Graph()

        # Try each discoverer
        discoverers = [
            ("Kubernetes", KubernetesDiscoverer()),
            ("Terraform", TerraformDiscoverer(path=str(self.path))),
            ("Airflow", AirflowDiscoverer()),
        ]

        found_any = False
        for name, discoverer in discoverers:
            try:
                sub_graph = discoverer.discover()
                if sub_graph.node_count() > 0:
                    graph.merge(sub_graph)
                    found_any = True
                    console.print(f"  ✓ {name}: {sub_graph.node_count()} nodes")
            except Exception as e:
                console.print(f"  ✗ {name}: {str(e)}", style="dim")

        return graph


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
                        graph.merge(sub_graph)

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
@click.option("--service", "-s", required=True, help="Target service")
@click.option("--max-depth", "-d", default=3, type=int, help="Max depth")
@click.option("--format", "-f", default="table", type=click.Choice(["table", "json", "csv"]))
def analyze(graph_file: str, service: str, max_depth: int, format: str):
    """Analyze blast radius for a service"""
    try:
        # Load graph
        with open(graph_file) as f:
            graph = Graph.from_json(f.read())

        # Resolve the human-readable --service name to the node id the
        # Rust engine keys on.
        node = graph.find_node_by_name(service)
        if node is None:
            raise ValueError(f"No node named '{service}' found in {graph_file}")

        # Run analysis (real Rust engine, via pyblastradius._core)
        result = BlastRadiusAnalyzer.analyze(graph, node.id, max_depth=max_depth)
        score = result.blast_radius_score / 100.0

        def resolve_names(node_ids):
            names = []
            for nid in node_ids:
                n = graph.get_node(nid)
                names.append(n.name if n else nid)
            return names

        directly_impacted = resolve_names(result.directly_impacted)
        indirectly_impacted = resolve_names(result.indirectly_impacted + result.tertiary_impacted)

        # Output
        if format == "json":
            click.echo(json.dumps({
                "service": service,
                "score": score,
                "directly_impacted": directly_impacted,
                "indirectly_impacted": indirectly_impacted,
                "total_impacted": result.total_impacted,
                "critical_path_count": result.critical_path_count,
            }, indent=2))
        elif format == "csv":
            # CSV output
            click.echo(f"service,score,directly_impacted,indirectly_impacted")
            click.echo(f"{service},{score:.2f},{len(directly_impacted)},{len(indirectly_impacted)}")
        else:
            # Table output
            table = Table(title=f"Blast Radius Analysis: {service}")
            table.add_column("Metric", style="cyan")
            table.add_column("Value", style="green")

            table.add_row("Blast Radius Score", f"{score:.1%}")
            table.add_row("Risk Level", get_risk_level(score))
            table.add_row("Directly Impacted", str(len(directly_impacted)))
            table.add_row("Indirectly Impacted", str(len(indirectly_impacted)))

            console.print(table)

            if directly_impacted:
                console.print("\n[bold]Directly Impacted Services:[/bold]")
                for name in directly_impacted[:5]:
                    console.print(f"  • {name}")

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

        # Score (real Rust engine, via pyblastradius._core). score_all()
        # already returns results sorted by criticality_score descending.
        #
        # NOTE: despite its Rust doc comment claiming a "0-100" scale,
        # CriticalityScore.criticality_score is actually computed on a 0-1
        # scale (see src/analysis/criticality.rs: the weighted components
        # are summed then divided by 100.0). Treat it as already-normalized
        # here - do not divide by 100 again.
        scores = CriticalityScorer.score_all(graph)[:limit]

        # Output
        if format == "json":
            data = [
                {"service": s.node_name, "score": s.criticality_score}
                for s in scores
            ]
            click.echo(json.dumps(data, indent=2))
        else:
            table = Table(title="Service Criticality Ranking")
            table.add_column("Service", style="cyan")
            table.add_column("Criticality Score", style="green")
            table.add_column("Risk Level", style="yellow")

            for s in scores:
                table.add_row(
                    s.node_name,
                    f"{s.criticality_score:.1%}",
                    get_risk_level(s.criticality_score),
                )

            console.print(table)

    except Exception as e:
        console.print(f"❌ Error: {str(e)}", style="red")
        sys.exit(1)


_SCENARIO_TO_FAILURE_TYPE = {
    "cascade": FailureType.Outage,
    "outage": FailureType.Outage,
    "degradation": FailureType.Degradation,
    "high-latency": FailureType.HighLatency,
    "high-error-rate": FailureType.HighErrorRate,
    "data-quality": FailureType.DataQualityIssue,
}


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

        node = graph.find_node_by_name(target)
        if node is None:
            raise ValueError(f"No node named '{target}' found in {graph_file}")

        failure_type = _SCENARIO_TO_FAILURE_TYPE.get(scenario.lower())
        if failure_type is None:
            raise ValueError(
                f"Unknown scenario '{scenario}'. Choose one of: "
                f"{', '.join(sorted(_SCENARIO_TO_FAILURE_TYPE))}"
            )

        console.print(f"🔴 Simulating {target} failure...")
        console.print(f"📊 Scenario: {scenario}")

        # Real Rust engine cascade simulation (pyblastradius._core.Simulator)
        result = Simulator.simulate(graph, node.id, failure_type)

        console.print(f"\n⚠️  Impacted Services: {result.estimated_impact_nodes}")
        for service in sorted(result.affected_services):
            console.print(f"  • {service}")

        console.print(
            f"\n⏱️  Estimated Recovery Time: {result.estimated_recovery_time_minutes} minutes"
        )
        console.print(f"💰 Business Impact Score: {result.business_impact_score:.1f}")

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


def generate_interactive_html(graph: Graph) -> str:
    """Generate interactive HTML visualization"""
    # Simple HTML with graph data
    nodes = [{"id": node.id, "label": node.name} for node in graph.nodes()]
    edges = [{"source": e.source, "target": e.target} for e in graph.edges()]

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
