use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// StatGuardian integration for data quality tracking
/// Detects data quality issues that could cascade through pipelines
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatGuardianClient {
    pub api_endpoint: String,
    pub api_key: String,
    pub data_warehouse: DataWarehouse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataWarehouse {
    Snowflake,
    BigQuery,
    Databricks,
    PostgreSQL,
    Redshift,
}

impl StatGuardianClient {
    /// Create new StatGuardian client
    pub fn new(api_endpoint: String, api_key: String, warehouse: DataWarehouse) -> Self {
        StatGuardianClient {
            api_endpoint,
            api_key,
            data_warehouse: warehouse,
        }
    }

    /// StatGuardian's HTTP service exposes contract validation, not a rule
    /// catalogue. Returning invented rules here would make an unavailable
    /// integration look healthy.
    pub async fn get_quality_rules(&self) -> Result<Vec<QualityRule>> {
        Err(Error::AnalysisError(
            "StatGuardian does not expose a quality-rule listing API; call validate_attributes() with a contract-bound /validate service"
                .to_string(),
        ))
    }

    /// Validate one entity's attributes against the contract bound to the
    /// StatGuardian `/validate` service.
    ///
    /// This is the real HTTP integration. It sends
    /// `{"entity": {"attributes": ...}}` and returns the service's
    /// `passed`/`quality_score`/`issues`/`schema_version` response.
    #[cfg(feature = "http-discovery")]
    pub async fn validate_attributes(&self, attributes: Value) -> Result<ValidationResponse> {
        let url = format!("{}/validate", self.api_endpoint.trim_end_matches('/'));
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|error| Error::HttpError(format!("client init failed: {error}")))?;
        let mut request = client
            .post(&url)
            .json(&serde_json::json!({ "entity": { "attributes": attributes } }));
        if !self.api_key.trim().is_empty() {
            request = request.bearer_auth(self.api_key.trim());
        }
        let response = request
            .send()
            .await
            .map_err(|error| Error::HttpError(format!("request to {url} failed: {error}")))?;
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(Error::HttpError(format!(
                "StatGuardian returned {status}: {body}"
            )));
        }
        serde_json::from_str(&body).map_err(Error::from)
    }

    /// Without the `http-discovery` feature there is no HTTP client linked in.
    #[cfg(not(feature = "http-discovery"))]
    pub async fn validate_attributes(&self, _attributes: Value) -> Result<ValidationResponse> {
        Err(Error::ConfigError(
            "PyBlastRadius was built without the `http-discovery` feature; rebuild with --features http-discovery to call StatGuardian"
                .to_string(),
        ))
    }

    /// A table name alone is not enough to run a contract-bound validation.
    /// Callers should use [`Self::validate_attributes`] with real column
    /// values instead of receiving a fabricated PASS.
    pub async fn check_table_quality(&self, _table_name: &str) -> Result<DataQualityCheck> {
        Err(Error::AnalysisError(
            "check_table_quality needs row attributes; use validate_attributes() so StatGuardian can evaluate the bound contract"
                .to_string(),
        ))
    }

    /// Column-level lineage is a warehouse metadata query. StatGuardian's
    /// HTTP service does not expose it, and this client has no warehouse
    /// connection configured, so no lineage is fabricated.
    pub async fn get_column_lineage(&self, _table_name: &str) -> Result<Vec<ColumnLineageEdge>> {
        Err(Error::AnalysisError(
            "column lineage is not exposed by StatGuardian's /validate service; provide lineage from the warehouse metadata API"
                .to_string(),
        ))
    }

    /// Track data quality issues and their impact on downstream systems.
    ///
    /// Impact is computed from real lineage. Without a lineage source this
    /// returns an explicit error instead of inventing affected tables.
    pub async fn track_quality_incident(
        &self,
        incident: QualityIncident,
    ) -> Result<IncidentImpact> {
        let column_lineage = self.get_column_lineage(&incident.table_name).await?;

        let affected_tables: Vec<String> = column_lineage
            .iter()
            .map(|edge| edge.target_table.clone())
            .collect();

        let affected_metrics: Vec<String> = column_lineage
            .iter()
            .flat_map(|edge| edge.affected_metrics.clone())
            .collect();

        Ok(IncidentImpact {
            incident_id: incident.incident_id.clone(),
            root_cause_table: incident.table_name.clone(),
            root_cause_column: incident.column_name.clone(),
            affected_downstream_tables: affected_tables,
            affected_metrics,
            estimated_recovery_hours: Self::estimate_recovery_time(&incident),
        })
    }

    fn estimate_recovery_time(incident: &QualityIncident) -> u32 {
        match incident.severity {
            QualitySeverity::Critical => 4, // 4 hours
            QualitySeverity::High => 8,     // 8 hours
            QualitySeverity::Medium => 24,  // 1 day
            QualitySeverity::Low => 72,     // 3 days
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResponse {
    pub passed: bool,
    pub quality_score: f32,
    #[serde(default)]
    pub issues: Vec<String>,
    #[serde(default)]
    pub schema_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityRule {
    pub rule_id: String,
    pub table_name: String,
    pub column_name: String,
    pub rule_type: QualityRuleType,
    pub threshold: f32,
    pub severity: QualitySeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QualityRuleType {
    NotNull,
    Uniqueness,
    Freshness,
    ReferentialIntegrity,
    RangeCheck,
    PatternMatch,
    CustomSQL,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QualitySeverity {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataQualityCheck {
    pub check_id: String,
    pub table_name: String,
    pub check_type: QualityRuleType,
    pub status: QualityStatus,
    pub last_run: chrono::DateTime<chrono::Utc>,
    pub failed_rows: u64,
    pub failure_rate: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QualityStatus {
    Pass,
    Fail,
    Warning,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnLineageEdge {
    pub source_table: String,
    pub source_column: String,
    pub target_table: String,
    pub target_column: String,
    pub transformation: Option<String>,
    pub affected_metrics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityIncident {
    pub incident_id: String,
    pub table_name: String,
    pub column_name: String,
    pub severity: QualitySeverity,
    pub description: String,
    pub detected_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentImpact {
    pub incident_id: String,
    pub root_cause_table: String,
    pub root_cause_column: String,
    pub affected_downstream_tables: Vec<String>,
    pub affected_metrics: Vec<String>,
    pub estimated_recovery_hours: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_statguardian_client() {
        let client = StatGuardianClient::new(
            "https://statguardian.example.com".to_string(),
            "api_key_123".to_string(),
            DataWarehouse::Snowflake,
        );

        assert_eq!(client.data_warehouse, DataWarehouse::Snowflake);
    }

    #[tokio::test]
    async fn test_get_quality_rules_is_explicitly_unavailable() {
        let client = StatGuardianClient::new(
            "https://statguardian.example.com".to_string(),
            "api_key_123".to_string(),
            DataWarehouse::BigQuery,
        );

        let error = client.get_quality_rules().await.unwrap_err();
        assert!(matches!(error, Error::AnalysisError(_)));
    }

    #[tokio::test]
    async fn test_column_lineage_is_explicitly_unavailable() {
        let client = StatGuardianClient::new(
            "https://statguardian.example.com".to_string(),
            "api_key_123".to_string(),
            DataWarehouse::Databricks,
        );

        let error = client.get_column_lineage("orders").await.unwrap_err();
        assert!(matches!(error, Error::AnalysisError(_)));
    }

    #[tokio::test]
    async fn test_quality_incident_impact_requires_real_lineage() {
        let client = StatGuardianClient::new(
            "https://statguardian.example.com".to_string(),
            "api_key_123".to_string(),
            DataWarehouse::Snowflake,
        );

        let incident = QualityIncident {
            incident_id: "inc_001".to_string(),
            table_name: "raw_orders".to_string(),
            column_name: "user_id".to_string(),
            severity: QualitySeverity::Critical,
            description: "Null values detected in user_id".to_string(),
            detected_at: chrono::Utc::now(),
        };

        let error = client.track_quality_incident(incident).await.unwrap_err();
        assert!(matches!(error, Error::AnalysisError(_)));
    }

    #[cfg(feature = "http-discovery")]
    #[tokio::test]
    async fn test_validate_attributes_uses_real_http_endpoint() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/validate")
            .match_header("authorization", "Bearer secret")
            .match_body(mockito::Matcher::Json(json!({
                "entity": {"attributes": {"amount": 10}}
            })))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"passed":true,"quality_score":1.0,"issues":[],"schema_version":"orders.sg"}"#,
            )
            .create_async()
            .await;

        let client = StatGuardianClient::new(
            server.url(),
            "secret".to_string(),
            DataWarehouse::PostgreSQL,
        );
        let result = client
            .validate_attributes(json!({"amount": 10}))
            .await
            .unwrap();

        assert!(result.passed);
        assert_eq!(result.quality_score, 1.0);
        assert_eq!(result.schema_version, "orders.sg");
        mock.assert_async().await;
    }

    #[cfg(feature = "http-discovery")]
    #[tokio::test]
    async fn test_validate_attributes_surfaces_http_errors() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/validate")
            .with_status(503)
            .with_body("no contract configured")
            .create_async()
            .await;

        let client =
            StatGuardianClient::new(server.url(), String::new(), DataWarehouse::PostgreSQL);
        let error = client
            .validate_attributes(json!({"amount": 10}))
            .await
            .unwrap_err();

        assert!(matches!(error, Error::HttpError(_)));
        mock.assert_async().await;
    }
}
