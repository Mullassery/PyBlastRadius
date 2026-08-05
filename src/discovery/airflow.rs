use crate::error::Result;
use crate::graph::{Graph, Node, Edge, NodeType, EdgeType};
use std::collections::HashMap;
use regex::Regex;
use walkdir::WalkDir;

/// Airflow DAG parser for data pipeline dependency discovery
pub struct AirflowDiscoverer {
    pub dag_folder: String,
}

impl AirflowDiscoverer {
    pub fn new(dag_folder: String) -> Self {
        AirflowDiscoverer { dag_folder }
    }

    pub fn discover(&self) -> Result<Graph> {
        let mut graph = Graph::new();
        let mut node_map: HashMap<String, crate::graph::node::NodeId> = HashMap::new();

        // Scan DAG folder for Python files
        self.parse_dag_files(&mut graph, &mut node_map)?;

        // Add task dependencies
        self.add_task_dependencies(&mut graph, &node_map)?;

        Ok(graph)
    }

    fn parse_dag_files(
        &self,
        graph: &mut Graph,
        node_map: &mut HashMap<String, crate::graph::node::NodeId>,
    ) -> Result<()> {
        // Scan for .py files in dag_folder
        for entry in WalkDir::new(&self.dag_folder)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "py"))
        {
            self.parse_dag_file(entry.path(), graph, node_map)?;
        }

        Ok(())
    }

    fn parse_dag_file(
        &self,
        path: &std::path::Path,
        graph: &mut Graph,
        node_map: &mut HashMap<String, crate::graph::node::NodeId>,
    ) -> Result<()> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| crate::error::Error::parse_error(format!("Failed to read {}: {}", path.display(), e)))?;

        // Extract DAG name
        let dag_id = self.extract_dag_id(&content)?;

        // Create DAG node
        let dag_node = Node::pipeline(&dag_id)
            .with_criticality(0.8)
            .add_discovered_via("airflow-dag");

        let dag_id_obj = graph.add_node(dag_node)?;
        node_map.insert(format!("dag.{}", dag_id), dag_id_obj);

        // Extract operators and external dependencies
        self.extract_operators(&content, graph, node_map, &dag_id)?;

        Ok(())
    }

    fn extract_dag_id(&self, content: &str) -> Result<String> {
        // Extract DAG ID from Airflow DAG() constructor
        if let Ok(re) = Regex::new(r#"dag\s*=\s*DAG\s*\(\s*["\'](\w+)["\']"#) {
            if let Some(cap) = re.captures(content) {
                if let Some(dag_id) = cap.get(1) {
                    return Ok(dag_id.as_str().to_string());
                }
            }
        }

        Ok("unknown_dag".to_string())
    }

    fn extract_operators(
        &self,
        content: &str,
        graph: &mut Graph,
        node_map: &mut HashMap<String, crate::graph::node::NodeId>,
        dag_id: &str,
    ) -> Result<()> {
        // Extract task definitions
        // Look for common patterns like:
        // task1 = BashOperator(...)
        // task2 = PythonOperator(...)
        // task1 >> task2  (dependencies)

        if let Ok(re) = Regex::new(r"(\w+)\s*=\s*(\w+Operator)") {
            for cap in re.captures_iter(content) {
                if let Some(task_name) = cap.get(1) {
                    let full_name = format!("{}.{}", dag_id, task_name.as_str());

                    let node = Node::pipeline(&full_name)
                        .with_criticality(0.6)
                        .add_discovered_via("airflow-operator");

                    let node_id = graph.add_node(node)?;
                    node_map.insert(full_name, node_id);
                }
            }
        }

        // Extract external dependencies (databases, APIs, etc.)
        self.extract_external_deps(content, graph, node_map)?;

        Ok(())
    }

    fn extract_external_deps(
        &self,
        content: &str,
        graph: &mut Graph,
        node_map: &mut HashMap<String, crate::graph::node::NodeId>,
    ) -> Result<()> {
        // Look for connection strings in SQL or Python operators
        // PostgresOperator(sql="SELECT * FROM ...")
        // MySqlOperator(sql="SELECT * FROM ...")

        let databases = vec![
            (r"postgres", "postgresql", NodeType::Database),
            (r"mysql", "mysql_db", NodeType::Database),
            (r"snowflake", "snowflake_warehouse", NodeType::Database),
        ];

        for (pattern, name, node_type) in databases {
            if content.contains(pattern) {
                if !node_map.contains_key(name) {
                    let node = Node::new(name, node_type)
                        .with_criticality(0.85)
                        .add_discovered_via("airflow-external");

                    let node_id = graph.add_node(node)?;
                    node_map.insert(name.to_string(), node_id);
                }
            }
        }

        Ok(())
    }

    fn add_task_dependencies(
        &self,
        graph: &mut Graph,
        node_map: &HashMap<String, crate::graph::node::NodeId>,
    ) -> Result<()> {
        // Add edges for task dependencies
        // This would be extracted from >> and << operators in DAG code

        // For MVP: example dependencies
        let dependencies = vec![
            ("example_dag.load_data", "example_dag.transform", EdgeType::DependsOn),
            ("example_dag.transform", "example_dag.publish", EdgeType::DependsOn),
            ("example_dag.publish", "postgresql", EdgeType::Feeds),
        ];

        for (source, target, edge_type) in dependencies {
            if let (Some(&source_id), Some(&target_id)) = (node_map.get(source), node_map.get(target)) {
                let edge = Edge::new(edge_type, source_id, target_id)
                    .with_criticality(true)
                    .add_discovered_via("airflow");

                graph.add_edge(edge)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_airflow_discovery() {
        let discoverer = AirflowDiscoverer::new("./dags".to_string());
        let graph = discoverer.discover();

        // May be empty if no DAGs, but should not error
        assert!(graph.is_ok());
    }
}
