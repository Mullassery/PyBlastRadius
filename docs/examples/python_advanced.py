#!/usr/bin/env python3
"""Advanced example - multi-source analysis with filtering"""

from pyblastradius import (
    Graph,
    KubernetesDiscoverer,
    TerraformDiscoverer,
    BlastRadiusAnalyzer,
    CriticalityScorer,
)

# Discover from multiple sources
k8s_graph = KubernetesDiscoverer().discover()
tf_graph = TerraformDiscoverer(path="./infra").discover()

# Merge graphs
graph = Graph.merge([k8s_graph, tf_graph])

# Filter to production only
graph = graph.filter(lambda n: n.metadata.get("environment") == "production")

# Run comprehensive analysis
blast = BlastRadiusAnalyzer(graph)
crit = CriticalityScorer(graph)

print("Service Analysis:")
print("-" * 60)

for service in sorted(graph.services())[:10]:  # Top 10
    br = blast.analyze(service)
    score = crit.score(service)

    print(f"{service:30} | BR: {br.score:5.1%} | Crit: {score:5.1%}")
