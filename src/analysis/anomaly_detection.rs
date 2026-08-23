use crate::error::Result;
use crate::graph::node::NodeId;
use crate::graph::Graph;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Types of anomalies that precede failures
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnomalyType {
    LatencySpike,       // Response time suddenly increased
    ErrorRateIncrease,  // Error percentage climbing
    ThroughputDrop,     // Request throughput decreased
    ResourceExhaustion, // CPU, memory, connections running out
    TrafficAnomaly,     // Unusual traffic pattern
    HealthCheckFailure, // Health checks failing
}

/// Detected anomaly with severity and predicted impact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedAnomaly {
    pub node_id: NodeId,
    pub node_name: String,
    pub anomaly_type: AnomalyType,
    pub severity: f32,      // 0-1.0, higher = more severe
    pub confidence: f32,    // 0-1.0, prediction confidence
    pub predicted_ttf: u32, // Time to failure in minutes
    pub evidence: Vec<String>,
}

pub struct AnomalyDetector;

impl AnomalyDetector {
    /// Detect anomalies that precede failures
    /// In production: Analyzes metrics from Prometheus/Datadog
    pub fn detect(graph: &Graph) -> Result<Vec<DetectedAnomaly>> {
        let mut anomalies = Vec::new();

        // Method 1: Latency analysis
        anomalies.extend(Self::detect_latency_spikes(graph)?);

        // Method 2: Error rate analysis
        anomalies.extend(Self::detect_error_rate_increase(graph)?);

        // Method 3: Resource exhaustion
        anomalies.extend(Self::detect_resource_exhaustion(graph)?);

        // Method 4: Traffic pattern analysis
        anomalies.extend(Self::detect_traffic_anomalies(graph)?);

        Ok(anomalies)
    }

    /// Detect sudden latency increases
    fn detect_latency_spikes(graph: &Graph) -> Result<Vec<DetectedAnomaly>> {
        let mut anomalies = Vec::new();

        for edge in graph.edges() {
            if let Some(latency) = edge.properties.latency_ms {
                // Baseline for different service types
                let baseline = match graph.get_node(&edge.target).map(|n| &n.node_type) {
                    Some(crate::graph::NodeType::Database) => 100.0, // DBs typically ~50-100ms
                    Some(crate::graph::NodeType::Cache) => 10.0,     // Cache typically ~5-10ms
                    _ => 50.0,                                       // Default baseline
                };

                // If latency > 2x baseline, it's a spike
                if latency > baseline * 2.0 {
                    let severity = ((latency - baseline) / baseline / 10.0).min(1.0);

                    if let Some(target) = graph.get_node(&edge.target) {
                        anomalies.push(DetectedAnomaly {
                            node_id: edge.target,
                            node_name: target.name.clone(),
                            anomaly_type: AnomalyType::LatencySpike,
                            severity,
                            confidence: 0.85,
                            predicted_ttf: Self::estimate_ttf(severity),
                            evidence: vec![
                                format!("Latency: {:.1}ms (baseline: {:.1}ms)", latency, baseline),
                                format!("Spike ratio: {:.1}x", latency / baseline),
                            ],
                        });
                    }
                }
            }
        }

        Ok(anomalies)
    }

    /// Detect increasing error rates
    fn detect_error_rate_increase(graph: &Graph) -> Result<Vec<DetectedAnomaly>> {
        let mut anomalies = Vec::new();

        for edge in graph.edges() {
            if let Some(error_rate) = edge.properties.error_rate {
                // Normal threshold: <1% errors
                let normal_threshold = 0.01;

                if error_rate > normal_threshold {
                    let severity = (error_rate / 0.1).min(1.0); // 10% is severe

                    if let Some(target) = graph.get_node(&edge.target) {
                        anomalies.push(DetectedAnomaly {
                            node_id: edge.target,
                            node_name: target.name.clone(),
                            anomaly_type: AnomalyType::ErrorRateIncrease,
                            severity,
                            confidence: 0.9,
                            predicted_ttf: Self::estimate_ttf(severity),
                            evidence: vec![
                                format!("Error rate: {:.1}%", error_rate * 100.0),
                                format!("Above threshold: {:.1}%", normal_threshold * 100.0),
                            ],
                        });
                    }
                }
            }
        }

        Ok(anomalies)
    }

    /// Detect resource exhaustion (CPU, memory, connections)
    fn detect_resource_exhaustion(graph: &Graph) -> Result<Vec<DetectedAnomaly>> {
        let mut anomalies = Vec::new();

        // In production: Query Prometheus/cloud provider metrics
        // Look for:
        // - CPU > 80%
        // - Memory > 85%
        // - Connection pool > 90%
        // - Open file descriptors high

        // For MVP: Simulate based on graph structure
        for node in graph.nodes() {
            let downstream_count = graph.get_neighbors(&node.id).unwrap_or_default().len();

            // If many downstream services, might be resource bottleneck
            if downstream_count > 10 {
                let severity = (downstream_count as f32 - 10.0) / 20.0;

                anomalies.push(DetectedAnomaly {
                    node_id: node.id,
                    node_name: node.name.clone(),
                    anomaly_type: AnomalyType::ResourceExhaustion,
                    severity: severity.min(1.0),
                    confidence: 0.65,
                    predicted_ttf: Self::estimate_ttf(severity),
                    evidence: vec![
                        format!(
                            "{} downstream services depending on this node",
                            downstream_count
                        ),
                        "High load concentration detected".to_string(),
                    ],
                });
            }
        }

        Ok(anomalies)
    }

    /// Detect unusual traffic patterns
    fn detect_traffic_anomalies(_graph: &Graph) -> Result<Vec<DetectedAnomaly>> {
        let anomalies = Vec::new();

        // In production: Compare current traffic vs. baseline
        // Detect:
        // - Traffic surge (might cause resource exhaustion)
        // - Traffic drop (might indicate upstream failure)
        // - Unusual distribution patterns

        // For MVP: Return empty
        Ok(anomalies)
    }

    /// Estimate time to failure based on severity
    fn estimate_ttf(severity: f32) -> u32 {
        match severity {
            s if s > 0.9 => 5,  // Critical: 5 minutes
            s if s > 0.7 => 15, // High: 15 minutes
            s if s > 0.5 => 30, // Medium: 30 minutes
            s if s > 0.3 => 60, // Low: 60 minutes
            _ => 120,           // Very low: 120 minutes
        }
    }
}

/// Statistical anomaly detection using consensus methods
pub struct ConsensusAnomalyDetector;

impl ConsensusAnomalyDetector {
    /// Multiple algorithms voting on anomalies
    pub fn detect_consensus(
        latency_anomalies: Vec<DetectedAnomaly>,
        error_anomalies: Vec<DetectedAnomaly>,
        resource_anomalies: Vec<DetectedAnomaly>,
    ) -> Vec<DetectedAnomaly> {
        let mut combined: HashMap<NodeId, Vec<DetectedAnomaly>> = HashMap::new();

        // Combine all anomalies
        for anom in latency_anomalies
            .into_iter()
            .chain(error_anomalies)
            .chain(resource_anomalies)
        {
            combined.entry(anom.node_id).or_default().push(anom);
        }

        // Filter: Keep only anomalies detected by multiple methods
        combined
            .into_values()
            .flat_map(|mut anoms| {
                if anoms.len() > 1 {
                    // Combine evidence and increase confidence
                    let first = anoms.remove(0);
                    let mut combined_anom = first;
                    combined_anom.confidence =
                        (combined_anom.confidence * anoms.len() as f32).min(1.0);

                    for anom in anoms {
                        combined_anom.evidence.extend(anom.evidence);
                    }

                    Some(combined_anom)
                } else {
                    anoms.into_iter().next()
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Node;

    #[test]
    fn test_anomaly_detection() {
        let mut graph = Graph::new();

        let _n1 = graph.add_node(Node::service("api")).unwrap();
        let n2 = graph
            .add_node(Node::database("postgres").with_recovery_time(30))
            .unwrap();

        let mut edge = crate::graph::Edge::new(crate::graph::EdgeType::Queries, _n1, n2);
        edge.properties.latency_ms = Some(250.0); // High latency
        edge.properties.error_rate = Some(0.05); // 5% errors
        graph.add_edge(edge).unwrap();

        let anomalies = AnomalyDetector::detect(&graph).unwrap();

        assert!(!anomalies.is_empty());
        assert!(anomalies
            .iter()
            .any(|a| a.anomaly_type == AnomalyType::LatencySpike));
        assert!(anomalies
            .iter()
            .any(|a| a.anomaly_type == AnomalyType::ErrorRateIncrease));
    }

    #[test]
    fn test_ttf_estimation() {
        assert_eq!(AnomalyDetector::estimate_ttf(0.95), 5);
        assert_eq!(AnomalyDetector::estimate_ttf(0.75), 15);
        assert_eq!(AnomalyDetector::estimate_ttf(0.55), 30);
    }
}
