"""REST API server for PyBlastRadius.

Authentication: every /api/v1/* endpoint requires a bearer API key, checked
against PYBLASTRADIUS_API_KEYS (comma-separated) or PYBLASTRADIUS_API_KEY.
There is no built-in default key - the server refuses to start with
/api/v1/* enabled unless at least one key is configured, so it can never be
silently exposed unauthenticated. Set PYBLASTRADIUS_ALLOW_NO_AUTH=1 to
explicitly opt out for local development only.

CORS: origins are locked down to PYBLASTRADIUS_CORS_ORIGINS (comma-separated
explicit origins, e.g. "https://app.example.com,https://ops.example.com").
No wildcard is used. If unset, no cross-origin browser access is permitted
(same-origin/non-browser clients like curl or the CLI are unaffected either
way, since CORS is a browser-enforced policy).
"""

import json
import logging
import os
import secrets
from typing import Optional, List, Dict, Any
from dataclasses import dataclass

try:
    from fastapi import FastAPI, HTTPException, BackgroundTasks, Security
    from fastapi.security import APIKeyHeader
    from fastapi.middleware.cors import CORSMiddleware
    import uvicorn
except ImportError:
    raise ImportError("FastAPI is required for REST API: pip install fastapi uvicorn")

from pyblastradius import (
    Graph,
    BlastRadiusAnalyzer,
    CriticalityScorer,
    FailureType,
    Simulator,
    TerraformDiscoverer,
    KubernetesDiscoverer,
    OtelDiscoverer,
    __version__,
)
from pyblastradius.discovery import AirflowDiscoverer

logger = logging.getLogger(__name__)

# Store analysis results in memory (in production, use a database)
analysis_cache: Dict[str, Dict[str, Any]] = {}
graph_cache: Optional[Graph] = None


def _load_api_keys() -> List[str]:
    """Load configured API keys from the environment.

    Supports either PYBLASTRADIUS_API_KEYS (comma-separated list) or the
    singular PYBLASTRADIUS_API_KEY, so a single-key deployment doesn't need
    a trailing comma.
    """
    keys = []
    if os.environ.get("PYBLASTRADIUS_API_KEYS"):
        keys.extend(k.strip() for k in os.environ["PYBLASTRADIUS_API_KEYS"].split(",") if k.strip())
    if os.environ.get("PYBLASTRADIUS_API_KEY"):
        keys.append(os.environ["PYBLASTRADIUS_API_KEY"].strip())
    return keys


_API_KEYS = _load_api_keys()
_ALLOW_NO_AUTH = os.environ.get("PYBLASTRADIUS_ALLOW_NO_AUTH") == "1"

if not _API_KEYS and not _ALLOW_NO_AUTH:
    raise RuntimeError(
        "No API key configured for the PyBlastRadius REST server. Set "
        "PYBLASTRADIUS_API_KEY (or PYBLASTRADIUS_API_KEYS) before starting "
        "the server, or explicitly set PYBLASTRADIUS_ALLOW_NO_AUTH=1 to run "
        "unauthenticated for local development only. Refusing to start "
        "unauthenticated by default so this can never be silently exposed."
    )

if _ALLOW_NO_AUTH:
    logger.warning(
        "SECURITY WARNING: PYBLASTRADIUS_ALLOW_NO_AUTH=1 is set - the "
        "PyBlastRadius REST server is running with authentication DISABLED "
        "when no API keys are configured. Never use this in production."
    )

_api_key_header = APIKeyHeader(name="Authorization", auto_error=False)


async def require_api_key(authorization: Optional[str] = Security(_api_key_header)) -> str:
    """FastAPI dependency enforcing bearer API key auth on every protected route."""
    if _ALLOW_NO_AUTH and not _API_KEYS:
        return "unauthenticated-dev-mode"

    if not authorization:
        raise HTTPException(status_code=401, detail="Missing Authorization header")

    token = authorization
    if authorization.lower().startswith("bearer "):
        token = authorization[7:]

    # Constant-time comparison against each configured key to avoid leaking
    # key material via timing side channels.
    for key in _API_KEYS:
        if secrets.compare_digest(token, key):
            return token

    raise HTTPException(status_code=401, detail="Invalid API key")


def _cors_origins() -> List[str]:
    raw = os.environ.get("PYBLASTRADIUS_CORS_ORIGINS", "")
    return [o.strip() for o in raw.split(",") if o.strip()]


app = FastAPI(
    title="PyBlastRadius API",
    description="Operational Blast Radius Intelligence Platform",
    version=__version__,
)

# CORS: explicit allowlist only. No wildcard ("*") - an empty list here
# means no cross-origin browser access at all, which is the safe default.
app.add_middleware(
    CORSMiddleware,
    allow_origins=_cors_origins(),
    allow_credentials=True,
    allow_methods=["GET", "POST"],
    allow_headers=["Authorization", "Content-Type"],
)


@app.get("/health")
async def health_check():
    """Health check endpoint (unauthenticated - used by load balancers/orchestrators)"""
    return {
        "status": "healthy",
        "version": __version__,
    }


@app.get("/api/v1/status")
async def status(_: str = Security(require_api_key)):
    """Get server status"""
    return {
        "status": "operational",
        "graph_loaded": graph_cache is not None,
        "graphs_count": len(analysis_cache),
    }


@dataclass
class DiscoveryRequest:
    """Request for discovering dependencies"""
    sources: List[Dict[str, str]]
    namespaces: Optional[List[str]] = None


@app.post("/api/v1/scan")
async def scan(request: DiscoveryRequest, _: str = Security(require_api_key)):
    """Scan infrastructure for dependencies"""
    try:
        global graph_cache
        graph = graph_cache if graph_cache is not None else Graph()

        for source in request.sources:
            source_type = source.get("type")
            if not source_type:
                continue

            if source_type == "kubernetes":
                discoverer = KubernetesDiscoverer(**{
                    k: v for k, v in source.items() if k != "type"
                })
            elif source_type == "terraform":
                discoverer = TerraformDiscoverer(**{
                    k: v for k, v in source.items() if k != "type"
                })
            elif source_type == "airflow":
                discoverer = AirflowDiscoverer(**{
                    k: v for k, v in source.items() if k != "type"
                })
            elif source_type == "otlp":
                discoverer = OtelDiscoverer(**{
                    k: v for k, v in source.items() if k != "type"
                })
            else:
                continue

            sub_graph = discoverer.discover()
            graph.merge(sub_graph)

        graph_cache = graph

        return {
            "nodes_count": graph.node_count(),
            "edges_count": graph.edge_count(),
            "sources": len(request.sources),
        }

    except Exception as e:
        raise HTTPException(status_code=400, detail=str(e))


@dataclass
class AnalysisRequest:
    """Request for analyzing blast radius"""
    service: str
    max_depth: int = 3


@app.post("/api/v1/analyze")
async def analyze(request: AnalysisRequest, _: str = Security(require_api_key)):
    """Analyze blast radius for a service (by node name)"""
    try:
        if not graph_cache:
            raise ValueError("No graph loaded. Run /api/v1/scan or /api/v1/graphs/load first.")

        node = graph_cache.find_node_by_name(request.service)
        if node is None:
            raise ValueError(f"No node named '{request.service}' in the loaded graph")

        # Real Rust engine (pyblastradius._core.BlastRadiusAnalyzer)
        result = BlastRadiusAnalyzer.analyze(graph_cache, node.id, max_depth=request.max_depth)

        def names(node_ids):
            resolved = []
            for nid in node_ids:
                n = graph_cache.get_node(nid)
                resolved.append(n.name if n else nid)
            return resolved

        analysis_id = f"analysis-{secrets.token_hex(8)}"
        analysis_cache[analysis_id] = {
            "service": request.service,
            "blast_radius_score": result.blast_radius_score,
            "directly_impacted": names(result.directly_impacted),
            "indirectly_impacted": names(result.indirectly_impacted),
            "tertiary_impacted": names(result.tertiary_impacted),
            "total_impacted": result.total_impacted,
            "critical_path_count": result.critical_path_count,
        }

        return {
            "id": analysis_id,
            "status": "completed",
            "result": analysis_cache[analysis_id],
        }

    except ValueError as e:
        raise HTTPException(status_code=400, detail=str(e))
    except Exception as e:
        raise HTTPException(status_code=400, detail=str(e))


@app.get("/api/v1/analyses/{analysis_id}")
async def get_analysis(analysis_id: str, _: str = Security(require_api_key)):
    """Get analysis result by ID"""
    if analysis_id not in analysis_cache:
        raise HTTPException(status_code=404, detail="Analysis not found")

    return {
        "id": analysis_id,
        "status": "completed",
        "result": analysis_cache[analysis_id],
    }


@dataclass
class CriticalityRequest:
    """Request for criticality scoring"""
    limit: int = 10


@app.post("/api/v1/criticality")
async def criticality(request: CriticalityRequest, _: str = Security(require_api_key)):
    """Score services by criticality"""
    try:
        if not graph_cache:
            raise ValueError("No graph loaded. Run /api/v1/scan or /api/v1/graphs/load first.")

        # Real Rust engine (pyblastradius._core.CriticalityScorer). Already
        # sorted by criticality_score descending; criticality_score is a
        # 0-1 value despite its Rust doc comment saying "0-100".
        scores = CriticalityScorer.score_all(graph_cache)[: request.limit]

        return {
            "count": len(scores),
            "services": [
                {"service": s.node_name, "score": s.criticality_score}
                for s in scores
            ],
        }

    except Exception as e:
        raise HTTPException(status_code=400, detail=str(e))


@app.get("/api/v1/graphs")
async def list_graphs(_: str = Security(require_api_key)):
    """List available graphs"""
    return {
        "current": {
            "nodes": graph_cache.node_count() if graph_cache else 0,
            "edges": graph_cache.edge_count() if graph_cache else 0,
        },
        "analyses": len(analysis_cache),
    }


@app.post("/api/v1/graphs/load")
async def load_graph(graph_json: Dict[str, Any], _: str = Security(require_api_key)):
    """Load a graph from JSON"""
    try:
        global graph_cache
        graph_cache = Graph.from_json(json.dumps(graph_json))

        return {
            "status": "loaded",
            "nodes": graph_cache.node_count(),
        }

    except Exception as e:
        raise HTTPException(status_code=400, detail=str(e))


@app.get("/api/v1/openapi.json")
async def openapi(_: str = Security(require_api_key)):
    """OpenAPI schema"""
    return app.openapi()


def run_server(host: str = "127.0.0.1", port: int = 8080):
    """Run the server"""
    uvicorn.run(
        app,
        host=host,
        port=port,
        log_level="info",
    )


if __name__ == "__main__":
    run_server()
