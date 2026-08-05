use crate::error::Result;

pub struct AirflowDiscoverer {
    pub dag_folder: String,
}

impl AirflowDiscoverer {
    pub fn new(dag_folder: String) -> Self {
        AirflowDiscoverer { dag_folder }
    }

    pub fn discover(&self) -> Result<crate::Graph> {
        // TODO: Implement Airflow DAG parsing
        Ok(crate::Graph::new())
    }
}
