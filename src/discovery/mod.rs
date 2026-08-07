pub mod kubernetes;
pub mod terraform;
pub mod airflow;
#[cfg(feature = "otlp-traces")]
pub mod otlp;

// Discovery trait
pub trait Discoverer {
    fn discover(&self) -> crate::error::Result<crate::Graph>;
}
