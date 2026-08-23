use crate::error::Result;
use serde::{Deserialize, Serialize};

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

    /// Get data quality checks and rules
    pub async fn get_quality_rules(&self) -> Result<Vec<QualityRule>> {
        // In production: Query StatGuardian API for configured rules
        let rules = vec![
            QualityRule {
                rule_id: "null_check_users".to_string(),
                table_name: "users".to_string(),
                column_name: "email".to_string(),
                rule_type: QualityRuleType::NotNull,
                threshold: 0.0, // No nulls allowed
                severity: QualitySeverity::Critical,
            },
            QualityRule {
                rule_id: "freshness_check_events".to_string(),
                table_name: "events".to_string(),
                column_name: "created_at".to_string(),
                rule_type: QualityRuleType::Freshness,
                threshold: 60.0, // Max 60 min old
                severity: QualitySeverity::High,
            },
            QualityRule {
                rule_id: "uniqueness_check_orders".to_string(),
                table_name: "orders".to_string(),
                column_name: "order_id".to_string(),
                rule_type: QualityRuleType::Uniqueness,
                threshold: 1.0, // 100% unique
                severity: QualitySeverity::Critical,
            },
        ];

        Ok(rules)
    }

    /// Check data quality status for specific table
    pub async fn check_table_quality(&self, table_name: &str) -> Result<DataQualityCheck> {
        let checks = [DataQualityCheck {
            check_id: format!("check_{}_001", table_name),
            table_name: table_name.to_string(),
            check_type: QualityRuleType::NotNull,
            status: QualityStatus::Pass,
            last_run: chrono::Utc::now(),
            failed_rows: 0,
            failure_rate: 0.0,
        }];

        Ok(checks[0].clone())
    }

    /// Get column-level lineage from data warehouse
    /// Returns which columns feed into other tables
    pub async fn get_column_lineage(&self, _table_name: &str) -> Result<Vec<ColumnLineageEdge>> {
        // In production: Query data warehouse for column-level lineage
        let lineage = vec![
            ColumnLineageEdge {
                source_table: "raw_orders".to_string(),
                source_column: "user_id".to_string(),
                target_table: "fact_orders".to_string(),
                target_column: "customer_id".to_string(),
                transformation: Some("CAST(user_id AS INT)".to_string()),
                affected_metrics: vec!["revenue".to_string(), "order_count".to_string()],
            },
            ColumnLineageEdge {
                source_table: "fact_orders".to_string(),
                source_column: "revenue".to_string(),
                target_table: "dashboards.sales_dashboard".to_string(),
                target_column: "total_revenue".to_string(),
                transformation: Some("SUM(revenue)".to_string()),
                affected_metrics: vec!["executive_kpi".to_string()],
            },
        ];

        Ok(lineage)
    }

    /// Track data quality issues and their impact on downstream systems
    pub async fn track_quality_incident(
        &self,
        incident: QualityIncident,
    ) -> Result<IncidentImpact> {
        // Find all downstream tables affected by this quality issue
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
    async fn test_get_quality_rules() {
        let client = StatGuardianClient::new(
            "https://statguardian.example.com".to_string(),
            "api_key_123".to_string(),
            DataWarehouse::BigQuery,
        );

        let rules = client.get_quality_rules().await.unwrap();
        assert!(!rules.is_empty());
        assert!(rules
            .iter()
            .any(|r| r.severity == QualitySeverity::Critical));
    }

    #[tokio::test]
    async fn test_column_lineage() {
        let client = StatGuardianClient::new(
            "https://statguardian.example.com".to_string(),
            "api_key_123".to_string(),
            DataWarehouse::Databricks,
        );

        let lineage = client.get_column_lineage("orders").await.unwrap();
        assert!(!lineage.is_empty());
        assert!(lineage[0].affected_metrics.len() > 0);
    }

    #[tokio::test]
    async fn test_quality_incident_impact() {
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

        let impact = client.track_quality_incident(incident).await.unwrap();
        assert!(!impact.affected_downstream_tables.is_empty());
        assert_eq!(impact.estimated_recovery_hours, 4); // Critical
    }
}
