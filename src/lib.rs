#![doc = include_str!("../README.md")]
#![allow(dead_code)]

pub mod graph;
pub mod discovery;
pub mod analysis;
pub mod models;
pub mod utils;
pub mod error;
pub mod api;

pub use graph::{Graph, Node, Edge, NodeType, EdgeType};
pub use analysis::{BlastRadius, BlastRadiusAnalyzer, CriticalityScorer, Simulator, FailureType};
pub use models::{Service, Database, Pipeline, DiscoveryConfig};
pub use error::{Error, Result};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");

pub fn about() -> String {
    format!("{} v{}\nOperational Blast Radius Intelligence Platform", NAME, VERSION)
}
