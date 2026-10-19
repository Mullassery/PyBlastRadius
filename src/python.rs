//! PyO3 bindings exposing the Rust graph engine to Python as `pyblastradius._core`.
//!
//! This is the module that was missing entirely before: the Rust core (graph
//! storage, blast-radius analysis, criticality scoring, cascade simulation)
//! was fully implemented and tested, but nothing in the crate ever compiled
//! it into a `#[pymodule]`, so `import pyblastradius` could never succeed.
//!
//! The bindings here are thin wrappers: every function delegates to the real
//! Rust implementations in `crate::graph` and `crate::analysis` rather than
//! reimplementing any logic in Python or in this module.

use std::collections::HashMap;

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::analysis::{
    BlastRadiusAnalyzer as RBlastRadiusAnalyzer, BlastRadiusResult as RBlastRadiusResult,
    CriticalityScore as RCriticalityScore, CriticalityScorer as RCriticalityScorer,
    FailureType as RFailureType, SimulationResult as RSimulationResult, Simulator as RSimulator,
};
use crate::api::integrations::{
    PagerDutyIntegration as RPagerDutyIntegration, SlackIntegration as RSlackIntegration,
};
use crate::error::Error as CoreError;
use crate::graph::edge::EdgeProperties;
use crate::graph::node::{HealthStatus as RHealthStatus, NodeId, NodeProperties};
use crate::graph::serialization::{GraphExporter, GraphImporter};
use crate::graph::{
    Edge as REdge, EdgeType as REdgeType, Graph as RGraph, Node as RNode, NodeType as RNodeType,
};

fn core_err_to_py(e: CoreError) -> PyErr {
    PyRuntimeError::new_err(e.to_string())
}

/// Drive an async Rust future from a synchronous Python method.
///
/// The webhook clients are `async` because they use `reqwest`; Python callers
/// get ordinary blocking methods backed by a fresh current-thread runtime.
fn block_on_raw<F>(future: F) -> Result<F::Output, String>
where
    F: std::future::Future,
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("failed to build tokio runtime: {e}"))
        .map(|runtime| runtime.block_on(future))
}

fn block_on<F>(future: F) -> PyResult<F::Output>
where
    F: std::future::Future,
{
    Ok(tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| PyRuntimeError::new_err(format!("failed to build tokio runtime: {e}")))?
        .block_on(future))
}

fn parse_node_id(s: &str) -> PyResult<NodeId> {
    uuid::Uuid::parse_str(s)
        .map(NodeId)
        .map_err(|e| PyValueError::new_err(format!("'{s}' is not a valid node id: {e}")))
}

// ---------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------

#[pyclass(eq, eq_int, from_py_object, name = "NodeType")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyNodeType {
    Service,
    Database,
    Pipeline,
    Topic,
    VectorDb,
    Dashboard,
    Agent,
    ExternalService,
    Table,
    Cache,
    Queue,
}

impl From<RNodeType> for PyNodeType {
    fn from(t: RNodeType) -> Self {
        match t {
            RNodeType::Service => PyNodeType::Service,
            RNodeType::Database => PyNodeType::Database,
            RNodeType::Pipeline => PyNodeType::Pipeline,
            RNodeType::Topic => PyNodeType::Topic,
            RNodeType::VectorDb => PyNodeType::VectorDb,
            RNodeType::Dashboard => PyNodeType::Dashboard,
            RNodeType::Agent => PyNodeType::Agent,
            RNodeType::ExternalService => PyNodeType::ExternalService,
            RNodeType::Table => PyNodeType::Table,
            RNodeType::Cache => PyNodeType::Cache,
            RNodeType::Queue => PyNodeType::Queue,
        }
    }
}

impl From<PyNodeType> for RNodeType {
    fn from(t: PyNodeType) -> Self {
        match t {
            PyNodeType::Service => RNodeType::Service,
            PyNodeType::Database => RNodeType::Database,
            PyNodeType::Pipeline => RNodeType::Pipeline,
            PyNodeType::Topic => RNodeType::Topic,
            PyNodeType::VectorDb => RNodeType::VectorDb,
            PyNodeType::Dashboard => RNodeType::Dashboard,
            PyNodeType::Agent => RNodeType::Agent,
            PyNodeType::ExternalService => RNodeType::ExternalService,
            PyNodeType::Table => RNodeType::Table,
            PyNodeType::Cache => RNodeType::Cache,
            PyNodeType::Queue => RNodeType::Queue,
        }
    }
}

#[pyclass(eq, eq_int, from_py_object, name = "EdgeType")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyEdgeType {
    Calls,
    Produces,
    Consumes,
    Queries,
    Feeds,
    DependsOn,
    Transforms,
    ExportsTo,
    Retrieves,
}

impl From<REdgeType> for PyEdgeType {
    fn from(t: REdgeType) -> Self {
        match t {
            REdgeType::Calls => PyEdgeType::Calls,
            REdgeType::Produces => PyEdgeType::Produces,
            REdgeType::Consumes => PyEdgeType::Consumes,
            REdgeType::Queries => PyEdgeType::Queries,
            REdgeType::Feeds => PyEdgeType::Feeds,
            REdgeType::DependsOn => PyEdgeType::DependsOn,
            REdgeType::Transforms => PyEdgeType::Transforms,
            REdgeType::ExportsTo => PyEdgeType::ExportsTo,
            REdgeType::Retrieves => PyEdgeType::Retrieves,
        }
    }
}

impl From<PyEdgeType> for REdgeType {
    fn from(t: PyEdgeType) -> Self {
        match t {
            PyEdgeType::Calls => REdgeType::Calls,
            PyEdgeType::Produces => REdgeType::Produces,
            PyEdgeType::Consumes => REdgeType::Consumes,
            PyEdgeType::Queries => REdgeType::Queries,
            PyEdgeType::Feeds => REdgeType::Feeds,
            PyEdgeType::DependsOn => REdgeType::DependsOn,
            PyEdgeType::Transforms => REdgeType::Transforms,
            PyEdgeType::ExportsTo => REdgeType::ExportsTo,
            PyEdgeType::Retrieves => REdgeType::Retrieves,
        }
    }
}

#[pyclass(eq, eq_int, from_py_object, name = "HealthStatus")]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyHealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

impl From<RHealthStatus> for PyHealthStatus {
    fn from(s: RHealthStatus) -> Self {
        match s {
            RHealthStatus::Healthy => PyHealthStatus::Healthy,
            RHealthStatus::Degraded => PyHealthStatus::Degraded,
            RHealthStatus::Unhealthy => PyHealthStatus::Unhealthy,
            RHealthStatus::Unknown => PyHealthStatus::Unknown,
        }
    }
}

impl From<PyHealthStatus> for RHealthStatus {
    fn from(s: PyHealthStatus) -> Self {
        match s {
            PyHealthStatus::Healthy => RHealthStatus::Healthy,
            PyHealthStatus::Degraded => RHealthStatus::Degraded,
            PyHealthStatus::Unhealthy => RHealthStatus::Unhealthy,
            PyHealthStatus::Unknown => RHealthStatus::Unknown,
        }
    }
}

#[pyclass(eq, eq_int, from_py_object, name = "FailureType")]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyFailureType {
    Outage,
    Degradation,
    HighLatency,
    HighErrorRate,
    DataQualityIssue,
}

impl From<PyFailureType> for RFailureType {
    fn from(f: PyFailureType) -> Self {
        match f {
            PyFailureType::Outage => RFailureType::Outage,
            PyFailureType::Degradation => RFailureType::Degradation,
            PyFailureType::HighLatency => RFailureType::HighLatency,
            PyFailureType::HighErrorRate => RFailureType::HighErrorRate,
            PyFailureType::DataQualityIssue => RFailureType::DataQualityIssue,
        }
    }
}

// ---------------------------------------------------------------------
// Node / Edge
// ---------------------------------------------------------------------

#[pyclass(from_py_object, name = "Node")]
#[derive(Clone)]
pub struct PyNode {
    pub(crate) inner: RNode,
}

#[pymethods]
impl PyNode {
    #[new]
    #[pyo3(signature = (name, node_type=PyNodeType::Service))]
    fn new(name: String, node_type: PyNodeType) -> Self {
        PyNode {
            inner: RNode::new(name, node_type.into()),
        }
    }

    #[getter]
    fn id(&self) -> String {
        self.inner.id.0.to_string()
    }

    #[getter]
    fn name(&self) -> &str {
        &self.inner.name
    }

    #[getter]
    fn node_type(&self) -> PyNodeType {
        self.inner.node_type.clone().into()
    }

    #[getter]
    fn criticality(&self) -> f32 {
        self.inner.properties.criticality
    }

    #[setter]
    fn set_criticality(&mut self, value: f32) {
        self.inner.properties.criticality = value.clamp(0.0, 1.0);
    }

    #[getter]
    fn health_status(&self) -> PyHealthStatus {
        self.inner.properties.health_status.into()
    }

    #[setter]
    fn set_health_status(&mut self, value: PyHealthStatus) {
        self.inner.properties.health_status = value.into();
    }

    #[getter]
    fn recovery_time_minutes(&self) -> u32 {
        self.inner.properties.recovery_time_minutes
    }

    #[setter]
    fn set_recovery_time_minutes(&mut self, value: u32) {
        self.inner.properties.recovery_time_minutes = value;
    }

    #[getter]
    fn business_exposure(&self) -> f32 {
        self.inner.properties.business_exposure
    }

    #[setter]
    fn set_business_exposure(&mut self, value: f32) {
        self.inner.properties.business_exposure = value.clamp(0.0, 1.0);
    }

    #[getter]
    fn owner(&self) -> Option<String> {
        self.inner.properties.owner.clone()
    }

    #[setter]
    fn set_owner(&mut self, value: Option<String>) {
        self.inner.properties.owner = value;
    }

    #[getter]
    fn confidence(&self) -> f32 {
        self.inner.properties.confidence
    }

    #[getter]
    fn tags(&self) -> HashMap<String, String> {
        self.inner.properties.tags.clone()
    }

    fn add_tag(&mut self, key: String, value: String) {
        self.inner.properties.tags.insert(key, value);
    }

    fn __repr__(&self) -> String {
        format!(
            "Node(id={:?}, name={:?}, node_type={:?})",
            self.id(),
            self.inner.name,
            self.node_type()
        )
    }
}

#[pyclass(from_py_object, name = "Edge")]
#[derive(Clone)]
pub struct PyEdge {
    pub(crate) inner: REdge,
}

#[pymethods]
impl PyEdge {
    #[new]
    #[pyo3(signature = (edge_type, source, target))]
    fn new(edge_type: PyEdgeType, source: &str, target: &str) -> PyResult<Self> {
        let source_id = parse_node_id(source)?;
        let target_id = parse_node_id(target)?;
        Ok(PyEdge {
            inner: REdge::new(edge_type.into(), source_id, target_id),
        })
    }

    #[getter]
    fn id(&self) -> String {
        self.inner.id.0.to_string()
    }

    #[getter]
    fn source(&self) -> String {
        self.inner.source.0.to_string()
    }

    #[getter]
    fn target(&self) -> String {
        self.inner.target.0.to_string()
    }

    #[getter]
    fn edge_type(&self) -> PyEdgeType {
        self.inner.edge_type.into()
    }

    #[getter]
    fn is_critical(&self) -> bool {
        self.inner.properties.is_critical
    }

    #[setter]
    fn set_is_critical(&mut self, value: bool) {
        self.inner.properties.is_critical = value;
    }

    #[getter]
    fn latency_ms(&self) -> Option<f32> {
        self.inner.properties.latency_ms
    }

    #[setter]
    fn set_latency_ms(&mut self, value: Option<f32>) {
        self.inner.properties.latency_ms = value;
    }

    #[getter]
    fn error_rate(&self) -> Option<f32> {
        self.inner.properties.error_rate
    }

    #[setter]
    fn set_error_rate(&mut self, value: Option<f32>) {
        self.inner.properties.error_rate = value.map(|v| v.clamp(0.0, 1.0));
    }

    #[getter]
    fn confidence(&self) -> f32 {
        self.inner.properties.confidence
    }

    fn __repr__(&self) -> String {
        format!(
            "Edge(source={:?}, target={:?}, edge_type={:?})",
            self.source(),
            self.target(),
            self.edge_type()
        )
    }
}

// Silence "unused" for properties structs pulled in only for type inference above.
#[allow(dead_code)]
fn _unused(_: NodeProperties, _: EdgeProperties) {}

// ---------------------------------------------------------------------
// Graph
// ---------------------------------------------------------------------

#[pyclass(name = "Graph")]
pub struct PyGraph {
    pub(crate) inner: RGraph,
}

#[pymethods]
impl PyGraph {
    #[new]
    fn new() -> Self {
        PyGraph {
            inner: RGraph::new(),
        }
    }

    /// Add a node to the graph, returning its generated id (a UUID string).
    fn add_node(&mut self, node: PyNode) -> PyResult<String> {
        let id = self.inner.add_node(node.inner).map_err(core_err_to_py)?;
        Ok(id.0.to_string())
    }

    /// Add an edge to the graph, returning its generated id (a UUID string).
    /// The edge's `source`/`target` must already exist as node ids in this graph.
    fn add_edge(&mut self, edge: PyEdge) -> PyResult<String> {
        let id = self.inner.add_edge(edge.inner).map_err(core_err_to_py)?;
        Ok(id.0.to_string())
    }

    fn get_node(&self, node_id: &str) -> PyResult<Option<PyNode>> {
        let id = parse_node_id(node_id)?;
        Ok(self
            .inner
            .get_node(&id)
            .map(|n| PyNode { inner: n.clone() }))
    }

    /// Convenience helper (not present in the Rust core, since it only keys
    /// nodes by id): linear-scan for the first node with an exact name
    /// match. Useful from Python/CLI code where users refer to services by
    /// name rather than by UUID.
    fn find_node_by_name(&self, name: &str) -> Option<PyNode> {
        self.inner
            .nodes()
            .find(|n| n.name == name)
            .map(|n| PyNode { inner: n.clone() })
    }

    fn node_count(&self) -> usize {
        self.inner.node_count()
    }

    fn edge_count(&self) -> usize {
        self.inner.edge_count()
    }

    fn nodes(&self) -> Vec<PyNode> {
        self.inner
            .nodes()
            .map(|n| PyNode { inner: n.clone() })
            .collect()
    }

    fn edges(&self) -> Vec<PyEdge> {
        self.inner
            .edges()
            .map(|e| PyEdge { inner: e.clone() })
            .collect()
    }

    fn neighbors(&self, node_id: &str) -> PyResult<Vec<String>> {
        let id = parse_node_id(node_id)?;
        let neighbors = self.inner.get_neighbors(&id).map_err(core_err_to_py)?;
        Ok(neighbors.into_iter().map(|n| n.0.to_string()).collect())
    }

    fn predecessors(&self, node_id: &str) -> PyResult<Vec<String>> {
        let id = parse_node_id(node_id)?;
        let preds = self.inner.get_predecessors(&id).map_err(core_err_to_py)?;
        Ok(preds.into_iter().map(|n| n.0.to_string()).collect())
    }

    fn has_path(&self, from: &str, to: &str) -> PyResult<bool> {
        let from_id = parse_node_id(from)?;
        let to_id = parse_node_id(to)?;
        self.inner
            .has_path(&from_id, &to_id)
            .map_err(core_err_to_py)
    }

    fn metrics<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let m = self.inner.metrics();
        let dict = PyDict::new(py);
        dict.set_item("node_count", m.node_count)?;
        dict.set_item("edge_count", m.edge_count)?;
        dict.set_item("avg_degree", m.avg_degree)?;
        dict.set_item("density", m.density)?;
        dict.set_item("connected_components", m.connected_components)?;
        Ok(dict)
    }

    /// Merge another graph's nodes/edges into this one (original ids are
    /// preserved; cross-graph entity resolution is not performed, matching
    /// the Rust `Graph::merge` this delegates to).
    fn merge(&mut self, other: &PyGraph) -> PyResult<()> {
        self.inner.merge(&other.inner).map_err(core_err_to_py)
    }

    /// Serialize the graph losslessly to JSON (round-trips via `from_json`).
    fn to_json(&self) -> PyResult<String> {
        GraphExporter::save_json(&self.inner).map_err(core_err_to_py)
    }

    #[staticmethod]
    fn from_json(json: &str) -> PyResult<PyGraph> {
        let inner = GraphImporter::load_json(json).map_err(core_err_to_py)?;
        Ok(PyGraph { inner })
    }

    /// Export to Graphviz DOT format for visualization.
    fn to_dot(&self) -> PyResult<String> {
        GraphExporter::export_dot(&self.inner).map_err(core_err_to_py)
    }

    fn __repr__(&self) -> String {
        format!(
            "Graph(nodes={}, edges={})",
            self.inner.node_count(),
            self.inner.edge_count()
        )
    }
}

// ---------------------------------------------------------------------
// Analysis result types
// ---------------------------------------------------------------------

#[pyclass(from_py_object, name = "BlastRadiusResult")]
#[derive(Clone)]
pub struct PyBlastRadiusResult {
    #[pyo3(get)]
    pub source_node: String,
    #[pyo3(get)]
    pub directly_impacted: Vec<String>,
    #[pyo3(get)]
    pub indirectly_impacted: Vec<String>,
    #[pyo3(get)]
    pub tertiary_impacted: Vec<String>,
    #[pyo3(get)]
    pub blast_radius_score: f32,
    #[pyo3(get)]
    pub critical_path_count: usize,
    #[pyo3(get)]
    pub total_impacted: usize,
}

impl From<RBlastRadiusResult> for PyBlastRadiusResult {
    fn from(r: RBlastRadiusResult) -> Self {
        PyBlastRadiusResult {
            source_node: r.source_node.0.to_string(),
            directly_impacted: r
                .directly_impacted
                .iter()
                .map(|n| n.0.to_string())
                .collect(),
            indirectly_impacted: r
                .indirectly_impacted
                .iter()
                .map(|n| n.0.to_string())
                .collect(),
            tertiary_impacted: r
                .tertiary_impacted
                .iter()
                .map(|n| n.0.to_string())
                .collect(),
            blast_radius_score: r.blast_radius_score,
            critical_path_count: r.critical_path_count,
            total_impacted: r.total_impacted,
        }
    }
}

#[pymethods]
impl PyBlastRadiusResult {
    fn __repr__(&self) -> String {
        format!(
            "BlastRadiusResult(source_node={:?}, score={:.2}, total_impacted={})",
            self.source_node, self.blast_radius_score, self.total_impacted
        )
    }
}

#[pyclass(from_py_object, name = "CriticalityScore")]
#[derive(Clone)]
pub struct PyCriticalityScore {
    #[pyo3(get)]
    pub node_id: String,
    #[pyo3(get)]
    pub node_name: String,
    #[pyo3(get)]
    pub criticality_score: f32,
    #[pyo3(get)]
    pub downstream_count: usize,
    #[pyo3(get)]
    pub upstream_count: usize,
    #[pyo3(get)]
    pub is_single_point_of_failure: bool,
}

impl From<RCriticalityScore> for PyCriticalityScore {
    fn from(s: RCriticalityScore) -> Self {
        PyCriticalityScore {
            node_id: s.node_id.0.to_string(),
            node_name: s.node_name,
            criticality_score: s.criticality_score,
            downstream_count: s.downstream_count,
            upstream_count: s.upstream_count,
            is_single_point_of_failure: s.is_single_point_of_failure,
        }
    }
}

#[pymethods]
impl PyCriticalityScore {
    fn __repr__(&self) -> String {
        format!(
            "CriticalityScore(node_name={:?}, score={:.2}, spof={})",
            self.node_name, self.criticality_score, self.is_single_point_of_failure
        )
    }
}

#[pyclass(from_py_object, name = "SimulationResult")]
#[derive(Clone)]
pub struct PySimulationResult {
    #[pyo3(get)]
    pub source_node: String,
    #[pyo3(get)]
    pub failure_type: String,
    #[pyo3(get)]
    pub cascade_path: Vec<String>,
    #[pyo3(get)]
    pub estimated_impact_nodes: usize,
    #[pyo3(get)]
    pub estimated_recovery_time_minutes: u32,
    #[pyo3(get)]
    pub affected_services: Vec<String>,
    #[pyo3(get)]
    pub business_impact_score: f32,
}

impl From<RSimulationResult> for PySimulationResult {
    fn from(r: RSimulationResult) -> Self {
        PySimulationResult {
            source_node: r.source_node.0.to_string(),
            failure_type: r.failure_type,
            cascade_path: r.cascade_path.iter().map(|n| n.0.to_string()).collect(),
            estimated_impact_nodes: r.estimated_impact_nodes,
            estimated_recovery_time_minutes: r.estimated_recovery_time_minutes,
            affected_services: r.affected_services,
            business_impact_score: r.business_impact_score,
        }
    }
}

#[pymethods]
impl PySimulationResult {
    fn __repr__(&self) -> String {
        format!(
            "SimulationResult(source_node={:?}, impacted_nodes={}, business_impact={:.1})",
            self.source_node, self.estimated_impact_nodes, self.business_impact_score
        )
    }
}

// ---------------------------------------------------------------------
// Analyzers (thin wrappers around crate::analysis, mirroring their
// static-method shape on the Rust side)
// ---------------------------------------------------------------------

#[pyclass(name = "BlastRadiusAnalyzer")]
pub struct PyBlastRadiusAnalyzer;

#[pymethods]
impl PyBlastRadiusAnalyzer {
    #[staticmethod]
    #[pyo3(signature = (graph, node_id, max_depth=None))]
    fn analyze(
        graph: &PyGraph,
        node_id: &str,
        max_depth: Option<usize>,
    ) -> PyResult<PyBlastRadiusResult> {
        let id = parse_node_id(node_id)?;
        let result = match max_depth {
            Some(depth) => RBlastRadiusAnalyzer::analyze_with_depth(&graph.inner, id, depth),
            None => RBlastRadiusAnalyzer::analyze(&graph.inner, id),
        }
        .map_err(core_err_to_py)?;
        Ok(result.into())
    }
}

#[pyclass(name = "CriticalityScorer")]
pub struct PyCriticalityScorer;

#[pymethods]
impl PyCriticalityScorer {
    #[staticmethod]
    fn score_all(graph: &PyGraph) -> PyResult<Vec<PyCriticalityScore>> {
        let scores = RCriticalityScorer::score_all(&graph.inner).map_err(core_err_to_py)?;
        Ok(scores.into_iter().map(Into::into).collect())
    }

    #[staticmethod]
    fn score_node(graph: &PyGraph, node_id: &str) -> PyResult<PyCriticalityScore> {
        let id = parse_node_id(node_id)?;
        let score = RCriticalityScorer::score_node(&graph.inner, id).map_err(core_err_to_py)?;
        Ok(score.into())
    }

    #[staticmethod]
    fn get_critical_nodes(graph: &PyGraph, threshold: f32) -> PyResult<Vec<PyCriticalityScore>> {
        let scores = RCriticalityScorer::get_critical_nodes(&graph.inner, threshold)
            .map_err(core_err_to_py)?;
        Ok(scores.into_iter().map(Into::into).collect())
    }

    #[staticmethod]
    fn get_single_points_of_failure(graph: &PyGraph) -> PyResult<Vec<PyCriticalityScore>> {
        let scores = RCriticalityScorer::get_single_points_of_failure(&graph.inner)
            .map_err(core_err_to_py)?;
        Ok(scores.into_iter().map(Into::into).collect())
    }
}

#[pyclass(name = "Simulator")]
pub struct PySimulator;

#[pymethods]
impl PySimulator {
    #[staticmethod]
    #[pyo3(signature = (graph, node_id, failure_type, max_depth=None))]
    fn simulate(
        graph: &PyGraph,
        node_id: &str,
        failure_type: PyFailureType,
        max_depth: Option<usize>,
    ) -> PyResult<PySimulationResult> {
        let id = parse_node_id(node_id)?;
        let result = match max_depth {
            Some(depth) => {
                RSimulator::simulate_with_time_limit(&graph.inner, id, failure_type.into(), depth)
            }
            None => RSimulator::simulate(&graph.inner, id, failure_type.into()),
        }
        .map_err(core_err_to_py)?;
        Ok(result.into())
    }
}

// ---------------------------------------------------------------------
// Integrations (real Slack / PagerDuty webhook dispatch)
// ---------------------------------------------------------------------

/// Slack incoming-webhook client.
///
/// Every method performs a real HTTP POST (10s timeout) to the configured
/// webhook URL and raises `RuntimeError` on transport failures or non-2xx
/// responses. No payload is fabricated or silently dropped.
#[pyclass(name = "SlackIntegration")]
pub struct PySlackIntegration {
    inner: RSlackIntegration,
}

#[pymethods]
impl PySlackIntegration {
    #[new]
    #[pyo3(signature = (webhook_url, channel=None, bot_name=None))]
    fn new(webhook_url: String, channel: Option<String>, bot_name: Option<String>) -> Self {
        let mut inner = RSlackIntegration::new(webhook_url);
        if let Some(channel) = channel {
            inner = inner.with_channel(channel);
        }
        if let Some(bot_name) = bot_name {
            inner.bot_name = Some(bot_name);
        }
        PySlackIntegration { inner }
    }

    #[getter]
    fn webhook_url(&self) -> &str {
        &self.inner.webhook_url
    }

    #[getter]
    fn channel(&self) -> Option<String> {
        self.inner.channel.clone()
    }

    #[getter]
    fn bot_name(&self) -> Option<String> {
        self.inner.bot_name.clone()
    }

    /// Send a blast-radius alert to Slack.
    fn send_blast_radius_alert(
        &self,
        py: Python<'_>,
        service_name: &str,
        blast_score: f32,
        affected_services: Vec<String>,
    ) -> PyResult<()> {
        let inner = self.inner.clone();
        let service_name = service_name.to_string();
        let result = py
            .detach(move || {
                block_on_raw(inner.send_blast_radius_alert(
                    &service_name,
                    blast_score,
                    affected_services,
                ))
            })
            .map_err(PyRuntimeError::new_err)?;
        result.map_err(core_err_to_py)
    }

    /// Send an anomaly alert to Slack.
    fn send_anomaly_alert(
        &self,
        py: Python<'_>,
        node_name: &str,
        anomaly_type: &str,
        severity: f32,
        predicted_ttf: u32,
    ) -> PyResult<()> {
        let inner = self.inner.clone();
        let node_name = node_name.to_string();
        let anomaly_type = anomaly_type.to_string();
        let result = py
            .detach(move || {
                block_on_raw(inner.send_anomaly_alert(
                    &node_name,
                    &anomaly_type,
                    severity,
                    predicted_ttf,
                ))
            })
            .map_err(PyRuntimeError::new_err)?;
        result.map_err(core_err_to_py)
    }

    fn __repr__(&self) -> String {
        format!(
            "SlackIntegration(webhook_url={:?}, channel={:?})",
            self.inner.webhook_url, self.inner.channel
        )
    }
}

/// PagerDuty Events API v2 client.
///
/// `create_incident`/`resolve_incident` perform real HTTP POSTs and raise
/// `RuntimeError` on transport failures or non-2xx responses. `create_incident`
/// returns the `dedup_key` from the PagerDuty response.
#[pyclass(name = "PagerDutyIntegration")]
pub struct PyPagerDutyIntegration {
    inner: RPagerDutyIntegration,
}

#[pymethods]
impl PyPagerDutyIntegration {
    #[new]
    #[pyo3(signature = (integration_key, api_url=None))]
    fn new(integration_key: String, api_url: Option<String>) -> Self {
        let mut inner = RPagerDutyIntegration::new(integration_key);
        if let Some(api_url) = api_url {
            inner.api_url = Some(api_url);
        }
        PyPagerDutyIntegration { inner }
    }

    #[getter]
    fn integration_key(&self) -> &str {
        &self.inner.integration_key
    }

    #[getter]
    fn api_url(&self) -> Option<String> {
        self.inner.api_url.clone()
    }

    /// Trigger a PagerDuty incident and return its `dedup_key`.
    fn create_incident(
        &self,
        py: Python<'_>,
        service_name: &str,
        blast_score: f32,
        description: &str,
    ) -> PyResult<String> {
        let inner = self.inner.clone();
        let service_name = service_name.to_string();
        let description = description.to_string();
        let result = py
            .detach(move || {
                block_on_raw(inner.create_incident(&service_name, blast_score, &description))
            })
            .map_err(PyRuntimeError::new_err)?;
        result.map_err(core_err_to_py)
    }

    /// Resolve a previously triggered PagerDuty incident.
    fn resolve_incident(&self, py: Python<'_>, incident_id: &str) -> PyResult<()> {
        let inner = self.inner.clone();
        let incident_id = incident_id.to_string();
        let result = py
            .detach(move || block_on_raw(inner.resolve_incident(&incident_id)))
            .map_err(PyRuntimeError::new_err)?;
        result.map_err(core_err_to_py)
    }

    fn __repr__(&self) -> String {
        format!(
            "PagerDutyIntegration(integration_key={:?}, api_url={:?})",
            self.inner.integration_key, self.inner.api_url
        )
    }
}

// ---------------------------------------------------------------------
// Discovery
// ---------------------------------------------------------------------

/// Real Terraform (.tf) dependency discovery, via `hcl-rs` parsing and
/// reference-based dependency detection (see `discovery::terraform`).
///
/// Previously the only `TerraformDiscoverer` reachable from Python was a
/// pure-Python placeholder in `pyblastradius.discovery` that walked
/// matching files and discarded their contents (`# For now, just create
/// mock nodes` / `pass`), always returning an empty graph -- the real Rust
/// implementation existed but was never exposed via pyo3 at all, so
/// nothing could reach it. `pyblastradius/__init__.py` now imports this
/// one instead.
#[pyclass(name = "TerraformDiscoverer")]
pub struct PyTerraformDiscoverer {
    inner: crate::discovery::terraform::TerraformDiscoverer,
}

#[pymethods]
impl PyTerraformDiscoverer {
    #[new]
    #[pyo3(signature = (path=".".to_string(), backend=None))]
    fn new(path: String, backend: Option<String>) -> Self {
        // `backend` (e.g. "local"/"s3" Terraform state backend) is accepted
        // for interface compatibility with the old Python stub's
        // constructor, but discovery here works directly from the real
        // *.tf source files on disk, not remote state, so it has no effect
        // on what gets discovered.
        let _ = backend;
        PyTerraformDiscoverer {
            inner: crate::discovery::terraform::TerraformDiscoverer::new(path),
        }
    }

    fn discover(&self) -> PyResult<PyGraph> {
        let inner = self.inner.discover().map_err(core_err_to_py)?;
        Ok(PyGraph { inner })
    }

    fn __repr__(&self) -> String {
        format!("TerraformDiscoverer(root_dir={:?})", self.inner.root_dir)
    }
}

/// Real Kubernetes manifest dependency discovery, via local YAML parsing
/// and container-env-var reference inference (see `discovery::kubernetes`).
///
/// Previously the only `KubernetesDiscoverer` reachable from Python was a
/// pure-Python placeholder in `pyblastradius.discovery` that globbed for
/// `*.yaml` files and then did `pass` in the loop body, always returning
/// an empty graph -- the real Rust implementation existed but was never
/// exposed via pyo3. `pyblastradius/__init__.py` now imports this one
/// instead.
///
/// `kubeconfig`/`context` are accepted for interface compatibility with
/// the old Python stub's constructor (and with existing `blastradius.yml`
/// configs / REST API payloads that pass them) but have no effect: this
/// discoverer reads local manifest YAML files, not a live cluster, so
/// there is no kubeconfig-based connection to make. If `manifests_dir`
/// isn't given, the conventional `k8s`/`kubernetes`/`.` directories are
/// probed in that order for the first one that exists -- the same
/// directories the old stub listed, now actually parsed instead of
/// discarded.
///
/// Note: `namespaces` is accepted and stored on the underlying Rust
/// `KubernetesDiscoverer`, but that struct doesn't yet use it to filter
/// discovered resources (see `discovery::kubernetes.rs`) -- passing it
/// here has no effect yet either. Documented honestly rather than
/// silently implying namespace filtering works.
#[pyclass(name = "KubernetesDiscoverer")]
pub struct PyKubernetesDiscoverer {
    inner: crate::discovery::kubernetes::KubernetesDiscoverer,
}

#[pymethods]
impl PyKubernetesDiscoverer {
    #[new]
    #[pyo3(signature = (manifests_dir=None, kubeconfig=None, context=None, namespaces=None))]
    fn new(
        manifests_dir: Option<String>,
        kubeconfig: Option<String>,
        context: Option<String>,
        namespaces: Option<Vec<String>>,
    ) -> Self {
        let _ = (kubeconfig, context);

        let dir = manifests_dir.or_else(|| {
            ["k8s", "kubernetes", "."]
                .into_iter()
                .find(|d| std::path::Path::new(d).is_dir())
                .map(|d| d.to_string())
        });

        let mut inner = match dir {
            Some(dir) => {
                crate::discovery::kubernetes::KubernetesDiscoverer::from_manifests_dir(dir)
            }
            None => crate::discovery::kubernetes::KubernetesDiscoverer::new(),
        };
        if let Some(namespaces) = namespaces {
            inner = inner.with_namespaces(namespaces);
        }

        PyKubernetesDiscoverer { inner }
    }

    fn discover(&self) -> PyResult<PyGraph> {
        let inner = self.inner.discover().map_err(core_err_to_py)?;
        Ok(PyGraph { inner })
    }

    fn __repr__(&self) -> String {
        format!(
            "KubernetesDiscoverer(manifests_dir={:?})",
            self.inner.manifests_dir
        )
    }
}

/// Real OTLP/gRPC trace-based dependency discovery (see `discovery::otlp`).
///
/// Previously the only `OtelDiscoverer` reachable from Python was a
/// pure-Python placeholder in `pyblastradius.discovery` whose `discover()`
/// body was just a comment ("Connect to OTLP collector and extract
/// traces") followed by returning an empty graph -- the real Rust
/// implementation existed but was never exposed via pyo3.
///
/// Important semantic correction vs. the old stub's docstring: this does
/// **not** connect outward to a remote OTLP collector. It runs its own
/// OTLP/gRPC trace *receiver* bound to `endpoint`, the same role a
/// collector plays -- your instrumented services need to be configured to
/// export spans *to* this address. `endpoint` must therefore be a literal
/// `ip:port` listen address (e.g. `"0.0.0.0:4317"`), not a hostname; the
/// old default `"localhost:4317"` would fail to parse for this reason, so
/// the default here is `"0.0.0.0:4317"` instead.
///
/// `discover()` blocks for up to `duration_minutes` (converted to seconds
/// for the real Rust `timeout_seconds` field) collecting whatever spans
/// are actually received, then builds the dependency graph from real
/// observed parent/child span pairs that cross a service boundary. It
/// releases the GIL while blocked so it doesn't freeze other Python
/// threads during the collection window.
#[cfg(feature = "otlp-traces")]
#[pyclass(name = "OtelDiscoverer")]
pub struct PyOtelDiscoverer {
    inner: crate::discovery::otlp::OtlpDiscoverer,
}

#[cfg(feature = "otlp-traces")]
#[pymethods]
impl PyOtelDiscoverer {
    #[new]
    #[pyo3(signature = (endpoint="0.0.0.0:4317".to_string(), duration_minutes=None, timeout_seconds=None))]
    fn new(endpoint: String, duration_minutes: Option<u64>, timeout_seconds: Option<u64>) -> Self {
        // `timeout_seconds` is the more precise real-unit override;
        // `duration_minutes` is kept for interface compatibility with the
        // old Python stub's constructor and existing config/REST payloads.
        let seconds = timeout_seconds.or_else(|| duration_minutes.map(|m| m * 60));

        let mut inner = crate::discovery::otlp::OtlpDiscoverer::new(endpoint);
        if let Some(seconds) = seconds {
            inner = inner.with_timeout(seconds);
        }

        PyOtelDiscoverer { inner }
    }

    fn discover(&self, py: Python<'_>) -> PyResult<PyGraph> {
        let inner = py
            .detach(|| self.inner.discover())
            .map_err(core_err_to_py)?;
        Ok(PyGraph { inner })
    }

    fn __repr__(&self) -> String {
        format!(
            "OtelDiscoverer(endpoint={:?}, timeout_seconds={})",
            self.inner.endpoint, self.inner.timeout_seconds
        )
    }
}

/// Real Airflow DAG dependency discovery (see `discovery::airflow`).
///
/// Previously the only `AirflowDiscoverer` reachable from Python was a
/// pure-Python placeholder in `pyblastradius.discovery` that stored a
/// `host`/`port`, never used them, and whose `discover()` body was a comment
/// followed by `return graph` -- an always-empty graph. The real Rust parser
/// existed but (like Kubernetes/Otel before it) was never exposed via pyo3.
///
/// Unlike the old stub's mental model, this does **not** talk to the Airflow
/// REST API at `host:port`. It parses DAG **Python source files** on disk,
/// extracting real tasks and their `>>`/`<<`/`set_downstream`/`chain(...)`
/// dependencies plus external systems referenced by operator class, `conn_id`
/// and SQL. `host`/`port` are therefore accepted (and ignored) only so that
/// existing `blastradius.yml` configs and REST payloads written against the
/// old constructor keep loading. Point it at your DAGs with `dag_folder`
/// (or `path`); when neither is given it defaults to the conventional `dags`
/// directory.
#[pyclass(name = "AirflowDiscoverer")]
pub struct PyAirflowDiscoverer {
    inner: crate::discovery::airflow::AirflowDiscoverer,
}

#[pymethods]
impl PyAirflowDiscoverer {
    #[new]
    #[pyo3(signature = (dag_folder=None, host=None, port=None, path=None))]
    fn new(
        dag_folder: Option<String>,
        host: Option<String>,
        port: Option<u64>,
        path: Option<String>,
    ) -> Self {
        // host/port are accepted for interface compatibility with the old
        // REST-API-shaped stub and its configs; this parser reads DAG files
        // from disk and has no HTTP connection to make.
        let _ = (host, port);

        let folder = dag_folder.or(path).unwrap_or_else(|| "dags".to_string());

        PyAirflowDiscoverer {
            inner: crate::discovery::airflow::AirflowDiscoverer::new(folder),
        }
    }

    fn discover(&self) -> PyResult<PyGraph> {
        let inner = self.inner.discover().map_err(core_err_to_py)?;
        Ok(PyGraph { inner })
    }

    fn __repr__(&self) -> String {
        format!("AirflowDiscoverer(dag_folder={:?})", self.inner.dag_folder)
    }
}

/// Enumerate every discovery scheme the installed build supports, as a list of
/// `(scheme, description)` pairs, sorted by scheme.
///
/// This is backed by the same `ProviderRegistry` the Rust CLIs use, so the
/// Python layer no longer needs a hand-maintained list that can drift from the
/// real set of connectors. A scheme with no Python-facing wrapper (if one is
/// ever added on the Rust side) still shows up here.
#[pyfunction]
fn available_providers() -> Vec<(String, String)> {
    crate::discovery::default_registry()
        .describe()
        .into_iter()
        .map(|info| (info.scheme, info.description))
        .collect()
}

// ---------------------------------------------------------------------
// Module
// ---------------------------------------------------------------------

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    m.add_class::<PyNodeType>()?;
    m.add_class::<PyEdgeType>()?;
    m.add_class::<PyHealthStatus>()?;
    m.add_class::<PyFailureType>()?;

    m.add_class::<PyNode>()?;
    m.add_class::<PyEdge>()?;
    m.add_class::<PyGraph>()?;

    m.add_class::<PyBlastRadiusResult>()?;
    m.add_class::<PyCriticalityScore>()?;
    m.add_class::<PySimulationResult>()?;

    m.add_class::<PyBlastRadiusAnalyzer>()?;
    m.add_class::<PyCriticalityScorer>()?;
    m.add_class::<PySimulator>()?;

    m.add_class::<PySlackIntegration>()?;
    m.add_class::<PyPagerDutyIntegration>()?;

    m.add_class::<PyTerraformDiscoverer>()?;
    m.add_class::<PyKubernetesDiscoverer>()?;
    m.add_class::<PyAirflowDiscoverer>()?;
    #[cfg(feature = "otlp-traces")]
    m.add_class::<PyOtelDiscoverer>()?;

    m.add_function(wrap_pyfunction!(available_providers, m)?)?;

    Ok(())
}
