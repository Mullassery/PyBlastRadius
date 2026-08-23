#![doc = include_str!("../README.md")]
#![allow(dead_code)]

pub mod analysis;
pub mod api;
pub mod discovery;
pub mod error;
pub mod graph;
pub mod integrations;
pub mod models;
mod python;
pub mod utils;

pub use analysis::{BlastRadius, BlastRadiusAnalyzer, CriticalityScorer, FailureType, Simulator};
pub use discovery::airflow::AirflowDiscoverer;
pub use discovery::kubernetes::KubernetesDiscoverer;
#[cfg(feature = "otlp-traces")]
pub use discovery::otlp::OtlpDiscoverer;
pub use discovery::terraform::TerraformDiscoverer;
pub use error::{Error, Result};
pub use graph::serialization::{ExportFormat, GraphExporter, GraphImporter};
pub use graph::{Edge, EdgeType, Graph, Node, NodeType};
pub use models::{Database, DiscoveryConfig, Pipeline, Service};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");

pub fn about() -> String {
    format!(
        "{} v{}\nOperational Blast Radius Intelligence Platform",
        NAME, VERSION
    )
}
