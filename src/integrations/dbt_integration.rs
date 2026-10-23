use crate::error::Result;
use crate::graph::{Edge, EdgeType, Graph, Node, NodeType};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;

/// dbt manifest parser for comprehensive data pipeline discovery
/// Extracts model dependencies, tests, and lineage from dbt projects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbtManifestParser {
    pub manifest_path: String,
    pub project_name: String,
}

impl DbtManifestParser {
    pub fn new(manifest_path: String) -> Self {
        DbtManifestParser {
            manifest_path,
            project_name: "dbt_project".to_string(),
        }
    }

    /// Parse a real dbt ``manifest.json`` and extract lineage.
    ///
    /// The parser reads the artifact dbt writes to ``target/manifest.json``;
    /// it does not require a live database and never substitutes sample data
    /// when the file is missing or malformed.
    pub fn parse_manifest(&self) -> Result<DbtMetadata> {
        let raw = fs::read_to_string(&self.manifest_path)?;
        let manifest: Value = serde_json::from_str(&raw)?;
        self.parse_manifest_value(&manifest)
    }

    fn parse_manifest_value(&self, manifest: &Value) -> Result<DbtMetadata> {
        let project_name = manifest
            .get("metadata")
            .and_then(|metadata| metadata.get("project_name"))
            .and_then(Value::as_str)
            .unwrap_or(&self.project_name)
            .to_string();

        let mut sources = Vec::new();
        let mut node_names: HashMap<String, String> = HashMap::new();
        if let Some(source_nodes) = manifest.get("sources").and_then(Value::as_object) {
            for (unique_id, source) in source_nodes {
                let Some(name) = string_field(source, "name") else {
                    continue;
                };
                node_names.insert(unique_id.clone(), name.clone());
                sources.push(DbtSource {
                    name,
                    database: string_field(source, "database").unwrap_or_default(),
                    schema: string_field(source, "schema").unwrap_or_default(),
                    // Source freshness thresholds live in the project config,
                    // not in manifest.json. Leave them unset instead of
                    // inventing defaults.
                    freshness_warning_after_hours: None,
                    freshness_error_after_hours: None,
                });
            }
        }

        let mut models = Vec::new();
        if let Some(nodes) = manifest.get("nodes").and_then(Value::as_object) {
            for (unique_id, node) in nodes {
                if node.get("resource_type").and_then(Value::as_str) != Some("model") {
                    continue;
                }
                let Some(name) = string_field(node, "name") else {
                    continue;
                };
                node_names.insert(unique_id.clone(), name.clone());
                let depends_on = node
                    .get("depends_on")
                    .and_then(|value| value.get("nodes"))
                    .and_then(Value::as_array)
                    .map(|nodes| {
                        nodes
                            .iter()
                            .filter_map(Value::as_str)
                            .map(|dependency| {
                                node_names
                                    .get(dependency)
                                    .cloned()
                                    .unwrap_or_else(|| short_node_name(dependency))
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let columns = node
                    .get("columns")
                    .and_then(Value::as_object)
                    .map(|columns| {
                        columns
                            .iter()
                            .map(|(column_name, column)| DbtColumn {
                                name: string_field(column, "name")
                                    .unwrap_or_else(|| column_name.clone()),
                                data_type: string_field(column, "data_type")
                                    .unwrap_or_else(|| "unknown".to_string()),
                                sources: column
                                    .get("meta")
                                    .and_then(|meta| meta.get("sources"))
                                    .and_then(Value::as_array)
                                    .map(|sources| {
                                        sources
                                            .iter()
                                            .filter_map(Value::as_str)
                                            .map(str::to_string)
                                            .collect()
                                    })
                                    .unwrap_or_default(),
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                let model = DbtModel {
                    name: name.clone(),
                    model_type: classify_model(&name, node),
                    depends_on,
                    tests: Vec::new(),
                    columns,
                };
                models.push(model);
            }
        }
        models.sort_by(|left, right| left.name.cmp(&right.name));
        let model_index: HashMap<String, usize> = models
            .iter()
            .enumerate()
            .map(|(index, model)| (model.name.clone(), index))
            .collect();

        // dbt tests are separate nodes. Attach generic/singular tests to the
        // model or source they depend on so callers get them per model.
        let mut model_tests: Vec<(usize, DbtTest)> = Vec::new();
        if let Some(nodes) = manifest.get("nodes").and_then(Value::as_object) {
            for (unique_id, node) in nodes {
                if node.get("resource_type").and_then(Value::as_str) != Some("test") {
                    continue;
                }
                let Some(column) = test_column(node) else {
                    continue;
                };
                let test_type = test_type(node);
                let Some(target) = node
                    .get("depends_on")
                    .and_then(|value| value.get("nodes"))
                    .and_then(Value::as_array)
                    .and_then(|deps| {
                        deps.iter()
                            .filter_map(Value::as_str)
                            .map(short_node_name)
                            .find(|name| model_index.contains_key(name))
                    })
                else {
                    continue;
                };
                if let Some(&index) = model_index.get(&target) {
                    model_tests.push((
                        index,
                        DbtTest {
                            test_id: unique_id.clone(),
                            test_type,
                            column,
                        },
                    ));
                }
            }
        }
        for (index, test) in model_tests {
            models[index].tests.push(test);
        }

        sources.sort_by(|left, right| left.name.cmp(&right.name));

        Ok(DbtMetadata {
            project_name,
            models,
            sources,
        })
    }

    /// Build graph from dbt metadata
    pub fn build_graph(&self, metadata: &DbtMetadata) -> Result<Graph> {
        let mut graph = Graph::new();
        let mut node_map: HashMap<String, crate::graph::node::NodeId> = HashMap::new();

        // Create nodes for sources
        for source in &metadata.sources {
            let node = Node::database(&source.name)
                .with_criticality(0.9)
                .add_discovered_via("dbt-source");

            let node_id = graph.add_node(node)?;
            node_map.insert(source.name.clone(), node_id);
        }

        // Create nodes for models
        for model in &metadata.models {
            let node_type = match model.model_type {
                DbtModelType::Staging => NodeType::Pipeline,
                DbtModelType::Intermediate => NodeType::Pipeline,
                DbtModelType::Fact => NodeType::Table,
                DbtModelType::Dimension => NodeType::Table,
                DbtModelType::Mart => NodeType::Table,
            };

            let criticality = match model.model_type {
                DbtModelType::Fact => 0.95,
                DbtModelType::Mart => 0.9,
                _ => 0.7,
            };

            let node = Node::new(&model.name, node_type)
                .with_criticality(criticality)
                .add_discovered_via("dbt-model");

            let node_id = graph.add_node(node)?;
            node_map.insert(model.name.clone(), node_id);
        }

        // Create edges for dependencies
        for model in &metadata.models {
            if let Some(&target_id) = node_map.get(&model.name) {
                for dep in &model.depends_on {
                    if let Some(&source_id) = node_map.get(dep) {
                        let edge = Edge::new(EdgeType::Feeds, source_id, target_id)
                            .with_criticality(true)
                            .add_discovered_via("dbt-dependency");

                        graph.add_edge(edge)?;
                    }
                }
            }
        }

        Ok(graph)
    }

    /// Extract column-level lineage (which source columns feed into target columns)
    pub fn extract_column_lineage(&self, metadata: &DbtMetadata) -> Result<Vec<DbtColumnLineage>> {
        let mut lineage = Vec::new();

        for model in &metadata.models {
            for column in &model.columns {
                for source_col in &column.sources {
                    lineage.push(DbtColumnLineage {
                        source_model: source_col.split('.').next().unwrap_or("").to_string(),
                        source_column: source_col.split('.').next_back().unwrap_or("").to_string(),
                        target_model: model.name.clone(),
                        target_column: column.name.clone(),
                        transformation: None, // Would be extracted from dbt code
                    });
                }
            }
        }

        Ok(lineage)
    }

    /// Get all tests defined in dbt project
    pub fn extract_tests(&self, metadata: &DbtMetadata) -> Result<Vec<DbtTestSummary>> {
        let mut tests = Vec::new();

        for model in &metadata.models {
            for test in &model.tests {
                tests.push(DbtTestSummary {
                    test_id: test.test_id.clone(),
                    model_name: model.name.clone(),
                    test_type: test.test_type,
                    column: test.column.clone(),
                    status: DbtTestStatus::Unknown, // Would be determined from dbt runs
                });
            }
        }

        Ok(tests)
    }
}

fn string_field(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn short_node_name(unique_id: &str) -> String {
    unique_id
        .rsplit('.')
        .next()
        .unwrap_or(unique_id)
        .to_string()
}

fn classify_model(name: &str, node: &Value) -> DbtModelType {
    if let Some(explicit) = node
        .get("meta")
        .and_then(|meta| meta.get("model_type"))
        .and_then(Value::as_str)
    {
        match explicit {
            "staging" => return DbtModelType::Staging,
            "intermediate" => return DbtModelType::Intermediate,
            "fact" => return DbtModelType::Fact,
            "dimension" => return DbtModelType::Dimension,
            "mart" => return DbtModelType::Mart,
            _ => {}
        }
    }
    if name.starts_with("stg_") {
        DbtModelType::Staging
    } else if name.starts_with("int_") {
        DbtModelType::Intermediate
    } else if name.starts_with("fct_") {
        DbtModelType::Fact
    } else if name.starts_with("dim_") {
        DbtModelType::Dimension
    } else {
        DbtModelType::Mart
    }
}

fn test_column(node: &Value) -> Option<String> {
    string_field(node, "column_name").or_else(|| {
        node.get("test_metadata")
            .and_then(|metadata| metadata.get("kwargs"))
            .and_then(|kwargs| kwargs.get("column_name"))
            .and_then(Value::as_str)
            .map(str::to_string)
    })
}

fn test_type(node: &Value) -> DbtTestType {
    match node
        .get("test_metadata")
        .and_then(|metadata| metadata.get("name"))
        .and_then(Value::as_str)
        .unwrap_or_default()
    {
        "not_null" => DbtTestType::NotNull,
        "unique" => DbtTestType::Unique,
        "accepted_values" => DbtTestType::Accepted,
        "relationships" => DbtTestType::Relationships,
        _ => DbtTestType::Custom,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbtMetadata {
    pub project_name: String,
    pub models: Vec<DbtModel>,
    pub sources: Vec<DbtSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbtModel {
    pub name: String,
    pub model_type: DbtModelType,
    pub depends_on: Vec<String>,
    pub tests: Vec<DbtTest>,
    pub columns: Vec<DbtColumn>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DbtModelType {
    Staging,      // stg_*
    Intermediate, // int_*
    Fact,         // fct_*
    Dimension,    // dim_*
    Mart,         // Custom schemas
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbtColumn {
    pub name: String,
    pub data_type: String,
    pub sources: Vec<String>, // Column lineage: model.column
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbtSource {
    pub name: String,
    pub database: String,
    pub schema: String,
    pub freshness_warning_after_hours: Option<u32>,
    pub freshness_error_after_hours: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbtTest {
    pub test_id: String,
    pub test_type: DbtTestType,
    pub column: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DbtTestType {
    NotNull,
    Unique,
    Accepted,
    Relationships,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbtColumnLineage {
    pub source_model: String,
    pub source_column: String,
    pub target_model: String,
    pub target_column: String,
    pub transformation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbtTestSummary {
    pub test_id: String,
    pub model_name: String,
    pub test_type: DbtTestType,
    pub column: String,
    pub status: DbtTestStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DbtTestStatus {
    Pass,
    Fail,
    Unknown,
}

/// Lineage tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbtModelLineage {
    pub model_name: String,
    pub upstream: Vec<String>,   // Dependencies
    pub downstream: Vec<String>, // Dependents
    pub impact_depth: usize,     // How many layers deep does impact go
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn write_manifest() -> (tempfile::TempDir, String) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("manifest.json");
        let manifest = json!({
            "metadata": {"project_name": "analytics"},
            "sources": {
                "source.analytics.raw_customers": {
                    "resource_type": "source",
                    "name": "raw_customers",
                    "database": "analytics",
                    "schema": "raw"
                }
            },
            "nodes": {
                "model.analytics.stg_customers": {
                    "resource_type": "model",
                    "name": "stg_customers",
                    "depends_on": {"nodes": ["source.analytics.raw_customers"]},
                    "columns": {
                        "customer_id": {"name": "customer_id", "data_type": "INT"}
                    }
                },
                "model.analytics.fct_orders": {
                    "resource_type": "model",
                    "name": "fct_orders",
                    "depends_on": {"nodes": ["model.analytics.stg_customers"]},
                    "columns": {
                        "order_id": {
                            "name": "order_id",
                            "data_type": "INT",
                            "meta": {"sources": ["stg_customers.customer_id"]}
                        }
                    }
                },
                "test.analytics.not_null_stg_customers_customer_id": {
                    "resource_type": "test",
                    "name": "not_null_stg_customers_customer_id",
                    "depends_on": {"nodes": ["model.analytics.stg_customers"]},
                    "column_name": "customer_id",
                    "test_metadata": {
                        "name": "not_null",
                        "kwargs": {"column_name": "customer_id"}
                    }
                }
            }
        });
        fs::write(&path, serde_json::to_vec(&manifest).unwrap()).expect("write manifest");
        let path = path.to_string_lossy().to_string();
        (dir, path)
    }

    #[test]
    fn test_dbt_parser_creation() {
        let parser = DbtManifestParser::new("./manifest.json".to_string());
        assert_eq!(parser.manifest_path, "./manifest.json");
    }

    #[test]
    fn test_parse_manifest_reads_real_nodes() {
        let (_dir, path) = write_manifest();
        let parser = DbtManifestParser::new(path);
        let metadata = parser.parse_manifest().unwrap();

        assert_eq!(metadata.project_name, "analytics");
        assert_eq!(metadata.models.len(), 2);
        assert_eq!(metadata.sources.len(), 1);
        assert_eq!(metadata.sources[0].name, "raw_customers");
        let staging = metadata
            .models
            .iter()
            .find(|model| model.name == "stg_customers")
            .expect("staging model");
        let fact = metadata
            .models
            .iter()
            .find(|model| model.name == "fct_orders")
            .expect("fact model");
        assert_eq!(staging.model_type, DbtModelType::Staging);
        assert_eq!(fact.model_type, DbtModelType::Fact);
        assert_eq!(fact.depends_on, vec!["stg_customers"]);
        assert_eq!(staging.tests.len(), 1);
        assert_eq!(staging.tests[0].test_type, DbtTestType::NotNull);
    }

    #[test]
    fn test_missing_manifest_is_an_error_not_sample_data() {
        let parser = DbtManifestParser::new("/nonexistent/manifest.json".to_string());
        assert!(parser.parse_manifest().is_err());
    }

    #[test]
    fn test_build_graph() {
        let (_dir, path) = write_manifest();
        let parser = DbtManifestParser::new(path);
        let metadata = parser.parse_manifest().unwrap();
        let graph = parser.build_graph(&metadata).unwrap();

        assert_eq!(graph.node_count(), 3);
        assert_eq!(graph.edge_count(), 2);
    }

    #[test]
    fn test_column_lineage_from_manifest_metadata() {
        let (_dir, path) = write_manifest();
        let parser = DbtManifestParser::new(path);
        let metadata = parser.parse_manifest().unwrap();
        let lineage = parser.extract_column_lineage(&metadata).unwrap();

        assert_eq!(lineage.len(), 1);
        assert_eq!(lineage[0].source_model, "stg_customers");
        assert_eq!(lineage[0].source_column, "customer_id");
        assert_eq!(lineage[0].target_model, "fct_orders");
        assert_eq!(lineage[0].target_column, "order_id");
    }

    #[test]
    fn test_extract_tests() {
        let (_dir, path) = write_manifest();
        let parser = DbtManifestParser::new(path);
        let metadata = parser.parse_manifest().unwrap();
        let tests = parser.extract_tests(&metadata).unwrap();

        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].model_name, "stg_customers");
        assert_eq!(tests[0].column, "customer_id");
    }
}
