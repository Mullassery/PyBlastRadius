use clap::{Parser, Subcommand};
use pyblastradius::graph::node::NodeId;
use pyblastradius::{
    about, AirflowDiscoverer, BlastRadiusAnalyzer, CriticalityScorer, Error, ExportFormat,
    FailureType, Graph, GraphExporter, GraphImporter, KubernetesDiscoverer, Simulator,
    TerraformDiscoverer,
};
use serde::Deserialize;
use std::path::PathBuf;

#[cfg(feature = "otlp-traces")]
use pyblastradius::OtlpDiscoverer;

#[derive(Parser)]
#[command(name = "pyblastradius")]
#[command(about = about(), long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(global = true, short, long)]
    verbose: bool,

    #[arg(global = true, short, long)]
    debug: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Discover dependencies from configs
    Scan {
        #[arg(short, long)]
        kubernetes: Option<PathBuf>,

        #[arg(short, long)]
        terraform: Option<PathBuf>,

        #[arg(short, long)]
        airflow: Option<PathBuf>,

        /// OTLP endpoint to listen on for a live trace collection window
        /// (e.g. "0.0.0.0:4317"). Requires the `otlp-traces` feature.
        #[arg(long)]
        otlp: Option<String>,

        /// How long to collect OTLP traces before building the graph
        #[arg(long, default_value_t = 30)]
        otlp_timeout_seconds: u64,

        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Analyze blast radius for a service
    Analyze {
        #[arg(short, long)]
        graph: PathBuf,

        #[arg(short, long)]
        service: String,

        /// No short flag: `-d` is already claimed by the global `--debug` flag.
        #[arg(long)]
        depth: Option<usize>,

        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Show criticality ranking
    Criticality {
        #[arg(short, long)]
        graph: PathBuf,

        #[arg(short, long)]
        threshold: Option<f32>,

        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Generate visualization
    Graph {
        #[arg(short, long)]
        input: PathBuf,

        #[arg(short, long)]
        output: Option<PathBuf>,

        #[arg(short, long)]
        format: Option<String>,
    },

    /// Simulate failure cascade
    Simulate {
        #[arg(short, long)]
        graph: PathBuf,

        #[arg(short, long)]
        service: String,

        #[arg(short, long)]
        failure_type: Option<String>,

        #[arg(short, long)]
        max_depth: Option<usize>,
    },

    /// Analyze deployment risk
    AnalyzeDeployment {
        #[arg(short, long)]
        manifest: PathBuf,

        #[arg(short, long)]
        graph: PathBuf,
    },

    /// Show system status and metrics
    Status {
        #[arg(short, long)]
        graph: PathBuf,
    },

    /// Show version
    Version,
}

/// Finds a node by exact name, falling back to a suffix match on
/// `<name>.<namespace>`-style keys (how Kubernetes-discovered nodes are
/// named) so `--service auth-service` matches a node named
/// `auth-service.default`.
fn find_node_by_name(graph: &Graph, name: &str) -> Option<NodeId> {
    graph
        .nodes()
        .find(|n| n.name == name)
        .or_else(|| {
            graph
                .nodes()
                .find(|n| n.name.starts_with(&format!("{}.", name)))
        })
        .map(|n| n.id)
}

fn parse_failure_type(s: Option<&str>) -> FailureType {
    match s {
        Some("degradation") => FailureType::Degradation,
        Some("high_latency") => FailureType::HighLatency,
        Some("high_error_rate") => FailureType::HighErrorRate,
        Some("data_quality") => FailureType::DataQualityIssue,
        _ => FailureType::Outage,
    }
}

fn export_format(s: Option<&str>) -> ExportFormat {
    match s.map(str::to_lowercase).as_deref() {
        Some("yaml") => ExportFormat::Yaml,
        Some("graphml") => ExportFormat::GraphML,
        Some("dot") | None => ExportFormat::Dot,
        Some(other) => {
            eprintln!("⚠️  Unknown format {:?}, defaulting to dot", other);
            ExportFormat::Dot
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if cli.verbose {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
    }

    match cli.command {
        Commands::Scan {
            kubernetes,
            terraform,
            airflow,
            otlp,
            otlp_timeout_seconds,
            output,
        } => {
            println!("🔍 Scanning for dependencies...\n");

            // These are only consumed by the `otlp-traces`-gated block below.
            #[cfg(not(feature = "otlp-traces"))]
            let _ = (&otlp, otlp_timeout_seconds);

            let mut graph = Graph::new();
            let mut sources_scanned = 0;

            if let Some(path) = &kubernetes {
                println!("  📦 Scanning Kubernetes manifests at {:?}...", path);
                let discoverer = KubernetesDiscoverer::from_manifests_dir(path.to_string_lossy());
                let discovered = discoverer.discover()?;
                println!(
                    "    ✅ Found {} nodes, {} edges",
                    discovered.node_count(),
                    discovered.edge_count()
                );
                graph.merge(&discovered)?;
                sources_scanned += 1;
            }

            if let Some(path) = &terraform {
                println!("  🏗️  Scanning Terraform configs at {:?}...", path);
                let discoverer = TerraformDiscoverer::new(path.to_string_lossy().to_string());
                let discovered = discoverer.discover()?;
                println!(
                    "    ✅ Found {} nodes, {} edges",
                    discovered.node_count(),
                    discovered.edge_count()
                );
                graph.merge(&discovered)?;
                sources_scanned += 1;
            }

            if let Some(path) = &airflow {
                println!("  📊 Scanning Airflow DAGs at {:?}...", path);
                let discoverer = AirflowDiscoverer::new(path.to_string_lossy().to_string());
                let discovered = discoverer.discover()?;
                println!(
                    "    ✅ Found {} nodes, {} edges",
                    discovered.node_count(),
                    discovered.edge_count()
                );
                graph.merge(&discovered)?;
                sources_scanned += 1;
            }

            #[cfg(feature = "otlp-traces")]
            if let Some(endpoint) = &otlp {
                println!(
                    "  📡 Listening for OTLP traces on {} for {}s...",
                    endpoint, otlp_timeout_seconds
                );
                let discoverer =
                    OtlpDiscoverer::new(endpoint.clone()).with_timeout(otlp_timeout_seconds);
                let discovered = discoverer.discover()?;
                println!(
                    "    ✅ Observed {} nodes, {} edges from live traces",
                    discovered.node_count(),
                    discovered.edge_count()
                );
                graph.merge(&discovered)?;
                sources_scanned += 1;
            }
            #[cfg(not(feature = "otlp-traces"))]
            if otlp.is_some() {
                eprintln!(
                    "⚠️  --otlp requires the `otlp-traces` feature (not enabled in this build)"
                );
            }

            if sources_scanned == 0 {
                println!("  ⚠️  No discovery source specified. Pass --kubernetes, --terraform, --airflow, and/or --otlp.");
            }

            let metrics = graph.metrics();
            println!(
                "\n📈 Discovery Results:\n   Nodes: {}\n   Edges: {}\n   Avg Degree: {:.2}",
                metrics.node_count, metrics.edge_count, metrics.avg_degree
            );

            if let Some(output_path) = output {
                GraphExporter::save_file(&graph, &output_path)?;
                println!("\n💾 Saved graph to {:?}", output_path);
            }

            Ok(())
        }

        Commands::Analyze {
            graph,
            service,
            depth: _depth,
            output,
        } => {
            println!("📊 Analyzing blast radius...\n");

            let graph = GraphImporter::load_file(&graph)?;
            let node_id = find_node_by_name(&graph, &service).ok_or_else(|| {
                Error::NotFound(format!("service {:?} not found in graph", service))
            })?;

            let result = BlastRadiusAnalyzer::analyze(&graph, node_id)?;

            println!("🎯 Blast Radius Analysis for {}\n", service);
            println!("   Directly Impacted: {}", result.directly_impacted.len());
            println!(
                "   Indirectly Impacted: {}",
                result.indirectly_impacted.len()
            );
            println!(
                "   Blast Radius Score: {:.1}/100",
                result.blast_radius_score
            );
            println!("   Critical Paths: {}\n", result.critical_path_count);

            if let Some(output_path) = output {
                let json = serde_json::to_string_pretty(&result)?;
                std::fs::write(&output_path, json)?;
                println!("💾 Saved analysis to {:?}", output_path);
            }

            Ok(())
        }

        Commands::Criticality {
            graph,
            threshold,
            output,
        } => {
            println!("🔴 Computing criticality scores...\n");

            let graph = GraphImporter::load_file(&graph)?;
            let scores = CriticalityScorer::score_all(&graph)?;

            println!("Criticality Ranking:\n");
            println!(
                "  {:<25} {:<12} {:<12}",
                "Node", "Criticality", "Downstream"
            );
            println!("  {}", "-".repeat(49));

            let filtered: Vec<_> = scores
                .into_iter()
                .filter(|score| {
                    threshold.is_none()
                        || score.criticality_score >= threshold.unwrap_or(0.0) * 100.0
                })
                .collect();

            for score in &filtered {
                println!(
                    "  {:<25} {:<12.1} {:<12}",
                    score.node_name, score.criticality_score, score.downstream_count
                );
            }

            if let Some(output_path) = output {
                let json = serde_json::to_string_pretty(&filtered)?;
                std::fs::write(&output_path, json)?;
                println!("\n💾 Saved criticality ranking to {:?}", output_path);
            }

            Ok(())
        }

        Commands::Graph {
            input,
            output,
            format,
        } => {
            println!("📈 Generating graph visualization...\n");

            let graph = GraphImporter::load_file(&input)?;
            let fmt = export_format(format.as_deref());
            let rendered = GraphExporter::export(&graph, fmt)?;

            println!("  Format: {:?}", fmt);
            println!(
                "  Source: {} nodes, {} edges",
                graph.node_count(),
                graph.edge_count()
            );

            if let Some(output_path) = output {
                std::fs::write(&output_path, &rendered)?;
                println!(
                    "  Output: {:?}\n  ✅ Graph visualization generated",
                    output_path
                );
            } else {
                println!();
                println!("{}", rendered);
            }

            Ok(())
        }

        Commands::Simulate {
            graph,
            service,
            failure_type,
            max_depth,
        } => {
            println!("⚡ Simulating failure cascade for {}\n", service);

            let graph = GraphImporter::load_file(&graph)?;
            let node_id = find_node_by_name(&graph, &service).ok_or_else(|| {
                Error::NotFound(format!("service {:?} not found in graph", service))
            })?;

            let failure = parse_failure_type(failure_type.as_deref());
            let result = if let Some(depth) = max_depth {
                Simulator::simulate_with_time_limit(&graph, node_id, failure, depth)?
            } else {
                Simulator::simulate(&graph, node_id, failure)?
            };

            println!("  Cascade Path: {:?}", result.affected_services);
            println!(
                "  Est. Recovery Time: {} minutes",
                result.estimated_recovery_time_minutes
            );
            println!(
                "  Business Impact Score: {:.1}/100\n",
                result.business_impact_score
            );

            Ok(())
        }

        Commands::AnalyzeDeployment { manifest, graph } => {
            println!("🚀 Analyzing deployment risk...\n");

            let graph_data = GraphImporter::load_file(&graph)?;

            // Identify what's being deployed from the manifest itself,
            // rather than assuming a fixed "healthy" outcome.
            let manifest_content = std::fs::read_to_string(&manifest)?;
            // Manifests are commonly multi-document (`---`-separated); take
            // the first document that actually names a service — plain
            // `serde_norway::from_str` errors out on multi-document input.
            let service_name = serde_norway::Deserializer::from_str(&manifest_content)
                .filter_map(|doc| serde_norway::Value::deserialize(doc).ok())
                .find_map(|v| {
                    v.get("metadata")
                        .and_then(|m| m.get("name"))
                        .and_then(|n| n.as_str())
                        .map(String::from)
                });

            match service_name
                .as_deref()
                .and_then(|name| find_node_by_name(&graph_data, name))
            {
                Some(node_id) => {
                    let node = graph_data.get_node(&node_id).unwrap();
                    let blast_result = BlastRadiusAnalyzer::analyze(&graph_data, node_id)?;

                    let risk = if node.properties.criticality > 0.8
                        || blast_result.blast_radius_score > 50.0
                    {
                        "HIGH"
                    } else if node.properties.criticality > 0.5 {
                        "MEDIUM"
                    } else {
                        "LOW"
                    };

                    println!("  Service: {}", node.name);
                    println!("  Criticality: {:.2}", node.properties.criticality);
                    println!(
                        "  Blast Radius Score: {:.1}/100",
                        blast_result.blast_radius_score
                    );
                    println!(
                        "  Downstream Impact: {} services",
                        blast_result.directly_impacted.len()
                            + blast_result.indirectly_impacted.len()
                    );
                    println!("  Risk Level: {}", risk);
                    if risk == "HIGH" {
                        println!("\n  ⚠️  Recommend deploying during your lowest-traffic window and having rollback ready.");
                    } else {
                        println!("\n  ✅ No elevated risk signals from the dependency graph.");
                    }
                }
                None => {
                    println!(
                        "  ⚠️  Could not identify this manifest's service in the dependency graph."
                    );
                    println!("     Risk Level: UNKNOWN (new or unrecognized service — no historical blast-radius data)");
                }
            }
            println!();

            Ok(())
        }

        Commands::Status { graph } => {
            println!("📊 PyBlastRadius System Status\n");

            let graph = GraphImporter::load_file(&graph)?;
            let metrics = graph.metrics();

            println!("  Nodes: {}", metrics.node_count);
            println!("  Edges: {}", metrics.edge_count);
            println!("  Avg Degree: {:.2}", metrics.avg_degree);
            println!("  Density: {:.3}", metrics.density);
            println!("  Connected Components: {}\n", metrics.connected_components);

            Ok(())
        }

        Commands::Version => {
            println!("{}", pyblastradius::about());
            Ok(())
        }
    }
}
