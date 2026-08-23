pub mod airflow;
pub mod kubernetes;
#[cfg(feature = "otlp-traces")]
pub mod otlp;
pub mod terraform;

// Discovery trait
pub trait Discoverer {
    fn discover(&self) -> crate::error::Result<crate::Graph>;
}
