use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Graph error: {0}")]
    GraphError(String),

    #[error("Discovery error: {0}")]
    DiscoveryError(String),

    #[error("Analysis error: {0}")]
    AnalysisError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("YAML error: {0}")]
    YamlError(#[from] serde_norway::Error),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("HTTP error: {0}")]
    HttpError(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl Error {
    pub fn graph_error(msg: impl Into<String>) -> Self {
        Error::GraphError(msg.into())
    }

    pub fn discovery_error(msg: impl Into<String>) -> Self {
        Error::DiscoveryError(msg.into())
    }

    pub fn analysis_error(msg: impl Into<String>) -> Self {
        Error::AnalysisError(msg.into())
    }

    pub fn parse_error(msg: impl Into<String>) -> Self {
        Error::ParseError(msg.into())
    }

    pub fn config_error(msg: impl Into<String>) -> Self {
        Error::ConfigError(msg.into())
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Error::NotFound(msg.into())
    }

    pub fn invalid_argument(msg: impl Into<String>) -> Self {
        Error::InvalidArgument(msg.into())
    }
}
