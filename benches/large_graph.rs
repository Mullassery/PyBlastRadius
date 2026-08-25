//! Benchmarks on large, interconnected dependency graphs.
//!
//! `criterion` and `[profile.bench]` were declared in Cargo.toml with a
//! `// Benchmarks (add later)` comment, but no `benches/` directory or
//! `[[bench]]` target ever existed -- every existing test uses small
//! fixture graphs (a handful of nodes), nothing at the scale this tool is
//! meant to operate at (real infrastructure/data-pipeline dependency
//! graphs with hundreds to thousands of interconnected resources).
//!
//! `build_layered_graph` builds a graph with real, non-trivial topology
//! (multiple fan-out layers, each layer's nodes depending on several nodes
//! in the layer below -- not a simple chain or star, which would make
//! blast-radius/criticality analysis artificially cheap) rather than a
//! degenerate synthetic shape.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use pyblastradius::analysis::{BlastRadiusAnalyzer, CriticalityScorer};
use pyblastradius::graph::{Edge, EdgeType, Graph, Node, NodeType};

/// Build a layered dependency graph: `layers` layers of `fan_out` nodes
/// each, where every node in layer N depends on `edges_per_node` nodes
/// chosen (deterministically, so runs are reproducible) from layer N+1.
/// The final layer has no outgoing edges (it's the "leaf" infrastructure --
/// databases, caches -- everything else ultimately depends on).
fn build_layered_graph(layers: usize, fan_out: usize, edges_per_node: usize) -> Graph {
    let mut graph = Graph::new();
    let mut layer_ids: Vec<Vec<pyblastradius::graph::node::NodeId>> = Vec::with_capacity(layers);

    for layer in 0..layers {
        let mut ids = Vec::with_capacity(fan_out);
        for i in 0..fan_out {
            let node_type = if layer == layers - 1 {
                NodeType::Database
            } else {
                NodeType::Service
            };
            let node = Node::new(format!("layer{layer}_node{i}"), node_type)
                .with_criticality(0.5 + (i as f32 % 5.0) / 10.0);
            ids.push(graph.add_node(node).expect("add_node"));
        }
        layer_ids.push(ids);
    }

    for layer in 0..layers.saturating_sub(1) {
        let (source_layer, target_layer) = (&layer_ids[layer], &layer_ids[layer + 1]);
        for (i, &source_id) in source_layer.iter().enumerate() {
            for k in 0..edges_per_node.min(target_layer.len()) {
                let target_idx = (i + k * 7 + 1) % target_layer.len();
                let edge = Edge::new(EdgeType::DependsOn, source_id, target_layer[target_idx])
                    .with_criticality(k == 0);
                graph.add_edge(edge).expect("add_edge");
            }
        }
    }

    graph
}

fn bench_graph_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("graph_construction");
    for &(layers, fan_out) in &[(5, 20), (10, 50), (10, 100)] {
        let node_count = layers * fan_out;
        group.bench_with_input(
            BenchmarkId::new("build_layered_graph", node_count),
            &(layers, fan_out),
            |b, &(layers, fan_out)| {
                b.iter(|| build_layered_graph(layers, fan_out, 3));
            },
        );
    }
    group.finish();
}

fn bench_blast_radius_analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("blast_radius_analyze");
    for &(layers, fan_out) in &[(5, 20), (10, 50), (10, 100)] {
        let graph = build_layered_graph(layers, fan_out, 3);
        let node_count = graph.node_count();
        // Analyze from a node in the first layer, so the blast radius
        // traversal actually has the full graph depth to walk -- picking a
        // leaf node would make this benchmark measure almost nothing.
        let source = graph.nodes().next().unwrap().id;

        group.bench_with_input(BenchmarkId::from_parameter(node_count), &source, |b, &source| {
            b.iter(|| BlastRadiusAnalyzer::analyze(&graph, source).unwrap());
        });
    }
    group.finish();
}

fn bench_criticality_scoring(c: &mut Criterion) {
    let mut group = c.benchmark_group("criticality_score_all");
    for &(layers, fan_out) in &[(5, 20), (10, 50), (10, 100)] {
        let graph = build_layered_graph(layers, fan_out, 3);
        let node_count = graph.node_count();

        group.bench_with_input(BenchmarkId::from_parameter(node_count), &graph, |b, graph| {
            b.iter(|| CriticalityScorer::score_all(graph).unwrap());
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_graph_construction,
    bench_blast_radius_analysis,
    bench_criticality_scoring
);
criterion_main!(benches);
