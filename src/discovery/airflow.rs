use crate::error::Result;
use crate::graph::{Edge, EdgeType, Graph, Node, NodeType};
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use walkdir::WalkDir;

/// Airflow DAG discovery.
///
/// Airflow DAGs are Python source, so there is no declarative format to parse
/// with a schema parser the way Terraform HCL or Kubernetes YAML can be.
/// Instead this reads the DAG files on disk and extracts the structures that
/// Airflow itself derives at import time: the DAG(s), each task, the task
/// dependency graph expressed with `>>` / `<<` / `set_downstream` /
/// `set_upstream` / `chain(...)`, and the external systems tasks reference via
/// their operator class and `conn_id`/`sql` arguments.
///
/// This replaces the previous implementation, whose `add_task_dependencies`
/// emitted a **hardcoded** `example_dag.load_data -> example_dag.transform ->
/// example_dag.publish` chain regardless of what any DAG actually said, and
/// whose declaration scan matched `task_id=` anywhere in the file. Every edge
/// produced now comes from a real dependency expression found in a real DAG
/// file, or is not produced at all.
///
/// Scope and honesty: this is a source-level heuristic parser, not a Python
/// interpreter. It understands the canonical Airflow idioms (assignment to an
/// `*Operator`/`*Sensor`, the TaskFlow `@task` decorator, the dependency DSL,
/// and the common `conn_id`/SQL table references). Dynamic, generated task ids
/// (e.g. f-strings, loops) or TaskGroup nesting are not resolved, and a file is
/// only treated as a DAG if it actually declares `DAG(...)` or `@dag`.
///
/// `host`/`port` are accepted by the Python layer for interface compatibility
/// with older configs, but they have no effect: this provider parses DAG files
/// on disk and does not connect to the Airflow REST API.
pub struct AirflowDiscoverer {
    pub dag_folder: String,
}

/// One task declaration found in a DAG file.
struct TaskDef {
    /// The Python variable the operator was assigned to (`extract` in
    /// `extract = PythonOperator(...)`), used to resolve dependency operands.
    var: String,
    /// The Airflow `task_id` (explicit `task_id=` argument, else the variable
    /// name, else the `@task` function name).
    task_id: String,
    /// Operator/Sensor class name (empty for TaskFlow `@task` functions).
    class_name: String,
    /// The raw argument list of the operator call, scanned for external refs.
    args: String,
}

impl AirflowDiscoverer {
    pub fn new(dag_folder: String) -> Self {
        AirflowDiscoverer { dag_folder }
    }

    /// Discover every DAG under `dag_folder`, merging them into one graph.
    ///
    /// A directory that does not exist (or contains no DAG files) yields an
    /// empty graph — never an error and never fabricated demo data.
    pub fn discover(&self) -> Result<Graph> {
        let mut graph = Graph::new();
        let mut node_map: HashMap<String, crate::graph::node::NodeId> = HashMap::new();

        for entry in WalkDir::new(&self.dag_folder)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file() && e.path().extension().is_some_and(|ext| ext == "py"))
        {
            let content = match std::fs::read_to_string(entry.path()) {
                Ok(content) => content,
                Err(_) => continue, // unreadable file: skip, don't abort the whole scan
            };
            if !looks_like_dag(&content) {
                continue; // a Python module that isn't a DAG (helpers, plugins, __init__)
            }
            self.parse_dag_file(entry.path(), &content, &mut graph, &mut node_map)?;
        }

        Ok(graph)
    }

    fn parse_dag_file(
        &self,
        path: &std::path::Path,
        content: &str,
        graph: &mut Graph,
        node_map: &mut HashMap<String, crate::graph::node::NodeId>,
    ) -> Result<()> {
        let dag_id = extract_dag_ids(content)
            .into_iter()
            .next()
            .unwrap_or_else(|| file_stem(path));

        let tasks = extract_tasks(content);
        if tasks.is_empty() {
            return Ok(()); // declared a DAG but no parseable tasks — nothing honest to add
        }

        // The DAG itself is represented as a pipeline node that all of its
        // tasks flow from, so a DAG-level failure has a real blast radius
        // within the file (rather than leaving every task isolated when a DAG
        // uses no explicit dependency operators).
        let dag_name = dag_id.clone();
        let dag_node = ensure_node(
            graph,
            node_map,
            &dag_name,
            Node::pipeline(&dag_name)
                .with_criticality(0.8)
                .add_discovered_via("airflow-dag"),
        )?;

        let mut canonical: HashMap<String, String> = HashMap::new();
        let mut task_nodes: HashMap<String, crate::graph::node::NodeId> = HashMap::new();

        for task in &tasks {
            let full_name = format!("{}.{}", dag_id, task.task_id);
            let node_id = ensure_node(
                graph,
                node_map,
                &full_name,
                Node::pipeline(&full_name)
                    .with_criticality(0.6)
                    .add_discovered_via("airflow-task"),
            )?;
            // Both the variable name and the task_id resolve to this task, so
            // `t1 >> t2` and `t2 << t1` work whether operands are variables or
            // explicit task ids.
            canonical.insert(task.var.clone(), full_name.clone());
            canonical.insert(task.task_id.clone(), full_name.clone());
            task_nodes.entry(full_name.clone()).or_insert(node_id);

            add_edge(
                graph,
                dag_node,
                node_id,
                EdgeType::Feeds,
                "airflow-dag",
                1.0,
            )?;
        }

        // Task-to-task dependencies: upstream >> downstream becomes an
        // upstream -> downstream edge (Airflow renders `>>` the same way), so
        // analyzing an upstream task reaches everything that runs after it.
        for (from, to) in extract_dependencies(content) {
            let (Some(from_name), Some(to_name)) = (canonical.get(&from), canonical.get(&to))
            else {
                continue; // operand was a TaskGroup / unknown symbol: don't guess
            };
            let (Some(&from_id), Some(&to_id)) =
                (task_nodes.get(from_name), task_nodes.get(to_name))
            else {
                continue;
            };
            if from_id != to_id {
                add_edge(
                    graph,
                    from_id,
                    to_id,
                    EdgeType::Feeds,
                    "airflow-dependency",
                    0.9,
                )?;
            }
        }

        // External systems each task references (databases, queues, storage,
        // HTTP services) via its operator class, conn_id, or SQL.
        let mut seen: HashSet<(String, String)> = HashSet::new();
        for task in &tasks {
            let full_name = format!("{}.{}", dag_id, task.task_id);
            let Some(&task_node) = task_nodes.get(&full_name) else {
                continue;
            };
            for (name, node_type, edge_type, via, confidence) in externals_for_task(task) {
                if !seen.insert((full_name.clone(), name.clone())) {
                    continue;
                }
                let criticality = criticality_for(&node_type);
                let ext_node = ensure_node(
                    graph,
                    node_map,
                    &name,
                    Node::new(&name, node_type)
                        .with_criticality(criticality)
                        .add_discovered_via(via),
                )?;
                add_edge(graph, task_node, ext_node, edge_type, via, confidence)?;
            }
        }

        Ok(())
    }
}

impl super::DiscoveryProvider for AirflowDiscoverer {
    fn scheme(&self) -> &'static str {
        "airflow"
    }

    fn description(&self) -> &'static str {
        "Parse local Airflow DAG (.py) files: DAGs, tasks, task-dependency operators (>>/<<, set_downstream/upstream), and external system references."
    }

    fn discover(&self) -> Result<Graph> {
        AirflowDiscoverer::discover(self)
    }
}

lazy_static! {
    static ref DAG_CALL: Regex = Regex::new(r"\bDAG\s*\(").unwrap();
    static ref DAG_DECORATOR: Regex = Regex::new(r"@dag\b").unwrap();
    static ref DAG_ID_ARG: Regex = Regex::new(r#"dag_id\s*=\s*["']([^"']+)["']"#).unwrap();
    static ref FIRST_STRING: Regex = Regex::new(r#"["']([^"']+)["']"#).unwrap();
    static ref DEF_NAME: Regex = Regex::new(r"(?m)def[ \t]+(\w+)").unwrap();
    static ref TASK_ASSIGN: Regex =
        Regex::new(r"(?m)^[ \t]*([A-Za-z_]\w*)[ \t]*=[ \t]*([A-Za-z_]\w*)[ \t]*\(").unwrap();
    static ref TASK_ID_ARG: Regex = Regex::new(r#"task_id\s*=\s*["']([^"']+)["']"#).unwrap();
    static ref TASKFLOW: Regex = Regex::new(
        r"(?m)@task(?:\.\w+)?(?:\([^)]*\))?[ \t]*\r?\n[ \t]*(?:async[ \t]+)?def[ \t]+(\w+)"
    )
    .unwrap();
    static ref SET_DOWNSTREAM: Regex =
        Regex::new(r"(\w+)[ \t]*\.[ \t]*set_downstream[ \t]*\(([^)]*)\)").unwrap();
    static ref SET_UPSTREAM: Regex =
        Regex::new(r"(\w+)[ \t]*\.[ \t]*set_upstream[ \t]*\(([^)]*)\)").unwrap();
    static ref CHAIN_CALL: Regex = Regex::new(r"chain[ \t]*\(([^)]*)\)").unwrap();
    static ref CONN_ID_ARG: Regex = Regex::new(r#"conn_id\s*=\s*["']([^"']+)["']"#).unwrap();
    static ref SQL_ARG: Regex = Regex::new(r#"(?s)sql\s*=\s*["']([^"']*)["']"#).unwrap();
    static ref SQL_TABLE: Regex =
        Regex::new(r"(?i)\b(?:from|join|into|update)\s+([A-Za-z_][A-Za-z0-9_.$]*)").unwrap();
}

fn looks_like_dag(content: &str) -> bool {
    DAG_CALL.is_match(content) || DAG_DECORATOR.is_match(content)
}

fn file_stem(path: &std::path::Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown_dag".to_string())
}

/// Read the argument list of a call whose opening `(` is at byte index
/// `open_paren`, tracking nested brackets and skipping string literals so a
/// `)` inside a nested call or string doesn't terminate early.
fn read_call_args(content: &str, open_paren: usize) -> String {
    let bytes = content.as_bytes();
    let mut depth = 0i32;
    let mut i = open_paren;
    let mut in_str: Option<u8> = None;
    let mut escaped = false;
    while i < bytes.len() {
        let b = bytes[i];
        if let Some(quote) = in_str {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == quote {
                in_str = None;
            }
            i += 1;
            continue;
        }
        match b {
            b'"' | b'\'' => in_str = Some(b),
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                depth -= 1;
                if depth == 0 {
                    return content[open_paren + 1..i].to_string();
                }
            }
            _ => {}
        }
        i += 1;
    }
    content[open_paren + 1..].to_string()
}

/// Every DAG id declared in the file, in first-seen order. Handles
/// `dag = DAG("id", ...)` and `with DAG(dag_id="id", ...) as dag:` (an explicit
/// string / `dag_id=` argument), and `@dag` (which, like Airflow itself, falls
/// back to the decorated function's name when no `dag_id` string is given).
fn extract_dag_ids(content: &str) -> Vec<String> {
    let mut ids = Vec::new();

    // `DAG("id")` / `DAG(dag_id="id")`: the regex match already includes `(`.
    for mat in DAG_CALL.find_iter(content) {
        let search_from = mat.end().saturating_sub(1);
        if let Some(rel) = content[search_from..].find('(') {
            if rel <= 1 {
                let args = read_call_args(content, search_from + rel);
                push_dag_id(&mut ids, dag_id_from_args(&args));
            }
        }
    }

    // `@dag(...)`: the call (if any) follows the decorator; when its arguments
    // carry no explicit id, Airflow uses the function name below it.
    for mat in DAG_DECORATOR.find_iter(content) {
        let search_from = mat.end();
        let id = match content[search_from..].find('(') {
            Some(rel) if rel <= 1 => {
                let args = read_call_args(content, search_from + rel);
                dag_id_from_args(&args).or_else(|| next_def_name(content, search_from))
            }
            _ => next_def_name(content, search_from),
        };
        push_dag_id(&mut ids, id);
    }

    ids
}

fn push_dag_id(ids: &mut Vec<String>, id: Option<String>) {
    if let Some(id) = id {
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
}

fn dag_id_from_args(args: &str) -> Option<String> {
    DAG_ID_ARG
        .captures(args)
        .map(|c| c[1].to_string())
        .or_else(|| FIRST_STRING.captures(args).map(|c| c[1].to_string()))
}

fn next_def_name(content: &str, from: usize) -> Option<String> {
    DEF_NAME
        .captures(&content[from..])
        .map(|c| c[1].to_string())
}

fn is_operator_class(class_name: &str) -> bool {
    class_name.ends_with("Operator") || class_name.ends_with("Sensor")
}

/// Parse every task declaration in the file.
fn extract_tasks(content: &str) -> Vec<TaskDef> {
    let mut tasks: Vec<TaskDef> = Vec::new();

    for cap in TASK_ASSIGN.captures_iter(content) {
        let class_name = cap[2].to_string();
        if !is_operator_class(&class_name) {
            continue;
        }
        let var = cap[1].to_string();
        let open_paren = cap.get(0).unwrap().end() - 1;
        let args = read_call_args(content, open_paren);
        let task_id = TASK_ID_ARG
            .captures(&args)
            .map(|c| c[1].to_string())
            .unwrap_or_else(|| var.clone());
        tasks.push(TaskDef {
            var,
            task_id,
            class_name,
            args,
        });
    }

    for cap in TASKFLOW.captures_iter(content) {
        let func = cap[1].to_string();
        tasks.push(TaskDef {
            var: func.clone(),
            task_id: func,
            class_name: String::new(),
            args: String::new(),
        });
    }

    tasks
}

fn join_continuations(content: &str) -> String {
    content.replace("\\\r\n", " ").replace("\\\n", " ")
}

fn strip_comment(line: &str) -> String {
    match line.find('#') {
        Some(idx) => line[..idx].to_string(),
        None => line.to_string(),
    }
}

/// Turn an operand (`a`, `[a, b]`, `[a,\n b]`) into the identifiers it names.
fn parse_operand_list(text: &str) -> Vec<String> {
    let cleaned = text.trim().trim_matches(|c| c == '[' || c == ']');
    cleaned
        .split(',')
        .filter_map(|part| {
            let token: String = part
                .trim()
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if token.is_empty() {
                None
            } else {
                Some(token)
            }
        })
        .collect()
}

fn split_top_level_commas(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut current = String::new();
    for ch in s.chars() {
        match ch {
            '(' | '[' | '{' => {
                depth += 1;
                current.push(ch);
            }
            ')' | ']' | '}' => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                parts.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }
    parts
}

/// All task dependency edges in the file as `(upstream, downstream)` operand
/// pairs, before operand -> task id resolution.
fn extract_dependencies(content: &str) -> Vec<(String, String)> {
    let joined = join_continuations(content);
    let mut edges = Vec::new();

    for raw in joined.lines() {
        let line = strip_comment(raw);
        if line.trim().is_empty() {
            continue;
        }
        extract_dependency_operators(&line, &mut edges);
        extract_set_calls(&line, &mut edges);
        extract_chain(&line, &mut edges);
    }

    edges
}

/// `a >> b >> c`, `a >> [b, c]`, `[a, b] >> c` and their `<<` mirrors.
fn extract_dependency_operators(line: &str, edges: &mut Vec<(String, String)>) {
    if line.contains(">>") {
        let groups: Vec<Vec<String>> = line.split(">>").map(parse_operand_list).collect();
        for window in groups.windows(2) {
            for from in &window[0] {
                for to in &window[1] {
                    edges.push((from.clone(), to.clone()));
                }
            }
        }
    }
    if line.contains("<<") {
        // `a << b` means b upstream of a, i.e. edge b -> a.
        let groups: Vec<Vec<String>> = line.split("<<").map(parse_operand_list).collect();
        for window in groups.windows(2) {
            for downstream in &window[0] {
                for upstream in &window[1] {
                    edges.push((upstream.clone(), downstream.clone()));
                }
            }
        }
    }
}

fn extract_set_calls(line: &str, edges: &mut Vec<(String, String)>) {
    for cap in SET_DOWNSTREAM.captures_iter(line) {
        let source = cap[1].to_string();
        for target in parse_operand_list(&cap[2]) {
            edges.push((source.clone(), target));
        }
    }
    for cap in SET_UPSTREAM.captures_iter(line) {
        let target = cap[1].to_string();
        for source in parse_operand_list(&cap[2]) {
            edges.push((source, target.clone()));
        }
    }
}

fn extract_chain(line: &str, edges: &mut Vec<(String, String)>) {
    for cap in CHAIN_CALL.captures_iter(line) {
        let groups: Vec<Vec<String>> = split_top_level_commas(&cap[1])
            .iter()
            .map(|arg| parse_operand_list(arg))
            .collect();
        for window in groups.windows(2) {
            for from in &window[0] {
                for to in &window[1] {
                    edges.push((from.clone(), to.clone()));
                }
            }
        }
    }
}

/// A task's external-system references: `(name, node_type, edge_type,
/// discovered_via, confidence)`.
fn externals_for_task(task: &TaskDef) -> Vec<(String, NodeType, EdgeType, &'static str, f32)> {
    let mut out = Vec::new();
    let conn = CONN_ID_ARG.captures(&task.args).map(|c| c[1].to_string());

    if let Some((node_type, default_name)) = classify_external(&task.class_name) {
        let name = conn.clone().unwrap_or_else(|| default_name.to_string());
        let (edge_type, via) = match &conn {
            Some(_) => (edge_type_for(&node_type), "airflow-conn"),
            None => (edge_type_for(&node_type), "airflow-operator"),
        };
        out.push((name, node_type, edge_type, via, 0.7));
    } else if let Some(conn) = conn {
        // An operator we don't specifically classify but that names a
        // connection still denotes a real external system.
        out.push((
            conn,
            NodeType::ExternalService,
            EdgeType::Calls,
            "airflow-conn",
            0.6,
        ));
    }

    if let Some(cap) = SQL_ARG.captures(&task.args) {
        for table in sql_tables(&cap[1]) {
            out.push((table, NodeType::Table, EdgeType::Feeds, "airflow-sql", 0.5));
        }
    }

    out
}

/// Map an operator class name to the kind of external system it implies.
fn classify_external(class_name: &str) -> Option<(NodeType, &'static str)> {
    let c = class_name.to_lowercase();
    if c.contains("postgres")
        || c.contains("redshift")
        || c.contains("snowflake")
        || c.contains("mysql")
        || c.contains("mongo")
        || c.contains("bigquery")
        || c.contains("sqlite")
        || c.contains("database")
    {
        Some((NodeType::Database, "database"))
    } else if c.contains("redis") || c.contains("memcache") {
        Some((NodeType::Cache, "cache"))
    } else if c.contains("kafka") || c.contains("pubsub") {
        Some((NodeType::Topic, "kafka"))
    } else if c.contains("sqs") || c.contains("sns") || c.contains("queue") {
        Some((NodeType::Queue, "queue"))
    } else if c.contains("s3") || c.contains("gcs") || c.contains("blob") || c.contains("storage") {
        Some((NodeType::Table, "object-storage"))
    } else if c.contains("http")
        || c.contains("slack")
        || c.contains("datadog")
        || c.contains("pagerduty")
        || c.contains("webhook")
    {
        Some((NodeType::ExternalService, "http-api"))
    } else {
        None
    }
}

fn edge_type_for(node_type: &NodeType) -> EdgeType {
    match node_type {
        NodeType::Database => EdgeType::Queries,
        NodeType::Cache => EdgeType::Queries,
        NodeType::Topic | NodeType::Queue => EdgeType::Produces,
        NodeType::Table => EdgeType::Feeds,
        _ => EdgeType::Calls,
    }
}

fn criticality_for(node_type: &NodeType) -> f32 {
    match node_type {
        NodeType::Database => 0.9,
        NodeType::Cache => 0.8,
        NodeType::Topic | NodeType::Queue => 0.7,
        NodeType::Table => 0.6,
        _ => 0.6,
    }
}

fn sql_tables(sql: &str) -> Vec<String> {
    let mut tables = Vec::new();
    for cap in SQL_TABLE.captures_iter(sql) {
        let table = cap[1].to_string();
        if !tables.contains(&table) {
            tables.push(table);
        }
    }
    tables
}

fn ensure_node(
    graph: &mut Graph,
    node_map: &mut HashMap<String, crate::graph::node::NodeId>,
    name: &str,
    node: Node,
) -> Result<crate::graph::node::NodeId> {
    if let Some(&id) = node_map.get(name) {
        return Ok(id);
    }
    let id = graph.add_node(node)?;
    node_map.insert(name.to_string(), id);
    Ok(id)
}

fn add_edge(
    graph: &mut Graph,
    source: crate::graph::node::NodeId,
    target: crate::graph::node::NodeId,
    edge_type: EdgeType,
    discovered_via: &str,
    confidence: f32,
) -> Result<()> {
    let edge = Edge::new(edge_type, source, target)
        .with_criticality(true)
        .with_confidence(confidence)
        .add_discovered_via(discovered_via);
    graph.add_edge(edge)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn discover(content: &str) -> Graph {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("dag.py"), content).unwrap();
        AirflowDiscoverer::new(dir.path().to_string_lossy().to_string())
            .discover()
            .unwrap()
    }

    fn node_named<'a>(graph: &'a Graph, name: &str) -> Option<&'a Node> {
        graph.nodes().find(|n| n.name == name)
    }

    #[test]
    fn test_missing_dag_folder_is_empty_not_an_error() {
        let discoverer = AirflowDiscoverer::new("/no/such/dags".to_string());
        let graph = discoverer.discover().unwrap();
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_non_dag_python_file_is_ignored() {
        let graph = discover("import os\n\ndef helper():\n    return 1\n");
        assert_eq!(graph.node_count(), 0);
    }

    #[test]
    fn test_real_shift_dependency_is_parsed() {
        let graph = discover(
            r#"
from airflow import DAG
from airflow.operators.python import PythonOperator

dag = DAG("etl", schedule=None)
extract = PythonOperator(task_id="extract", python_callable=lambda: None)
transform = PythonOperator(task_id="transform", python_callable=lambda: None)
load = PythonOperator(task_id="load", python_callable=lambda: None)

extract >> transform >> load
"#,
        );

        // DAG node + 3 task nodes.
        assert!(node_named(&graph, "etl").is_some());
        let extract = node_named(&graph, "etl.extract").unwrap();
        let transform = node_named(&graph, "etl.transform").unwrap();
        let load = node_named(&graph, "etl.load").unwrap();

        // Upstream failure must reach downstream tasks.
        assert!(graph.has_path(&extract.id, &transform.id).unwrap());
        assert!(graph.has_path(&extract.id, &load.id).unwrap());
        assert!(!graph.has_path(&load.id, &extract.id).unwrap());
    }

    #[test]
    fn test_no_hardcoded_example_dependencies_are_fabricated() {
        // Two tasks with no dependency expression between them: the previous
        // implementation would still have emitted example_dag edges. This one
        // must produce no task-to-task edge.
        let graph = discover(
            r#"
from airflow import DAG
dag = DAG("lonely", schedule=None)
a = BashOperator(task_id="a", bash_command="true")
b = BashOperator(task_id="b", bash_command="true")
"#,
        );

        let a = node_named(&graph, "lonely.a").unwrap();
        let b = node_named(&graph, "lonely.b").unwrap();
        let task_edges = graph
            .edges()
            .filter(|e| {
                e.properties
                    .discovered_via
                    .iter()
                    .any(|v| v == "airflow-dependency")
            })
            .count();
        assert_eq!(task_edges, 0);
        assert!(!graph.has_path(&a.id, &b.id).unwrap());
        assert!(!node_named(&graph, "etl").is_some());
    }

    #[test]
    fn test_list_shift_connects_one_to_many() {
        let graph = discover(
            r#"
from airflow import DAG
dag = DAG("fan", schedule=None)
start = BashOperator(task_id="start", bash_command="true")
left = BashOperator(task_id="left", bash_command="true")
right = BashOperator(task_id="right", bash_command="true")
start >> [left, right]
"#,
        );

        let start = node_named(&graph, "fan.start").unwrap();
        let left = node_named(&graph, "fan.left").unwrap();
        let right = node_named(&graph, "fan.right").unwrap();
        assert!(graph.has_path(&start.id, &left.id).unwrap());
        assert!(graph.has_path(&start.id, &right.id).unwrap());
    }

    #[test]
    fn test_reverse_shift_operator() {
        let graph = discover(
            r#"
from airflow import DAG
dag = DAG("rev", schedule=None)
a = BashOperator(task_id="a", bash_command="true")
b = BashOperator(task_id="b", bash_command="true")
b << a
"#,
        );
        let a = node_named(&graph, "rev.a").unwrap();
        let b = node_named(&graph, "rev.b").unwrap();
        // b << a  =>  a upstream of b.
        assert!(graph.has_path(&a.id, &b.id).unwrap());
    }

    #[test]
    fn test_set_downstream_and_upstream() {
        let graph = discover(
            r#"
from airflow import DAG
dag = DAG("s", schedule=None)
a = BashOperator(task_id="a", bash_command="true")
b = BashOperator(task_id="b", bash_command="true")
c = BashOperator(task_id="c", bash_command="true")
a.set_downstream(b)
c.set_upstream(b)
"#,
        );
        let a = node_named(&graph, "s.a").unwrap();
        let b = node_named(&graph, "s.b").unwrap();
        let c = node_named(&graph, "s.c").unwrap();
        assert!(graph.has_path(&a.id, &b.id).unwrap());
        assert!(graph.has_path(&b.id, &c.id).unwrap());
    }

    #[test]
    fn test_chain_helper() {
        let graph = discover(
            r#"
from airflow import DAG
dag = DAG("chained", schedule=None)
a = BashOperator(task_id="a", bash_command="true")
b = BashOperator(task_id="b", bash_command="true")
c = BashOperator(task_id="c", bash_command="true")
chain(a, b, c)
"#,
        );
        let a = node_named(&graph, "chained.a").unwrap();
        let c = node_named(&graph, "chained.c").unwrap();
        assert!(graph.has_path(&a.id, &c.id).unwrap());
    }

    #[test]
    fn test_operator_conn_id_becomes_external_database_node() {
        let graph = discover(
            r#"
from airflow import DAG
dag = DAG("ext", schedule=None)
load = PostgresOperator(task_id="load", postgres_conn_id="warehouse", sql="SELECT 1")
"#,
        );
        let warehouse = node_named(&graph, "warehouse");
        assert!(
            warehouse.is_some(),
            "expected a node for conn_id 'warehouse'"
        );
        assert_eq!(warehouse.unwrap().node_type, NodeType::Database);
    }

    #[test]
    fn test_sql_tables_are_discovered() {
        let graph = discover(
            r#"
from airflow import DAG
dag = DAG("sql", schedule=None)
report = PostgresOperator(
    task_id="report",
    postgres_conn_id="warehouse",
    sql="INSERT INTO analytics.daily SELECT * FROM raw.events",
)
"#,
        );
        assert!(node_named(&graph, "analytics.daily").is_some());
        assert!(node_named(&graph, "raw.events").is_some());
    }

    #[test]
    fn test_taskflow_decorator_is_discovered() {
        let graph = discover(
            r#"
from airflow.decorators import dag, task

@dag(schedule=None)
def my_pipeline():
    @task
    def first():
        return 1

    @task
    def second(value):
        return value + 1

    first() >> second()
"#,
        );
        assert!(node_named(&graph, "my_pipeline.first").is_some());
        assert!(node_named(&graph, "my_pipeline.second").is_some());
    }

    #[test]
    fn test_multiple_dags_in_one_file_use_first_dag_id() {
        let graph = discover(
            r#"
from airflow import DAG
dag1 = DAG("first_dag", schedule=None)
dag2 = DAG("second_dag", schedule=None)
a = BashOperator(task_id="a", bash_command="true")
"#,
        );
        assert!(node_named(&graph, "first_dag.a").is_some());
    }

    #[test]
    fn test_provider_trait_metadata() {
        let discoverer = AirflowDiscoverer::new(".".to_string());
        assert_eq!(
            super::super::DiscoveryProvider::scheme(&discoverer),
            "airflow"
        );
    }
}
