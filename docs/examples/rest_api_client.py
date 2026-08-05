#!/usr/bin/env python3
"""Example REST API client for PyBlastRadius"""

import requests
import json
import time

API_BASE = "http://127.0.0.1:8080"


def main():
    """Example workflow"""
    client = PyBlastRadiusClient(API_BASE)

    # 1. Check health
    print("1. Checking server health...")
    health = client.health_check()
    print(f"   Status: {health['status']}")

    # 2. Scan infrastructure
    print("\n2. Scanning infrastructure...")
    scan_result = client.scan([
        {"type": "kubernetes"},
        {"type": "terraform", "path": "./infra"},
    ])
    print(f"   Discovered {scan_result['nodes_count']} nodes")

    # 3. Analyze blast radius
    print("\n3. Analyzing blast radius for payment-service...")
    analysis = client.analyze("payment-service", max_depth=3)
    print(f"   Score: {analysis['result']['score']:.1%}")
    print(f"   Directly impacted: {len(analysis['result']['directly_impacted'])}")
    print(f"   Indirectly impacted: {len(analysis['result']['indirectly_impacted'])}")

    # 4. Get criticality ranking
    print("\n4. Getting criticality ranking...")
    criticality = client.criticality(limit=5)
    print("   Top services by criticality:")
    for svc in criticality["services"]:
        print(f"   - {svc['service']}: {svc['score']:.1%}")


class PyBlastRadiusClient:
    """Client for PyBlastRadius REST API"""

    def __init__(self, base_url: str):
        self.base_url = base_url

    def health_check(self) -> dict:
        """Check server health"""
        resp = requests.get(f"{self.base_url}/health")
        resp.raise_for_status()
        return resp.json()

    def scan(self, sources: list) -> dict:
        """Scan infrastructure"""
        resp = requests.post(
            f"{self.base_url}/api/v1/scan",
            json={"sources": sources}
        )
        resp.raise_for_status()
        return resp.json()

    def analyze(self, service: str, max_depth: int = 3) -> dict:
        """Analyze blast radius"""
        resp = requests.post(
            f"{self.base_url}/api/v1/analyze",
            json={
                "service": service,
                "max_depth": max_depth,
            }
        )
        resp.raise_for_status()
        return resp.json()

    def criticality(self, limit: int = 10) -> dict:
        """Get criticality ranking"""
        resp = requests.post(
            f"{self.base_url}/api/v1/criticality",
            json={"limit": limit}
        )
        resp.raise_for_status()
        return resp.json()


if __name__ == "__main__":
    try:
        main()
    except requests.exceptions.ConnectionError:
        print("❌ Error: Could not connect to server")
        print("   Start the server with: pyblastradius server")
