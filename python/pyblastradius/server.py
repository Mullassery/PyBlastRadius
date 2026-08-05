"""REST API server for PyBlastRadius"""

import json
import asyncio
from typing import Optional, List, Dict, Any
from dataclasses import dataclass, asdict

try:
    from fastapi import FastAPI, HTTPException, BackgroundTasks
    from fastapi.responses import JSONResponse
    from fastapi.middleware.cors import CORSMiddleware
    import uvicorn
except ImportError:
    raise ImportError("FastAPI is required for REST API: pip install fastapi uvicorn")

from pyblastradius import Graph, BlastRadiusAnalyzer, CriticalityScorer
from pyblastradius.discovery import (
    KubernetesDiscoverer,
    TerraformDiscoverer,
    AirflowDiscoverer,
    OtelDiscoverer,
)

# Store analysis results in memory (in production, use a database)
analysis_cache: Dict[str, Dict[str, Any]] = {}
graph_cache: Optional[Graph] = None


app = FastAPI(
    title="PyBlastRadius API",
    description="Operational Blast Radius Intelligence Platform",
    version="0.1.0",
)

# Enable CORS
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


@app.get("/health")
async def health_check():
    """Health check endpoint"""
    return {
        "status": "healthy",
        "version": "0.1.0",
    }


@app.get("/api/v1/status")
async def status():
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
async def scan(request: DiscoveryRequest):
    """Scan infrastructure for dependencies"""
    try:
        graph = Graph()

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
            graph = Graph.merge([graph, sub_graph])

        return {
            "nodes_count": graph.node_count(),
            "edges_count": len(list(graph.edges())),
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
async def analyze(request: AnalysisRequest, background_tasks: BackgroundTasks):
    """Analyze blast radius for a service"""
    try:
        if not graph_cache:
            raise ValueError("No graph loaded. Run /api/v1/scan first.")

        analyzer = BlastRadiusAnalyzer(graph_cache)
        result = analyzer.analyze(request.service, max_depth=request.max_depth)

        analysis_id = f"analysis-{id(result)}"
        analysis_cache[analysis_id] = {
            "service": request.service,
            "score": result.score,
            "directly_impacted": result.directly_impacted,
            "indirectly_impacted": result.indirectly_impacted,
            "critical_paths": result.critical_paths,
        }

        return {
            "id": analysis_id,
            "status": "completed",
            "result": analysis_cache[analysis_id],
        }

    except Exception as e:
        raise HTTPException(status_code=400, detail=str(e))


@app.get("/api/v1/analyses/{analysis_id}")
async def get_analysis(analysis_id: str):
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
async def criticality(request: CriticalityRequest):
    """Score services by criticality"""
    try:
        if not graph_cache:
            raise ValueError("No graph loaded. Run /api/v1/scan first.")

        scorer = CriticalityScorer(graph_cache)
        scores = []

        for service in sorted(graph_cache.services())[:request.limit]:
            score = scorer.score(service)
            scores.append({
                "service": service,
                "score": score,
            })

        scores.sort(key=lambda x: x["score"], reverse=True)

        return {
            "count": len(scores),
            "services": scores,
        }

    except Exception as e:
        raise HTTPException(status_code=400, detail=str(e))


@app.get("/api/v1/graphs")
async def list_graphs():
    """List available graphs"""
    return {
        "current": {
            "nodes": graph_cache.node_count() if graph_cache else 0,
            "edges": len(list(graph_cache.edges())) if graph_cache else 0,
        },
        "analyses": len(analysis_cache),
    }


@app.post("/api/v1/graphs/load")
async def load_graph(graph_json: Dict[str, Any]):
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
async def openapi():
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
