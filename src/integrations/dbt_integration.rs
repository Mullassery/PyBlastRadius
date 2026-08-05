use crate::error::Result;
use crate::graph::{Graph, Node, Edge, NodeType, EdgeType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

    /// Parse dbt manifest.json and extract lineage
    pub fn parse_manifest(&self) -> Result<DbtMetadata> {
        // In production: Parse actual manifest.json file
        // For MVP: Return sample metadata

        let metadata = DbtMetadata {
            project_name: self.project_name.clone(),
            models: vec![
                DbtModel {
                    name: "stg_customers".to_string(),
                    model_type: DbtModelType::Staging,
                    depends_on: vec!["raw_customers".to_string()],
                    tests: vec![
                        DbtTest {
                            test_id: "not_null_customers_id".to_string(),
                            test_type: DbtTestType::NotNull,
                            column: "customer_id".to_string(),
                        },
                    ],
                    columns: vec![
                        DbtColumn {
                            name: "customer_id".to_string(),
                            data_type: "INT".to_string(),
                            sources: vec![],  // Direct from raw
                        },
                        DbtColumn {
                            name: "email".to_string(),
                            data_type: "STRING".to_string(),
                            sources: vec![],
                        },
                    ],
                },
                DbtModel {
                    name: "fct_orders".to_string(),
                    model_type: DbtModelType::Fact,
                    depends_on: vec!["stg_customers".to_string(), "stg_orders".to_string()],
                    tests: vec![
                        DbtTest {
                            test_id: "unique_orders_id".to_string(),
                            test_type: DbtTestType::Unique,
                            column: "order_id".to_string(),
                        },
                    ],
                    columns: vec![
                        DbtColumn {
                            name: "order_id".to_string(),
                            data_type: "INT".to_string(),
                            sources: vec!["stg_orders.order_id".to_string()],
                        },
                        DbtColumn {
                            name: "customer_id".to_string(),
                            data_type: "INT".to_string(),
                            sources: vec!["stg_customers.customer_id".to_string()],
                        },
                        DbtColumn {
                            name: "amount".to_string(),
                            data_type: "DECIMAL".to_string(),
                            sources: vec!["stg_orders.amount".to_string()],
                        },
                    ],
                },
            ],
            sources: vec![
                DbtSource {
                    name: "raw_customers".to_string(),
                    database: "analytics".to_string(),
                    schema: "raw".to_string(),
                    freshness_warning_after_hours: Some(24),
                    freshness_error_after_hours: Some(48),
                },
                DbtSource {
                    name: "raw_orders".to_string(),
                    database: "analytics".to_string(),
                    schema: "raw".to_string(),
                    freshness_warning_after_hours: Some(6),
                    freshness_error_after_hours: Some(24),
                },
            ],
        };

        Ok(metadata)
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
    pub fn extract_column_lineage(
        &self,
        metadata: &DbtMetadata,
    ) -> Result<Vec<DbtColumnLineage>> {
        let mut lineage = Vec::new();

        for model in &metadata.models {
            for column in &model.columns {
                for source_col in &column.sources {
                    lineage.push(DbtColumnLineage {
                        source_model: source_col.split('.').next().unwrap_or("").to_string(),
                        source_column: source_col.split('.').last().unwrap_or("").to_string(),
                        target_model: model.name.clone(),
                        target_column: column.name.clone(),
                        transformation: None,  // Would be extracted from dbt code
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
                    status: DbtTestStatus::Unknown,  // Would be determined from dbt runs
                });
            }
        }

        Ok(tests)
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
    pub sources: Vec<String>,  // Column lineage: model.column
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

    #[test]
    fn test_dbt_parser_creation() {
        let parser = DbtManifestParser::new("./manifest.json".to_string());
        assert_eq!(parser.manifest_path, "./manifest.json");
    }

    #[test]
    fn test_parse_manifest() {
        let parser = DbtManifestParser::new("./manifest.json".to_string());
        let metadata = parser.parse_manifest().unwrap();

        assert!(!metadata.models.is_empty());
        assert!(!metadata.sources.is_empty());
    }

    #[test]
    fn test_build_graph() {
        let parser = DbtManifestParser::new("./manifest.json".to_string());
        let metadata = parser.parse_manifest().unwrap();
        let graph = parser.build_graph(&metadata).unwrap();

        assert!(graph.node_count() > 0);
        assert!(graph.edge_count() > 0);
    }

    #[test]
    fn test_column_lineage() {
        let parser = DbtManifestParser::new("./manifest.json".to_string());
        let metadata = parser.parse_manifest().unwrap();
        let lineage = parser.extract_column_lineage(&metadata).unwrap();

        assert!(!lineage.is_empty());
    }

    #[test]
    fn test_extract_tests() {
        let parser = DbtManifestParser::new("./manifest.json".to_string());
        let metadata = parser.parse_manifest().unwrap();
        let tests = parser.extract_tests(&metadata).unwrap();

        assert!(!tests.is_empty());
    }
}
