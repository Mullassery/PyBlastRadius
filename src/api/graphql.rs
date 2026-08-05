use serde::{Deserialize, Serialize};

/// GraphQL query request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLQuery {
    pub query: String,
    pub variables: Option<serde_json::Value>,
    pub operation_name: Option<String>,
}

/// GraphQL schema definition
pub struct GraphQLSchema;

impl GraphQLSchema {
    /// Create GraphQL query for blast radius analysis
    pub fn blast_radius_query(node_id: &str, max_depth: Option<usize>) -> GraphQLQuery {
        let depth = max_depth.unwrap_or(3);
        GraphQLQuery {
            query: format!(
                r#"
                query {{
                  blastRadius(nodeId: "{}", maxDepth: {}) {{
                    nodeId
                    directlyImpacted {{ id name }}
                    indirectlyImpacted {{ id name }}
                    tertiaryImpacted {{ id name }}
                    score
                    criticalPaths
                  }}
                }}
                "#,
                node_id, depth
            ),
            variables: None,
            operation_name: Some("BlastRadiusQuery".to_string()),
        }
    }

    /// Create GraphQL query for criticality ranking
    pub fn criticality_query(limit: Option<usize>) -> GraphQLQuery {
        let limit = limit.unwrap_or(10);
        GraphQLQuery {
            query: format!(
                r#"
                query {{
                  criticality(limit: {}) {{
                    nodeId
                    nodeName
                    score
                    downstreamCount
                    isSinglePointOfFailure
                  }}
                }}
                "#,
                limit
            ),
            variables: None,
            operation_name: Some("CriticalityQuery".to_string()),
        }
    }

    /// Create GraphQL query for anomaly detection
    pub fn anomalies_query(min_severity: Option<f32>) -> GraphQLQuery {
        let severity = min_severity.unwrap_or(0.5);
        GraphQLQuery {
            query: format!(
                r#"
                query {{
                  anomalies(minSeverity: {}) {{
                    nodeId
                    nodeName
                    type
                    severity
                    confidence
                    predictedTTF
                    evidence
                  }}
                }}
                "#,
                severity
            ),
            variables: None,
            operation_name: Some("AnomaliesQuery".to_string()),
        }
    }

    /// Create GraphQL mutation to simulate failure
    pub fn simulate_failure_mutation(node_id: &str, failure_type: &str) -> GraphQLQuery {
        GraphQLQuery {
            query: format!(
                r#"
                mutation {{
                  simulateFailure(nodeId: "{}", failureType: "{}") {{
                    nodeId
                    cascadePath
                    estimatedImpactNodes
                    estimatedRecoveryMinutes
                    businessImpactScore
                  }}
                }}
                "#,
                node_id, failure_type
            ),
            variables: None,
            operation_name: Some("SimulateFailure".to_string()),
        }
    }

    /// Create GraphQL query for hidden dependencies
    pub fn hidden_dependencies_query(min_confidence: Option<f32>) -> GraphQLQuery {
        let confidence = min_confidence.unwrap_or(0.6);
        GraphQLQuery {
            query: format!(
                r#"
                query {{
                  hiddenDependencies(minConfidence: {}) {{
                    source {{ id name }}
                    target {{ id name }}
                    type
                    confidence
                    detectionMethod
                    evidence
                  }}
                }}
                "#,
                confidence
            ),
            variables: None,
            operation_name: Some("HiddenDependencies".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blast_radius_query() {
        let query = GraphQLSchema::blast_radius_query("node-123", Some(2));
        assert!(query.query.contains("blastRadius"));
        assert!(query.query.contains("maxDepth: 2"));
    }

    #[test]
    fn test_criticality_query() {
        let query = GraphQLSchema::criticality_query(Some(20));
        assert!(query.query.contains("criticality"));
        assert!(query.query.contains("limit: 20"));
    }

    #[test]
    fn test_simulate_failure_mutation() {
        let mutation = GraphQLSchema::simulate_failure_mutation("postgres", "outage");
        assert!(mutation.query.contains("simulateFailure"));
        assert!(mutation.query.contains("postgres"));
        assert!(mutation.query.contains("outage"));
    }
}
