use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub name: String,
    pub namespace: Option<String>,
    pub version: Option<String>,
    pub owner: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Database {
    pub name: String,
    pub db_type: String, // postgres, mysql, mongodb, etc
    pub host: Option<String>,
    pub port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub name: String,
    pub orchestrator: String, // airflow, dbt, spark, etc
    pub schedule: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    pub kubernetes: Option<KubernetesConfig>,
    pub terraform: Option<TerraformConfig>,
    pub airflow: Option<AirflowConfig>,
    pub otlp: Option<OtlpConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KubernetesConfig {
    pub kubeconfig_path: Option<String>,
    pub context: Option<String>,
    pub namespaces: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerraformConfig {
    pub root_dir: String,
    pub state_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirflowConfig {
    pub dag_folder: String,
    pub airflow_home: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtlpConfig {
    pub endpoint: String,
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessImpactConfig {
    pub mappings: HashMap<String, BusinessImpactMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessImpactMapping {
    pub technical_failure: String,
    pub business_impact: String,
    pub affected_metrics: Vec<String>,
    pub financial_impact: Option<String>,
}
