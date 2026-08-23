use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Track column-level lineage shifts and their impact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnLineageTracker {
    pub lineage_map: HashMap<String, Vec<ColumnDependency>>,
    pub tracked_columns: HashMap<String, ColumnMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDependency {
    pub source_table: String,
    pub source_column: String,
    pub target_table: String,
    pub target_column: String,
    pub transformation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnMetadata {
    pub table_name: String,
    pub column_name: String,
    pub data_type: String,
    pub is_pii: bool,
    pub is_key: bool,
    pub is_metric: bool,
    pub used_in_models: Vec<String>,
}

impl ColumnLineageTracker {
    pub fn new() -> Self {
        ColumnLineageTracker {
            lineage_map: HashMap::new(),
            tracked_columns: HashMap::new(),
        }
    }

    /// Track a column through the pipeline
    pub fn track_column(
        &mut self,
        source: String,
        target: String,
        transformation: Option<String>,
    ) -> Result<()> {
        let key = format!("{}_{}_to_{}", source, target, target);
        self.lineage_map
            .entry(key)
            .or_default()
            .push(ColumnDependency {
                source_table: source.split('.').next().unwrap_or("").to_string(),
                source_column: source.split('.').next_back().unwrap_or("").to_string(),
                target_table: target.split('.').next().unwrap_or("").to_string(),
                target_column: target.split('.').next_back().unwrap_or("").to_string(),
                transformation,
            });

        Ok(())
    }

    /// Find all downstream tables affected by a column change
    pub fn find_downstream_impact(&self, source_column: &str) -> Vec<String> {
        let mut affected = Vec::new();

        for deps in self.lineage_map.values() {
            for dep in deps {
                if format!("{}.{}", dep.source_table, dep.source_column) == source_column {
                    affected.push(format!("{}.{}", dep.target_table, dep.target_column));
                }
            }
        }

        affected
    }
}

impl Default for ColumnLineageTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Data quality event (from StatGuardian)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataQualityEvent {
    pub event_id: String,
    pub event_type: QualityEventType,
    pub table_name: String,
    pub column_name: String,
    pub severity: EventSeverity,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub details: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QualityEventType {
    NullValuesDetected,
    SchemaChange,
    DataFreshness,
    DuplicatesDetected,
    ReferentialIntegrityViolation,
    OutOfRange,
    PatternMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventSeverity {
    Critical,
    High,
    Medium,
    Low,
}

/// Quality metric for a column or table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetric {
    pub metric_id: String,
    pub table_name: String,
    pub column_name: Option<String>,
    pub metric_type: MetricType,
    pub value: f32,
    pub threshold: f32,
    pub is_passing: bool,
    pub last_check: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricType {
    Freshness,    // How recent is the data (hours)
    Completeness, // % non-null values
    Uniqueness,   // % unique values
    Timeliness,   // SLA compliance
    Validity,     // Format/range checks
    Accuracy,     // Validation against source
}

/// Integrates column lineage with quality metrics to predict cascade impact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataQualityImpactAnalyzer {
    pub lineage_tracker: ColumnLineageTracker,
    pub quality_metrics: HashMap<String, QualityMetric>,
}

impl DataQualityImpactAnalyzer {
    pub fn new() -> Self {
        DataQualityImpactAnalyzer {
            lineage_tracker: ColumnLineageTracker::new(),
            quality_metrics: HashMap::new(),
        }
    }

    /// Analyze impact of quality degradation on downstream systems
    pub fn analyze_quality_degradation(
        &self,
        affected_column: &str,
    ) -> Result<QualityImpactReport> {
        let downstream = self.lineage_tracker.find_downstream_impact(affected_column);

        let affected_dashboards = downstream
            .iter()
            .filter(|col| col.contains("dashboard") || col.contains("mart"))
            .count();

        let affected_models = downstream
            .iter()
            .filter(|col| col.contains("fct_") || col.contains("dim_"))
            .count();

        let severity = if downstream.len() > 10 {
            EventSeverity::Critical
        } else if downstream.len() > 5 {
            EventSeverity::High
        } else if !downstream.is_empty() {
            EventSeverity::Medium
        } else {
            EventSeverity::Low
        };

        Ok(QualityImpactReport {
            source_column: affected_column.to_string(),
            affected_downstream_columns: downstream,
            affected_dashboards: affected_dashboards as u32,
            affected_models: affected_models as u32,
            estimated_recovery_hours: Self::estimate_recovery(&severity),
            severity,
            recommendations: Self::generate_recommendations(&severity),
        })
    }

    /// Register a quality metric
    pub fn register_metric(&mut self, metric: QualityMetric) -> Result<()> {
        let key = format!(
            "{}_{}",
            metric.table_name,
            metric.column_name.clone().unwrap_or_default()
        );
        self.quality_metrics.insert(key, metric);
        Ok(())
    }

    /// Process a quality event and measure cascade impact
    pub fn process_quality_event(&self, event: &DataQualityEvent) -> Result<EventCascadeReport> {
        let source_col = format!("{}.{}", event.table_name, event.column_name);
        let impact = self.analyze_quality_degradation(&source_col)?;

        Ok(EventCascadeReport {
            event_id: event.event_id.clone(),
            event_type: event.event_type,
            root_cause: source_col,
            cascade_impact: impact,
            timestamp: event.timestamp,
        })
    }

    fn estimate_recovery(severity: &EventSeverity) -> u32 {
        match severity {
            EventSeverity::Critical => 2, // 2 hours
            EventSeverity::High => 4,     // 4 hours
            EventSeverity::Medium => 8,   // 8 hours
            EventSeverity::Low => 24,     // 1 day
        }
    }

    fn generate_recommendations(severity: &EventSeverity) -> Vec<String> {
        match severity {
            EventSeverity::Critical => vec![
                "Immediately pause dependent pipelines".to_string(),
                "Alert data governance team".to_string(),
                "Investigate root cause urgently".to_string(),
            ],
            EventSeverity::High => vec![
                "Schedule investigation within 1 hour".to_string(),
                "Notify downstream stakeholders".to_string(),
                "Prepare rollback plan".to_string(),
            ],
            EventSeverity::Medium => vec![
                "Schedule investigation within 4 hours".to_string(),
                "Monitor downstream metrics".to_string(),
            ],
            EventSeverity::Low => vec![
                "Log issue for next review".to_string(),
                "Monitor for escalation".to_string(),
            ],
        }
    }
}

impl Default for DataQualityImpactAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityImpactReport {
    pub source_column: String,
    pub affected_downstream_columns: Vec<String>,
    pub affected_dashboards: u32,
    pub affected_models: u32,
    pub estimated_recovery_hours: u32,
    pub severity: EventSeverity,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventCascadeReport {
    pub event_id: String,
    pub event_type: QualityEventType,
    pub root_cause: String,
    pub cascade_impact: QualityImpactReport,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_lineage_tracker() {
        let mut tracker = ColumnLineageTracker::new();
        tracker
            .track_column(
                "raw.customers.id".to_string(),
                "stg.customers.customer_id".to_string(),
                None,
            )
            .unwrap();

        assert!(!tracker.lineage_map.is_empty());
    }

    #[test]
    fn test_find_downstream_impact() {
        let mut tracker = ColumnLineageTracker::new();
        tracker
            .track_column(
                "raw.orders.amount".to_string(),
                "fact.orders.revenue".to_string(),
                Some("CAST(amount AS DECIMAL)".to_string()),
            )
            .unwrap();

        let downstream = tracker.find_downstream_impact("raw.orders.amount");
        assert!(!downstream.is_empty());
    }

    #[test]
    fn test_quality_impact_analyzer() {
        let mut analyzer = DataQualityImpactAnalyzer::new();
        analyzer
            .lineage_tracker
            .track_column(
                "raw.users.email".to_string(),
                "mart.email_list.email".to_string(),
                None,
            )
            .unwrap();

        let impact = analyzer
            .analyze_quality_degradation("raw.users.email")
            .unwrap();

        assert!(!impact.affected_downstream_columns.is_empty());
    }

    #[test]
    fn test_quality_event_cascade() {
        let analyzer = DataQualityImpactAnalyzer::new();

        let event = DataQualityEvent {
            event_id: "qev_001".to_string(),
            event_type: QualityEventType::NullValuesDetected,
            table_name: "raw".to_string(),
            column_name: "orders".to_string(),
            severity: EventSeverity::Critical,
            timestamp: chrono::Utc::now(),
            details: "10% null values in orders table".to_string(),
        };

        let cascade = analyzer.process_quality_event(&event);
        assert!(cascade.is_ok());
    }
}
