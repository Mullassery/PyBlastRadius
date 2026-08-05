use crate::error::Result;

pub struct TerraformDiscoverer {
    pub root_dir: String,
}

impl TerraformDiscoverer {
    pub fn new(root_dir: String) -> Self {
        TerraformDiscoverer { root_dir }
    }

    pub fn discover(&self) -> Result<crate::Graph> {
        // TODO: Implement Terraform HCL parsing
        Ok(crate::Graph::new())
    }
}
