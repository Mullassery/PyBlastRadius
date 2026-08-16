#![doc = include_str!("../README.md")]
#![allow(dead_code)]

pub mod graph;
pub mod discovery;
pub mod analysis;
pub mod models;
pub mod utils;
pub mod error;
pub mod api;
pub mod integrations;
mod python;

pub use graph::{Graph, Node, Edge, NodeType, EdgeType};
pub use graph::serialization::{GraphExporter, GraphImporter, ExportFormat};
pub use analysis::{BlastRadius, BlastRadiusAnalyzer, CriticalityScorer, Simulator, FailureType};
pub use models::{Service, Database, Pipeline, DiscoveryConfig};
pub use error::{Error, Result};
pub use discovery::kubernetes::KubernetesDiscoverer;
pub use discovery::terraform::TerraformDiscoverer;
pub use discovery::airflow::AirflowDiscoverer;
#[cfg(feature = "otlp-traces")]
pub use discovery::otlp::OtlpDiscoverer;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");

pub fn about() -> String {
    format!("{} v{}\nOperational Blast Radius Intelligence Platform", NAME, VERSION)
}
