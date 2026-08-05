#!/usr/bin/env python3
"""Quick start example - analyze Kubernetes cluster in 5 lines"""

from pyblastradius import KubernetesDiscoverer, BlastRadiusAnalyzer

# Discover
discoverer = KubernetesDiscoverer()
graph = discoverer.discover()

# Analyze
analyzer = BlastRadiusAnalyzer(graph)
result = analyzer.analyze("payment-service")

# Print results
print(f"Blast radius: {result.score:.1%}")
print(f"Directly impacted: {len(result.directly_impacted)} services")
print(f"Indirectly impacted: {len(result.indirectly_impacted)} services")
