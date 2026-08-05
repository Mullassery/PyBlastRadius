use pyo3::prelude::*;
use std::collections::HashMap;
use crate::graph::{Graph, node::{Node, NodeId, NodeType}, edge::{Edge, EdgeType}};

#[pyclass]
pub struct PyGraph {
    inner: Graph,
    name_to_id: HashMap<String, NodeId>,
}

#[pymethods]
impl PyGraph {
    #[new]
    fn new() -> Self {
        PyGraph {
            inner: Graph::new(),
            name_to_id: HashMap::new(),
        }
    }

    fn add_node(&mut self, name: String, node_type_str: String) -> PyResult<()> {
        let node_type = match node_type_str.as_str() {
            "Service" => NodeType::Service,
            "Database" => NodeType::Database,
            "Pipeline" => NodeType::Pipeline,
            "Topic" => NodeType::Topic,
            "VectorDb" => NodeType::VectorDb,
            "Dashboard" => NodeType::Dashboard,
            "Agent" => NodeType::Agent,
            "ExternalService" => NodeType::ExternalService,
            "Table" => NodeType::Table,
            "Cache" => NodeType::Cache,
            "Queue" => NodeType::Queue,
            "Module" => NodeType::Module,
            "Test" => NodeType::Test,
            _ => return Err(pyo3::exceptions::PyValueError::new_err(
                format!("Unknown node type: {}", node_type_str)
            )),
        };

        let node = Node::new(name.clone(), node_type);
        let node_id = node.id;
        self.inner.add_node(node)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        self.name_to_id.insert(name, node_id);
        Ok(())
    }

    fn add_edge(&mut self, source_name: String, target_name: String, edge_type_str: String) -> PyResult<()> {
        let source_id = self.name_to_id.get(&source_name)
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err(
                format!("Source node '{}' not found", source_name)
            ))?
            .clone();

        let target_id = self.name_to_id.get(&target_name)
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err(
                format!("Target node '{}' not found", target_name)
            ))?
            .clone();

        let edge_type = match edge_type_str.as_str() {
            "Calls" => EdgeType::Calls,
            "Produces" => EdgeType::Produces,
            "Consumes" => EdgeType::Consumes,
            "Queries" => EdgeType::Queries,
            "Feeds" => EdgeType::Feeds,
            "DependsOn" => EdgeType::DependsOn,
            "Transforms" => EdgeType::Transforms,
            "ExportsTo" => EdgeType::ExportsTo,
            "Retrieves" => EdgeType::Retrieves,
            "Imports" => EdgeType::Imports,
            "Tests" => EdgeType::Tests,
            _ => return Err(pyo3::exceptions::PyValueError::new_err(
                format!("Unknown edge type: {}", edge_type_str)
            )),
        };

        let edge = Edge::new(edge_type, source_id, target_id);
        self.inner.add_edge(edge)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(())
    }

    fn node_count(&self) -> usize {
        self.inner.node_count()
    }

    fn edge_count(&self) -> usize {
        self.inner.edge_count()
    }

    fn get_neighbors(&self, name: String) -> PyResult<Vec<String>> {
        let node_id = self.name_to_id.get(&name)
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err(
                format!("Node '{}' not found", name)
            ))?;

        let neighbor_ids = self.inner.get_neighbors(node_id)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        let names: Vec<String> = neighbor_ids.iter()
            .filter_map(|id| self.inner.get_node(id).map(|n| n.name.clone()))
            .collect();
        Ok(names)
    }

    fn get_predecessors(&self, name: String) -> PyResult<Vec<String>> {
        let node_id = self.name_to_id.get(&name)
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err(
                format!("Node '{}' not found", name)
            ))?;

        let pred_ids = self.inner.get_predecessors(node_id)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        let names: Vec<String> = pred_ids.iter()
            .filter_map(|id| self.inner.get_node(id).map(|n| n.name.clone()))
            .collect();
        Ok(names)
    }

    fn get_node_type(&self, name: String) -> PyResult<String> {
        let node_id = self.name_to_id.get(&name)
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err(
                format!("Node '{}' not found", name)
            ))?;

        let node = self.inner.get_node(node_id)
            .ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("Node not found"))?;

        let type_str = match node.node_type {
            NodeType::Service => "Service",
            NodeType::Database => "Database",
            NodeType::Pipeline => "Pipeline",
            NodeType::Topic => "Topic",
            NodeType::VectorDb => "VectorDb",
            NodeType::Dashboard => "Dashboard",
            NodeType::Agent => "Agent",
            NodeType::ExternalService => "ExternalService",
            NodeType::Table => "Table",
            NodeType::Cache => "Cache",
            NodeType::Queue => "Queue",
            NodeType::Module => "Module",
            NodeType::Test => "Test",
        };
        Ok(type_str.to_string())
    }

    fn has_path(&self, from: String, to: String) -> PyResult<bool> {
        let from_id = self.name_to_id.get(&from)
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err(
                format!("Node '{}' not found", from)
            ))?;

        let to_id = self.name_to_id.get(&to)
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err(
                format!("Node '{}' not found", to)
            ))?;

        let has_path = self.inner.has_path(from_id, to_id)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(has_path)
    }

    fn list_nodes(&self) -> Vec<String> {
        self.inner.nodes()
            .map(|n| n.name.clone())
            .collect()
    }

    fn __repr__(&self) -> String {
        format!("PyGraph with {} nodes and {} edges",
            self.node_count(),
            self.edge_count())
    }
}

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyGraph>()?;
    Ok(())
}
