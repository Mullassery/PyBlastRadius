pub mod kubernetes;
pub mod terraform;
pub mod airflow;
pub mod otlp;

// Discovery trait
pub trait Discoverer {
    fn discover(&self) -> crate::error::Result<crate::Graph>;
}
