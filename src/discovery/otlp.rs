use crate::error::Result;

pub struct OtlpDiscoverer {
    pub endpoint: String,
}

impl OtlpDiscoverer {
    pub fn new(endpoint: String) -> Self {
        OtlpDiscoverer { endpoint }
    }

    pub fn discover(&self) -> Result<crate::Graph> {
        // TODO: Implement OTLP trace consumer
        Ok(crate::Graph::new())
    }
}
