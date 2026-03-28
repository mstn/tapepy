use std::collections::BTreeSet;
use std::fmt;

use graphviz_rust::dot_structures::{
    Attribute, Edge, EdgeTy, Graph, Id, Node, NodeId, Port, Stmt, Subgraph, Vertex,
};
use graphviz_rust::{
    cmd::{CommandArg, Format},
    exec,
    printer::PrinterContext,
};
use open_hypergraphs::lax::OpenHypergraph;
use open_hypergraphs_dot::{Options, Theme};

use crate::hypergraph_utils::connected_components_with_edges;
use crate::tape_language::{Monomial, MonomialHyperNode, MonomialTapeEdge, TapeEdge};
use crate::types::TypeExpr;

#[derive(Debug, Clone, PartialEq)]
pub enum CommandEdge {
    Atom(String),
    Embedded(Box<OpenHypergraph<TypeExpr, CommandEdge>>),
}

impl std::fmt::Display for CommandEdge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandEdge::Atom(label) => write!(f, "{}", label),
            CommandEdge::Embedded(child) => {
                write!(
                    f,
                    "Embedded({}x{})",
                    child.sources.len(),
                    child.targets.len()
                )
            }
        }
    }
}

pub fn to_svg_with_clusters<O: Clone>(
    graph: &OpenHypergraph<O, CommandEdge>,
    opts: &Options<O, CommandEdge>,
) -> Result<Vec<u8>, std::io::Error> {
    let dot_graph = generate_dot_with_clusters(graph, opts);
    exec(
        dot_graph,
        &mut PrinterContext::default(),
        vec![CommandArg::Format(Format::Svg)],
    )
}

pub fn generate_dot_with_clusters<O: Clone>(
    graph: &OpenHypergraph<O, CommandEdge>,
    opts: &Options<O, CommandEdge>,
) -> Graph {
    let theme = &opts.theme;

    let mut dot_graph = Graph::DiGraph {
        id: Id::Plain(String::from("G")),
        strict: false,
        stmts: Vec::new(),
    };

    dot_graph.add_stmt(Stmt::Attribute(Attribute(
        Id::Plain(String::from("rankdir")),
        Id::Plain(opts.orientation.to_string()),
    )));
    dot_graph.add_stmt(Stmt::Attribute(Attribute(
        Id::Plain(String::from("compound")),
        Id::Plain(String::from("true")),
    )));

    dot_graph.add_stmt(Stmt::Attribute(Attribute(
        Id::Plain(String::from("bgcolor")),
        Id::Plain(format!("\"{}\"", theme.bgcolor.clone())),
    )));

    dot_graph.add_stmt(Stmt::Node(Node {
        id: NodeId(Id::Plain(String::from("node")), None),
        attributes: vec![
            Attribute(
                Id::Plain(String::from("shape")),
                Id::Plain(String::from("record")),
            ),
            Attribute(
                Id::Plain(String::from("style")),
                Id::Plain(String::from("rounded")),
            ),
            Attribute(
                Id::Plain(String::from("fontcolor")),
                Id::Plain(format!("\"{}\"", theme.fontcolor.clone())),
            ),
            Attribute(
                Id::Plain(String::from("color")),
                Id::Plain(format!("\"{}\"", theme.color.clone())),
            ),
        ],
    }));

    dot_graph.add_stmt(Stmt::Node(Node {
        id: NodeId(Id::Plain(String::from("edge")), None),
        attributes: vec![
            Attribute(
                Id::Plain(String::from("fontcolor")),
                Id::Plain(format!("\"{}\"", theme.fontcolor.clone())),
            ),
            Attribute(
                Id::Plain(String::from("color")),
                Id::Plain(format!("\"{}\"", theme.color.clone())),
            ),
            Attribute(
                Id::Plain(String::from("arrowhead")),
                Id::Plain(String::from("none")),
            ),
        ],
    }));

    extend_graph(&mut dot_graph, graph, opts, "".to_string());
    dot_graph
}

pub fn to_svg_with_embedded_clusters<O: Clone, E: Clone + fmt::Display>(
    graph: &OpenHypergraph<O, OpenHypergraph<TypeExpr, E>>,
    opts: &Options<O, CommandEdge>,
) -> Result<Vec<u8>, std::io::Error> {
    let dot_graph = generate_dot_with_embedded_clusters(graph, opts);
    exec(
        dot_graph,
        &mut PrinterContext::default(),
        vec![CommandArg::Format(Format::Svg)],
    )
}

pub fn generate_dot_with_embedded_clusters<O: Clone, E: Clone + fmt::Display>(
    graph: &OpenHypergraph<O, OpenHypergraph<TypeExpr, E>>,
    opts: &Options<O, CommandEdge>,
) -> Graph {
    let wrapped = graph.clone().map_edges(|edge| {
        let child = edge
            .clone()
            .map_edges(|gen| CommandEdge::Atom(gen.to_string()));
        CommandEdge::Embedded(Box::new(child))
    });
    generate_dot_with_clusters(&wrapped, opts)
}

pub fn to_svg_with_tape_clusters<G: Clone + fmt::Display>(
    graph: &OpenHypergraph<Monomial<TypeExpr>, TapeEdge<TypeExpr, G>>,
    opts: &Options<Monomial<TypeExpr>, CommandEdge>,
) -> Result<Vec<u8>, std::io::Error> {
    let dot_graph = generate_dot_with_tape_clusters(graph, opts);
    exec(
        dot_graph,
        &mut PrinterContext::default(),
        vec![CommandArg::Format(Format::Svg)],
    )
}

pub fn generate_dot_with_tape_clusters<G: Clone + fmt::Display>(
    graph: &OpenHypergraph<Monomial<TypeExpr>, TapeEdge<TypeExpr, G>>,
    opts: &Options<Monomial<TypeExpr>, CommandEdge>,
) -> Graph {
    let theme = &opts.theme;

    let mut dot_graph = Graph::DiGraph {
        id: Id::Plain(String::from("G")),
        strict: false,
        stmts: Vec::new(),
    };

    dot_graph.add_stmt(Stmt::Attribute(Attribute(
        Id::Plain(String::from("rankdir")),
        Id::Plain(opts.orientation.to_string()),
    )));
    dot_graph.add_stmt(Stmt::Attribute(Attribute(
        Id::Plain(String::from("compound")),
        Id::Plain(String::from("true")),
    )));

    dot_graph.add_stmt(Stmt::Attribute(Attribute(
        Id::Plain(String::from("bgcolor")),
        Id::Plain(format!("\"{}\"", theme.bgcolor.clone())),
    )));

    dot_graph.add_stmt(Stmt::Node(Node {
        id: NodeId(Id::Plain(String::from("node")), None),
        attributes: vec![
            Attribute(
                Id::Plain(String::from("shape")),
                Id::Plain(String::from("record")),
            ),
            Attribute(
                Id::Plain(String::from("style")),
                Id::Plain(String::from("rounded")),
            ),
            Attribute(
                Id::Plain(String::from("fontcolor")),
                Id::Plain(format!("\"{}\"", theme.fontcolor.clone())),
            ),
            Attribute(
                Id::Plain(String::from("color")),
                Id::Plain(format!("\"{}\"", theme.color.clone())),
            ),
        ],
    }));

    dot_graph.add_stmt(Stmt::Node(Node {
        id: NodeId(Id::Plain(String::from("edge")), None),
        attributes: vec![
            Attribute(
                Id::Plain(String::from("fontcolor")),
                Id::Plain(format!("\"{}\"", theme.fontcolor.clone())),
            ),
            Attribute(
                Id::Plain(String::from("color")),
                Id::Plain(format!("\"{}\"", theme.color.clone())),
            ),
            Attribute(
                Id::Plain(String::from("arrowhead")),
                Id::Plain(String::from("none")),
            ),
        ],
    }));

    extend_tape_graph(&mut dot_graph, graph, opts, "".to_string());
    dot_graph
}

pub fn to_svg_with_monomial_tape_clusters<G: Clone + fmt::Display>(
    graph: &OpenHypergraph<MonomialHyperNode<TypeExpr>, MonomialTapeEdge<TypeExpr, G>>,
    opts: &Options<MonomialHyperNode<TypeExpr>, CommandEdge>,
) -> Result<Vec<u8>, std::io::Error> {
    let dot_graph = generate_dot_with_monomial_tape_clusters(graph, opts);
    exec(
        dot_graph,
        &mut PrinterContext::default(),
        vec![CommandArg::Format(Format::Svg)],
    )
}

pub fn generate_dot_with_monomial_tape_clusters<G: Clone + fmt::Display>(
    graph: &OpenHypergraph<MonomialHyperNode<TypeExpr>, MonomialTapeEdge<TypeExpr, G>>,
    opts: &Options<MonomialHyperNode<TypeExpr>, CommandEdge>,
) -> Graph {
    let theme = &opts.theme;

    let mut dot_graph = Graph::DiGraph {
        id: Id::Plain(String::from("G")),
        strict: false,
        stmts: Vec::new(),
    };

    dot_graph.add_stmt(Stmt::Attribute(Attribute(
        Id::Plain(String::from("rankdir")),
        Id::Plain(opts.orientation.to_string()),
    )));
    dot_graph.add_stmt(Stmt::Attribute(Attribute(
        Id::Plain(String::from("compound")),
        Id::Plain(String::from("true")),
    )));

    dot_graph.add_stmt(Stmt::Attribute(Attribute(
        Id::Plain(String::from("bgcolor")),
        Id::Plain(format!("\"{}\"", theme.bgcolor.clone())),
    )));

    dot_graph.add_stmt(Stmt::Node(Node {
        id: NodeId(Id::Plain(String::from("edge")), None),
        attributes: vec![
            Attribute(
                Id::Plain(String::from("fontcolor")),
                Id::Plain(format!("\"{}\"", theme.fontcolor.clone())),
            ),
            Attribute(
                Id::Plain(String::from("color")),
                Id::Plain(format!("\"{}\"", theme.color.clone())),
            ),
            Attribute(
                Id::Plain(String::from("arrowhead")),
                Id::Plain(String::from("none")),
            ),
        ],
    }));

    extend_monomial_tape_graph(&mut dot_graph, graph, opts, "".to_string());
    dot_graph
}

fn extend_graph<O: Clone>(
    dot_graph: &mut Graph,
    graph: &OpenHypergraph<O, CommandEdge>,
    opts: &Options<O, CommandEdge>,
    prefix: String,
) {
    for stmt in generate_node_stmts(graph, opts, &prefix) {
        dot_graph.add_stmt(stmt);
    }
    for stmt in generate_edge_stmts(graph, opts, &prefix) {
        dot_graph.add_stmt(stmt);
    }
    for stmt in generate_interface_stmts(graph, &prefix) {
        dot_graph.add_stmt(stmt);
    }
    for stmt in generate_connection_stmts(graph, &prefix) {
        dot_graph.add_stmt(stmt);
    }
    for stmt in generate_quotient_stmts(graph, &prefix) {
        dot_graph.add_stmt(stmt);
    }
    for stmt in generate_edge_clusters(graph, opts, &prefix) {
        dot_graph.add_stmt(stmt);
    }
}

fn edge_label<O>(edge: &CommandEdge, opts: &Options<O, CommandEdge>) -> String {
    match edge {
        CommandEdge::Atom(_) => (opts.edge_label)(edge),
        CommandEdge::Embedded(_) => "Embed".to_string(),
    }
}

fn child_opts_from_parent<O>(opts: &Options<O, CommandEdge>) -> Options<TypeExpr, CommandEdge> {
    Options {
        orientation: opts.orientation,
        theme: Theme {
            bgcolor: opts.theme.bgcolor.clone(),
            fontcolor: opts.theme.fontcolor.clone(),
            color: opts.theme.color.clone(),
            orientation: opts.theme.orientation,
        },
        node_label: Box::new(|n: &TypeExpr| n.to_string()),
        edge_label: Box::new(|e: &CommandEdge| e.to_string()),
    }
}

fn extend_tape_graph<G: Clone + fmt::Display>(
    dot_graph: &mut Graph,
    graph: &OpenHypergraph<Monomial<TypeExpr>, TapeEdge<TypeExpr, G>>,
    opts: &Options<Monomial<TypeExpr>, CommandEdge>,
    prefix: String,
) {
    for stmt in generate_tape_node_stmts(graph, opts, &prefix) {
        dot_graph.add_stmt(stmt);
    }
    for stmt in generate_tape_interface_stmts(graph, &prefix) {
        dot_graph.add_stmt(stmt);
    }
    for stmt in generate_tape_quotient_stmts(graph, &prefix) {
        dot_graph.add_stmt(stmt);
    }
    for stmt in generate_tape_edge_clusters(graph, opts, &prefix) {
        dot_graph.add_stmt(stmt);
    }
}

fn extend_monomial_tape_graph<G: Clone + fmt::Display>(
    dot_graph: &mut Graph,
    graph: &OpenHypergraph<MonomialHyperNode<TypeExpr>, MonomialTapeEdge<TypeExpr, G>>,
    opts: &Options<MonomialHyperNode<TypeExpr>, CommandEdge>,
    prefix: String,
) {
    let graph = graph
        .clone()
        .map_edges(|edge| CommandEdge::Atom(edge.to_string()));
    for stmt in generate_monomial_node_stmts(&graph, opts, &prefix) {
        dot_graph.add_stmt(stmt);
    }
    for stmt in generate_edge_stmts(&graph, opts, &prefix) {
        dot_graph.add_stmt(stmt);
    }
    for stmt in generate_interface_stmts(&graph, &prefix) {
        dot_graph.add_stmt(stmt);
    }
    for stmt in generate_connection_stmts(&graph, &prefix) {
        dot_graph.add_stmt(stmt);
    }
    for stmt in generate_quotient_stmts(&graph, &prefix) {
        dot_graph.add_stmt(stmt);
    }
    for stmt in generate_edge_clusters(&graph, opts, &prefix) {
        dot_graph.add_stmt(stmt);
    }
}

fn generate_monomial_node_stmts(
    graph: &OpenHypergraph<MonomialHyperNode<TypeExpr>, CommandEdge>,
    opts: &Options<MonomialHyperNode<TypeExpr>, CommandEdge>,
    prefix: &str,
) -> Vec<Stmt> {
    let mut stmts = Vec::new();
    for i in 0..graph.hypergraph.nodes.len() {
        let node = &graph.hypergraph.nodes[i];
        let label = escape_dot_label(&(opts.node_label)(node));
        let (fillcolor, fontcolor) = match node.tensor_kind {
            crate::tape_language::TensorKind::Multiplicative => ("#000000", "#ffffff"),
            crate::tape_language::TensorKind::Additive => ("#ffffff", "#000000"),
        };
        stmts.push(Stmt::Node(Node {
            id: NodeId(Id::Plain(format!("{}n_{}", prefix, i)), None),
            attributes: vec![
                Attribute(
                    Id::Plain(String::from("shape")),
                    Id::Plain(String::from("circle")),
                ),
                Attribute(
                    Id::Plain(String::from("style")),
                    Id::Plain(String::from("filled")),
                ),
                Attribute(
                    Id::Plain(String::from("fillcolor")),
                    Id::Plain(format!("\"{}\"", fillcolor)),
                ),
                Attribute(
                    Id::Plain(String::from("color")),
                    Id::Plain(String::from("\"#000000\"")),
                ),
                Attribute(
                    Id::Plain(String::from("fontcolor")),
                    Id::Plain(format!("\"{}\"", fontcolor)),
                ),
                Attribute(Id::Plain(String::from("label")), Id::Plain(String::from("\"\""))),
                Attribute(
                    Id::Plain(String::from("width")),
                    Id::Plain(String::from("0.18")),
                ),
                Attribute(
                    Id::Plain(String::from("height")),
                    Id::Plain(String::from("0.18")),
                ),
                Attribute(
                    Id::Plain(String::from("fixedsize")),
                    Id::Plain(String::from("true")),
                ),
                Attribute(
                    Id::Plain(String::from("xlabel")),
                    Id::Plain(format!("\"{}\"", label)),
                ),
            ],
        }));
    }
    stmts
}

fn generate_tape_node_stmts<E>(
    graph: &OpenHypergraph<Monomial<TypeExpr>, E>,
    opts: &Options<Monomial<TypeExpr>, CommandEdge>,
    prefix: &str,
) -> Vec<Stmt> {
    let mut stmts = Vec::new();
    for i in 0..graph.hypergraph.nodes.len() {
        let label = (opts.node_label)(&graph.hypergraph.nodes[i]);
        let label = escape_dot_label(&label);
        stmts.push(Stmt::Node(Node {
            id: NodeId(Id::Plain(format!("{}n_{}", prefix, i)), None),
            attributes: vec![
                Attribute(
                    Id::Plain(String::from("shape")),
                    Id::Plain(String::from("point")),
                ),
                Attribute(
                    Id::Plain(String::from("xlabel")),
                    Id::Plain(format!("\"{}\"", label)),
                ),
            ],
        }));
    }
    stmts
}

fn generate_tape_interface_stmts<O: Clone, E>(
    graph: &OpenHypergraph<O, E>,
    prefix: &str,
) -> Vec<Stmt> {
    let mut stmts = Vec::new();

    if !graph.sources.is_empty() {
        let mut source_ports = String::new();
        for i in 0..graph.sources.len() {
            source_ports.push_str(&format!("<p_{i}> | "));
        }
        if !source_ports.is_empty() {
            source_ports.truncate(source_ports.len() - 3);
        }

        stmts.push(Stmt::Node(Node {
            id: NodeId(Id::Plain(format!("{}sources", prefix)), None),
            attributes: vec![
                Attribute(
                    Id::Plain(String::from("label")),
                    Id::Plain(format!("\"{{ {{}} | {{ {} }} }}\"", source_ports)),
                ),
                Attribute(
                    Id::Plain(String::from("shape")),
                    Id::Plain(String::from("record")),
                ),
                Attribute(
                    Id::Plain(String::from("style")),
                    Id::Plain(String::from("invisible")),
                ),
                Attribute(
                    Id::Plain(String::from("rank")),
                    Id::Plain(String::from("source")),
                ),
            ],
        }));

        for (i, &source_node_id) in graph.sources.iter().enumerate() {
            let edge = Edge {
                ty: EdgeTy::Pair(
                    Vertex::N(NodeId(
                        Id::Plain(format!("{}sources", prefix)),
                        Some(Port(None, Some(format!("p_{}", i)))),
                    )),
                    Vertex::N(NodeId(
                        Id::Plain(format!("{}n_{}", prefix, source_node_id.0)),
                        None,
                    )),
                ),
                attributes: vec![Attribute(
                    Id::Plain(String::from("style")),
                    Id::Plain(String::from("dashed")),
                )],
            };
            stmts.push(Stmt::Edge(edge));
        }
    }

    if !graph.targets.is_empty() {
        let mut target_ports = String::new();
        for i in 0..graph.targets.len() {
            target_ports.push_str(&format!("<p_{i}> | "));
        }
        if !target_ports.is_empty() {
            target_ports.truncate(target_ports.len() - 3);
        }

        stmts.push(Stmt::Node(Node {
            id: NodeId(Id::Plain(format!("{}targets", prefix)), None),
            attributes: vec![
                Attribute(
                    Id::Plain(String::from("label")),
                    Id::Plain(format!("\"{{ {{ {} }} | {{}} }}\"", target_ports)),
                ),
                Attribute(
                    Id::Plain(String::from("shape")),
                    Id::Plain(String::from("record")),
                ),
                Attribute(
                    Id::Plain(String::from("style")),
                    Id::Plain(String::from("invisible")),
                ),
                Attribute(
                    Id::Plain(String::from("rank")),
                    Id::Plain(String::from("sink")),
                ),
            ],
        }));

        for (i, &target_node_id) in graph.targets.iter().enumerate() {
            let edge = Edge {
                ty: EdgeTy::Pair(
                    Vertex::N(NodeId(
                        Id::Plain(format!("{}n_{}", prefix, target_node_id.0)),
                        None,
                    )),
                    Vertex::N(NodeId(
                        Id::Plain(format!("{}targets", prefix)),
                        Some(Port(None, Some(format!("p_{}", i)))),
                    )),
                ),
                attributes: vec![Attribute(
                    Id::Plain(String::from("style")),
                    Id::Plain(String::from("dashed")),
                )],
            };
            stmts.push(Stmt::Edge(edge));
        }
    }

    stmts
}

fn generate_tape_quotient_stmts<O: Clone, E>(
    graph: &OpenHypergraph<O, E>,
    prefix: &str,
) -> Vec<Stmt> {
    let mut stmts = Vec::new();
    let (lefts, rights) = &graph.hypergraph.quotient;
    let mut unified_nodes = std::collections::BTreeSet::new();

    for (left, right) in lefts.iter().zip(rights.iter()) {
        let left_idx = left.0;
        let right_idx = right.0;
        let pair_key = if left_idx < right_idx {
            (left_idx, right_idx)
        } else {
            (right_idx, left_idx)
        };

        if unified_nodes.insert(pair_key) {
            let edge = Edge {
                ty: EdgeTy::Pair(
                    Vertex::N(NodeId(Id::Plain(format!("{}n_{}", prefix, left_idx)), None)),
                    Vertex::N(NodeId(
                        Id::Plain(format!("{}n_{}", prefix, right_idx)),
                        None,
                    )),
                ),
                attributes: vec![Attribute(
                    Id::Plain(String::from("style")),
                    Id::Plain(String::from("dotted")),
                )],
            };
            stmts.push(Stmt::Edge(edge));
        }
    }

    stmts
}

fn generate_tape_edge_clusters<G: Clone + fmt::Display>(
    graph: &OpenHypergraph<Monomial<TypeExpr>, TapeEdge<TypeExpr, G>>,
    opts: &Options<Monomial<TypeExpr>, CommandEdge>,
    prefix: &str,
) -> Vec<Stmt> {
    let mut stmts = Vec::new();
    let theme = &opts.theme;

    for (edge_idx, edge_label) in graph.hypergraph.edges.iter().enumerate() {
        match edge_label {
            TapeEdge::Embedded(child) => {
                let child_opts = child_opts_from_parent(opts);
                let cluster_id = format!("cluster_{}e_{}", prefix, edge_idx);
                let mut cluster = Subgraph {
                    id: Id::Plain(cluster_id),
                    stmts: vec![
                        Stmt::Attribute(Attribute(
                            Id::Plain(String::from("label")),
                            Id::Plain(String::from("\"\"")),
                        )),
                        Stmt::Attribute(Attribute(
                            Id::Plain(String::from("color")),
                            Id::Plain(format!("\"{}\"", theme.color.clone())),
                        )),
                        Stmt::Attribute(Attribute(
                            Id::Plain(String::from("fontcolor")),
                            Id::Plain(format!("\"{}\"", theme.fontcolor.clone())),
                        )),
                        Stmt::Attribute(Attribute(
                            Id::Plain(String::from("style")),
                            Id::Plain(String::from("rounded")),
                        )),
                    ],
                };

                let parent_sources_len = graph.hypergraph.adjacency[edge_idx].sources.len();
                let parent_targets_len = graph.hypergraph.adjacency[edge_idx].targets.len();
                let child_graph = child
                    .clone()
                    .map_edges(|gen| CommandEdge::Atom(gen.to_string()));
                let child_graph =
                    ensure_interface_lengths(child_graph, parent_sources_len, parent_targets_len);
                let child_prefix = format!("{}e_{}_c0_", prefix, edge_idx);
                for (j, &child_source) in child_graph.sources.iter().enumerate() {
                    let parent_node = graph.hypergraph.adjacency[edge_idx].sources[j];
                    let edge = Edge {
                        ty: EdgeTy::Pair(
                            Vertex::N(NodeId(
                                Id::Plain(format!("{}n_{}", prefix, parent_node.0)),
                                None,
                            )),
                            Vertex::N(NodeId(
                                Id::Plain(format!("{}n_{}", child_prefix, child_source.0)),
                                None,
                            )),
                        ),
                        attributes: vec![Attribute(
                            Id::Plain(String::from("style")),
                            Id::Plain(String::from("dotted")),
                        )],
                    };
                    stmts.push(Stmt::Edge(edge));
                }
                for (j, &child_target) in child_graph.targets.iter().enumerate() {
                    let parent_node = graph.hypergraph.adjacency[edge_idx].targets[j];
                    let edge = Edge {
                        ty: EdgeTy::Pair(
                            Vertex::N(NodeId(
                                Id::Plain(format!("{}n_{}", child_prefix, child_target.0)),
                                None,
                            )),
                            Vertex::N(NodeId(
                                Id::Plain(format!("{}n_{}", prefix, parent_node.0)),
                                None,
                            )),
                        ),
                        attributes: vec![Attribute(
                            Id::Plain(String::from("style")),
                            Id::Plain(String::from("dotted")),
                        )],
                    };
                    stmts.push(Stmt::Edge(edge));
                }

                for stmt in generate_interface_stmts(&child_graph, &child_prefix) {
                    cluster.add_stmt(stmt);
                }
                let components = connected_components_with_edges(&child_graph);
                let mut anchors = Vec::new();
                for (component_idx, component) in components.iter().enumerate() {
                    let component_id =
                        format!("cluster_{}e_{}_cc_{}", prefix, edge_idx, component_idx);
                    let mut component_cluster = Subgraph {
                        id: Id::Plain(component_id),
                        stmts: vec![
                            Stmt::Attribute(Attribute(
                                Id::Plain(String::from("label")),
                                Id::Plain(String::from("\"\"")),
                            )),
                            Stmt::Attribute(Attribute(
                                Id::Plain(String::from("style")),
                                Id::Plain(String::from("dashed")),
                            )),
                        ],
                    };

                    let node_set: BTreeSet<usize> =
                        component.nodes.iter().map(|node| node.0).collect();
                    let edge_set: BTreeSet<usize> = component.edges.iter().cloned().collect();

                    for stmt in generate_node_stmts_subset(
                        &child_graph,
                        &child_opts,
                        &child_prefix,
                        &node_set,
                    ) {
                        component_cluster.add_stmt(stmt);
                    }
                    for stmt in generate_edge_stmts_subset(
                        &child_graph,
                        &child_opts,
                        &child_prefix,
                        &edge_set,
                    ) {
                        component_cluster.add_stmt(stmt);
                    }
                    for stmt in
                        generate_connection_stmts_subset(&child_graph, &child_prefix, &edge_set)
                    {
                        component_cluster.add_stmt(stmt);
                    }
                    for stmt in
                        generate_quotient_stmts_subset(&child_graph, &child_prefix, &node_set)
                    {
                        component_cluster.add_stmt(stmt);
                    }

                    if let Some(node) = component.nodes.first() {
                        anchors.push(format!("{}n_{}", child_prefix, node.0));
                    } else if let Some(edge_idx) = component.edges.first() {
                        anchors.push(format!("{}e_{}", child_prefix, edge_idx));
                    }

                    cluster.add_stmt(Stmt::Subgraph(component_cluster));
                }

                for window in anchors.windows(2) {
                    let edge = Edge {
                        ty: EdgeTy::Pair(
                            Vertex::N(NodeId(Id::Plain(window[0].clone()), None)),
                            Vertex::N(NodeId(Id::Plain(window[1].clone()), None)),
                        ),
                        attributes: vec![
                            Attribute(
                                Id::Plain(String::from("style")),
                                Id::Plain(String::from("invis")),
                            ),
                            Attribute(
                                Id::Plain(String::from("constraint")),
                                Id::Plain(String::from("true")),
                            ),
                        ],
                    };
                    cluster.add_stmt(Stmt::Edge(edge));
                }

                stmts.push(Stmt::Subgraph(cluster));
            }
            TapeEdge::Product(left, right) => {
                let cluster_id = format!("cluster_{}e_{}", prefix, edge_idx);
                let mut cluster = Subgraph {
                    id: Id::Plain(cluster_id),
                    stmts: vec![
                        Stmt::Attribute(Attribute(
                            Id::Plain(String::from("label")),
                            Id::Plain(String::from("\"Product\"")),
                        )),
                        Stmt::Attribute(Attribute(
                            Id::Plain(String::from("color")),
                            Id::Plain(format!("\"{}\"", theme.color.clone())),
                        )),
                        Stmt::Attribute(Attribute(
                            Id::Plain(String::from("fontcolor")),
                            Id::Plain(format!("\"{}\"", theme.fontcolor.clone())),
                        )),
                        Stmt::Attribute(Attribute(
                            Id::Plain(String::from("style")),
                            Id::Plain(String::from("rounded")),
                        )),
                    ],
                };

                let parent_edge = &graph.hypergraph.adjacency[edge_idx];
                let left_sources_len = left.sources.len();
                let left_targets_len = left.targets.len();
                let (parent_left_sources, parent_right_sources) =
                    parent_edge.sources.split_at(left_sources_len);
                let (parent_left_targets, parent_right_targets) =
                    parent_edge.targets.split_at(left_targets_len);

                let left_prefix = format!("{}e_{}_l_", prefix, edge_idx);
                let right_prefix = format!("{}e_{}_r_", prefix, edge_idx);

                for (outer_node, &child_node) in parent_left_sources.iter().zip(left.sources.iter())
                {
                    let edge = Edge {
                        ty: EdgeTy::Pair(
                            Vertex::N(NodeId(
                                Id::Plain(format!("{}n_{}", prefix, outer_node.0)),
                                None,
                            )),
                            Vertex::N(NodeId(
                                Id::Plain(format!("{}n_{}", left_prefix, child_node.0)),
                                None,
                            )),
                        ),
                        attributes: vec![Attribute(
                            Id::Plain(String::from("style")),
                            Id::Plain(String::from("dotted")),
                        )],
                    };
                    stmts.push(Stmt::Edge(edge));
                }
                for (outer_node, &child_node) in parent_left_targets.iter().zip(left.targets.iter())
                {
                    let edge = Edge {
                        ty: EdgeTy::Pair(
                            Vertex::N(NodeId(
                                Id::Plain(format!("{}n_{}", left_prefix, child_node.0)),
                                None,
                            )),
                            Vertex::N(NodeId(
                                Id::Plain(format!("{}n_{}", prefix, outer_node.0)),
                                None,
                            )),
                        ),
                        attributes: vec![Attribute(
                            Id::Plain(String::from("style")),
                            Id::Plain(String::from("dotted")),
                        )],
                    };
                    stmts.push(Stmt::Edge(edge));
                }
                for (outer_node, &child_node) in
                    parent_right_sources.iter().zip(right.sources.iter())
                {
                    let edge = Edge {
                        ty: EdgeTy::Pair(
                            Vertex::N(NodeId(
                                Id::Plain(format!("{}n_{}", prefix, outer_node.0)),
                                None,
                            )),
                            Vertex::N(NodeId(
                                Id::Plain(format!("{}n_{}", right_prefix, child_node.0)),
                                None,
                            )),
                        ),
                        attributes: vec![Attribute(
                            Id::Plain(String::from("style")),
                            Id::Plain(String::from("dotted")),
                        )],
                    };
                    stmts.push(Stmt::Edge(edge));
                }
                for (outer_node, &child_node) in
                    parent_right_targets.iter().zip(right.targets.iter())
                {
                    let edge = Edge {
                        ty: EdgeTy::Pair(
                            Vertex::N(NodeId(
                                Id::Plain(format!("{}n_{}", right_prefix, child_node.0)),
                                None,
                            )),
                            Vertex::N(NodeId(
                                Id::Plain(format!("{}n_{}", prefix, outer_node.0)),
                                None,
                            )),
                        ),
                        attributes: vec![Attribute(
                            Id::Plain(String::from("style")),
                            Id::Plain(String::from("dotted")),
                        )],
                    };
                    stmts.push(Stmt::Edge(edge));
                }

                let mut left_cluster = Subgraph {
                    id: Id::Plain(format!("cluster_{}", left_prefix)),
                    stmts: vec![Stmt::Attribute(Attribute(
                        Id::Plain(String::from("label")),
                        Id::Plain(String::from("\"left\"")),
                    ))],
                };
                for stmt in generate_tape_node_stmts(left, opts, &left_prefix) {
                    left_cluster.add_stmt(stmt);
                }
                for stmt in generate_tape_interface_stmts(left, &left_prefix) {
                    left_cluster.add_stmt(stmt);
                }
                for stmt in generate_tape_quotient_stmts(left, &left_prefix) {
                    left_cluster.add_stmt(stmt);
                }
                for stmt in generate_tape_edge_clusters(left, opts, &left_prefix) {
                    left_cluster.add_stmt(stmt);
                }

                let mut right_cluster = Subgraph {
                    id: Id::Plain(format!("cluster_{}", right_prefix)),
                    stmts: vec![Stmt::Attribute(Attribute(
                        Id::Plain(String::from("label")),
                        Id::Plain(String::from("\"right\"")),
                    ))],
                };
                for stmt in generate_tape_node_stmts(right, opts, &right_prefix) {
                    right_cluster.add_stmt(stmt);
                }
                for stmt in generate_tape_interface_stmts(right, &right_prefix) {
                    right_cluster.add_stmt(stmt);
                }
                for stmt in generate_tape_quotient_stmts(right, &right_prefix) {
                    right_cluster.add_stmt(stmt);
                }
                for stmt in generate_tape_edge_clusters(right, opts, &right_prefix) {
                    right_cluster.add_stmt(stmt);
                }

                cluster.add_stmt(Stmt::Subgraph(left_cluster));
                cluster.add_stmt(Stmt::Subgraph(right_cluster));
                stmts.push(Stmt::Subgraph(cluster));
            }
        }
    }

    stmts
}

fn generate_monomial_tape_edge_clusters<G: Clone + fmt::Display>(
    _graph: &OpenHypergraph<MonomialHyperNode<TypeExpr>, MonomialTapeEdge<TypeExpr, G>>,
    _opts: &Options<MonomialHyperNode<TypeExpr>, CommandEdge>,
    _prefix: &str,
) -> Vec<Stmt> {
    Vec::new()
}

fn generate_node_stmts<O: Clone>(
    graph: &OpenHypergraph<O, CommandEdge>,
    opts: &Options<O, CommandEdge>,
    prefix: &str,
) -> Vec<Stmt> {
    let mut stmts = Vec::new();
    for i in 0..graph.hypergraph.nodes.len() {
        let label = (opts.node_label)(&graph.hypergraph.nodes[i]);
        let label = escape_dot_label(&label);
        stmts.push(Stmt::Node(Node {
            id: NodeId(Id::Plain(format!("{}n_{}", prefix, i)), None),
            attributes: vec![
                Attribute(
                    Id::Plain(String::from("shape")),
                    Id::Plain(String::from("point")),
                ),
                Attribute(
                    Id::Plain(String::from("xlabel")),
                    Id::Plain(format!("\"{}\"", label)),
                ),
            ],
        }));
    }
    stmts
}

fn generate_node_stmts_subset<O: Clone>(
    graph: &OpenHypergraph<O, CommandEdge>,
    opts: &Options<O, CommandEdge>,
    prefix: &str,
    nodes: &BTreeSet<usize>,
) -> Vec<Stmt> {
    let mut stmts = Vec::new();
    for &i in nodes.iter() {
        let label = (opts.node_label)(&graph.hypergraph.nodes[i]);
        let label = escape_dot_label(&label);
        stmts.push(Stmt::Node(Node {
            id: NodeId(Id::Plain(format!("{}n_{}", prefix, i)), None),
            attributes: vec![
                Attribute(
                    Id::Plain(String::from("shape")),
                    Id::Plain(String::from("point")),
                ),
                Attribute(
                    Id::Plain(String::from("xlabel")),
                    Id::Plain(format!("\"{}\"", label)),
                ),
            ],
        }));
    }
    stmts
}

fn generate_edge_stmts<O: Clone>(
    graph: &OpenHypergraph<O, CommandEdge>,
    opts: &Options<O, CommandEdge>,
    prefix: &str,
) -> Vec<Stmt> {
    let mut stmts = Vec::new();
    for i in 0..graph.hypergraph.edges.len() {
        let hyperedge = &graph.hypergraph.adjacency[i];
        match &graph.hypergraph.edges[i] {
            CommandEdge::Atom(_) => {
                let raw_label = edge_label(&graph.hypergraph.edges[i], opts);
                let label = escape_dot_label(&raw_label);
                let hide_node = raw_label == "context";

                let mut source_ports = String::new();
                for j in 0..hyperedge.sources.len() {
                    source_ports.push_str(&format!("<s_{j}> | "));
                }
                if !source_ports.is_empty() {
                    source_ports.truncate(source_ports.len() - 3);
                }

                let mut target_ports = String::new();
                for j in 0..hyperedge.targets.len() {
                    target_ports.push_str(&format!("<t_{j}> | "));
                }
                if !target_ports.is_empty() {
                    target_ports.truncate(target_ports.len() - 3);
                }

                let record_label = if source_ports.is_empty() && target_ports.is_empty() {
                    format!("\"{}\"", label)
                } else if source_ports.is_empty() {
                    format!("\"{{ {} | {{ {} }} }}\"", label, target_ports)
                } else if target_ports.is_empty() {
                    format!("\"{{ {{ {} }} | {} }}\"", source_ports, label)
                } else {
                    format!(
                        "\"{{ {{ {} }} | {} | {{ {} }} }}\"",
                        source_ports, label, target_ports
                    )
                };

                let mut attributes = vec![
                    Attribute(Id::Plain(String::from("label")), Id::Plain(record_label)),
                    Attribute(
                        Id::Plain(String::from("shape")),
                        Id::Plain(String::from("record")),
                    ),
                ];
                if hide_node {
                    attributes.push(Attribute(
                        Id::Plain(String::from("style")),
                        Id::Plain(String::from("invis")),
                    ));
                }
                stmts.push(Stmt::Node(Node {
                    id: NodeId(Id::Plain(format!("{}e_{}", prefix, i)), None),
                    attributes,
                }));
            }
            CommandEdge::Embedded(child) => {
                let _child = child;
            }
        }
    }
    stmts
}

fn generate_edge_stmts_subset<O: Clone>(
    graph: &OpenHypergraph<O, CommandEdge>,
    opts: &Options<O, CommandEdge>,
    prefix: &str,
    edges: &BTreeSet<usize>,
) -> Vec<Stmt> {
    let mut stmts = Vec::new();
    for &i in edges.iter() {
        let hyperedge = &graph.hypergraph.adjacency[i];
        match &graph.hypergraph.edges[i] {
            CommandEdge::Atom(_) => {
                let raw_label = edge_label(&graph.hypergraph.edges[i], opts);
                let label = escape_dot_label(&raw_label);
                let hide_node = raw_label == "context";

                let mut source_ports = String::new();
                for j in 0..hyperedge.sources.len() {
                    source_ports.push_str(&format!("<s_{j}> | "));
                }
                if !source_ports.is_empty() {
                    source_ports.truncate(source_ports.len() - 3);
                }

                let mut target_ports = String::new();
                for j in 0..hyperedge.targets.len() {
                    target_ports.push_str(&format!("<t_{j}> | "));
                }
                if !target_ports.is_empty() {
                    target_ports.truncate(target_ports.len() - 3);
                }

                let record_label = if source_ports.is_empty() && target_ports.is_empty() {
                    format!("\"{}\"", label)
                } else if source_ports.is_empty() {
                    format!("\"{{ {} | {{ {} }} }}\"", label, target_ports)
                } else if target_ports.is_empty() {
                    format!("\"{{ {{ {} }} | {} }}\"", source_ports, label)
                } else {
                    format!(
                        "\"{{ {{ {} }} | {} | {{ {} }} }}\"",
                        source_ports, label, target_ports
                    )
                };

                let mut attributes = vec![
                    Attribute(Id::Plain(String::from("label")), Id::Plain(record_label)),
                    Attribute(
                        Id::Plain(String::from("shape")),
                        Id::Plain(String::from("record")),
                    ),
                ];
                if hide_node {
                    attributes.push(Attribute(
                        Id::Plain(String::from("style")),
                        Id::Plain(String::from("invis")),
                    ));
                }
                stmts.push(Stmt::Node(Node {
                    id: NodeId(Id::Plain(format!("{}e_{}", prefix, i)), None),
                    attributes,
                }));
            }
            CommandEdge::Embedded(_) => {}
        }
    }
    stmts
}

fn generate_connection_stmts<O: Clone>(
    graph: &OpenHypergraph<O, CommandEdge>,
    prefix: &str,
) -> Vec<Stmt> {
    let mut stmts = Vec::new();

    for (i, hyperedge) in graph.hypergraph.adjacency.iter().enumerate() {
        if !matches!(graph.hypergraph.edges[i], CommandEdge::Atom(_)) {
            continue;
        }
        for (j, &node_id) in hyperedge.sources.iter().enumerate() {
            let node_idx = node_id.0;
            let port = Some(Port(None, Some(format!("s_{}", j))));
            let edge = Edge {
                ty: EdgeTy::Pair(
                    Vertex::N(NodeId(Id::Plain(format!("{}n_{}", prefix, node_idx)), None)),
                    Vertex::N(NodeId(Id::Plain(format!("{}e_{}", prefix, i)), port)),
                ),
                attributes: vec![],
            };
            stmts.push(Stmt::Edge(edge));
        }

        for (j, &node_id) in hyperedge.targets.iter().enumerate() {
            let node_idx = node_id.0;
            let port = Some(Port(None, Some(format!("t_{}", j))));
            let edge = Edge {
                ty: EdgeTy::Pair(
                    Vertex::N(NodeId(Id::Plain(format!("{}e_{}", prefix, i)), port)),
                    Vertex::N(NodeId(Id::Plain(format!("{}n_{}", prefix, node_idx)), None)),
                ),
                attributes: vec![],
            };
            stmts.push(Stmt::Edge(edge));
        }
    }

    stmts
}

fn generate_connection_stmts_subset<O: Clone>(
    graph: &OpenHypergraph<O, CommandEdge>,
    prefix: &str,
    edges: &BTreeSet<usize>,
) -> Vec<Stmt> {
    let mut stmts = Vec::new();

    for &i in edges.iter() {
        let hyperedge = &graph.hypergraph.adjacency[i];
        if !matches!(graph.hypergraph.edges[i], CommandEdge::Atom(_)) {
            continue;
        }
        for (j, &node_id) in hyperedge.sources.iter().enumerate() {
            let node_idx = node_id.0;
            let port = Some(Port(None, Some(format!("s_{}", j))));
            let edge = Edge {
                ty: EdgeTy::Pair(
                    Vertex::N(NodeId(Id::Plain(format!("{}n_{}", prefix, node_idx)), None)),
                    Vertex::N(NodeId(Id::Plain(format!("{}e_{}", prefix, i)), port)),
                ),
                attributes: vec![],
            };
            stmts.push(Stmt::Edge(edge));
        }

        for (j, &node_id) in hyperedge.targets.iter().enumerate() {
            let node_idx = node_id.0;
            let port = Some(Port(None, Some(format!("t_{}", j))));
            let edge = Edge {
                ty: EdgeTy::Pair(
                    Vertex::N(NodeId(Id::Plain(format!("{}e_{}", prefix, i)), port)),
                    Vertex::N(NodeId(Id::Plain(format!("{}n_{}", prefix, node_idx)), None)),
                ),
                attributes: vec![],
            };
            stmts.push(Stmt::Edge(edge));
        }
    }

    stmts
}

fn generate_interface_stmts<O: Clone>(
    graph: &OpenHypergraph<O, CommandEdge>,
    prefix: &str,
) -> Vec<Stmt> {
    let mut stmts = Vec::new();

    if !graph.sources.is_empty() {
        let mut source_ports = String::new();
        for i in 0..graph.sources.len() {
            source_ports.push_str(&format!("<p_{i}> | "));
        }
        if !source_ports.is_empty() {
            source_ports.truncate(source_ports.len() - 3);
        }

        stmts.push(Stmt::Node(Node {
            id: NodeId(Id::Plain(format!("{}sources", prefix)), None),
            attributes: vec![
                Attribute(
                    Id::Plain(String::from("label")),
                    Id::Plain(format!("\"{{ {{}} | {{ {} }} }}\"", source_ports)),
                ),
                Attribute(
                    Id::Plain(String::from("shape")),
                    Id::Plain(String::from("record")),
                ),
                Attribute(
                    Id::Plain(String::from("style")),
                    Id::Plain(String::from("invisible")),
                ),
                Attribute(
                    Id::Plain(String::from("rank")),
                    Id::Plain(String::from("source")),
                ),
            ],
        }));

        for (i, &source_node_id) in graph.sources.iter().enumerate() {
            let edge = Edge {
                ty: EdgeTy::Pair(
                    Vertex::N(NodeId(
                        Id::Plain(format!("{}sources", prefix)),
                        Some(Port(None, Some(format!("p_{}", i)))),
                    )),
                    Vertex::N(NodeId(
                        Id::Plain(format!("{}n_{}", prefix, source_node_id.0)),
                        None,
                    )),
                ),
                attributes: vec![Attribute(
                    Id::Plain(String::from("style")),
                    Id::Plain(String::from("dashed")),
                )],
            };
            stmts.push(Stmt::Edge(edge));
        }
    }

    if !graph.targets.is_empty() {
        let mut target_ports = String::new();
        for i in 0..graph.targets.len() {
            target_ports.push_str(&format!("<p_{i}> | "));
        }
        if !target_ports.is_empty() {
            target_ports.truncate(target_ports.len() - 3);
        }

        stmts.push(Stmt::Node(Node {
            id: NodeId(Id::Plain(format!("{}targets", prefix)), None),
            attributes: vec![
                Attribute(
                    Id::Plain(String::from("label")),
                    Id::Plain(format!("\"{{ {{ {} }} | {{}} }}\"", target_ports)),
                ),
                Attribute(
                    Id::Plain(String::from("shape")),
                    Id::Plain(String::from("record")),
                ),
                Attribute(
                    Id::Plain(String::from("style")),
                    Id::Plain(String::from("invisible")),
                ),
                Attribute(
                    Id::Plain(String::from("rank")),
                    Id::Plain(String::from("sink")),
                ),
            ],
        }));

        for (i, &target_node_id) in graph.targets.iter().enumerate() {
            let edge = Edge {
                ty: EdgeTy::Pair(
                    Vertex::N(NodeId(
                        Id::Plain(format!("{}n_{}", prefix, target_node_id.0)),
                        None,
                    )),
                    Vertex::N(NodeId(
                        Id::Plain(format!("{}targets", prefix)),
                        Some(Port(None, Some(format!("p_{}", i)))),
                    )),
                ),
                attributes: vec![Attribute(
                    Id::Plain(String::from("style")),
                    Id::Plain(String::from("dashed")),
                )],
            };
            stmts.push(Stmt::Edge(edge));
        }
    }

    stmts
}

fn generate_quotient_stmts_subset<O: Clone>(
    graph: &OpenHypergraph<O, CommandEdge>,
    prefix: &str,
    nodes: &BTreeSet<usize>,
) -> Vec<Stmt> {
    let mut stmts = Vec::new();
    let (lefts, rights) = &graph.hypergraph.quotient;
    let mut unified_nodes = std::collections::BTreeSet::new();

    for (left, right) in lefts.iter().zip(rights.iter()) {
        let left_idx = left.0;
        let right_idx = right.0;
        if !nodes.contains(&left_idx) || !nodes.contains(&right_idx) {
            continue;
        }
        let pair_key = if left_idx < right_idx {
            (left_idx, right_idx)
        } else {
            (right_idx, left_idx)
        };

        if unified_nodes.insert(pair_key) {
            let edge = Edge {
                ty: EdgeTy::Pair(
                    Vertex::N(NodeId(Id::Plain(format!("{}n_{}", prefix, left_idx)), None)),
                    Vertex::N(NodeId(
                        Id::Plain(format!("{}n_{}", prefix, right_idx)),
                        None,
                    )),
                ),
                attributes: vec![Attribute(
                    Id::Plain(String::from("style")),
                    Id::Plain(String::from("dotted")),
                )],
            };
            stmts.push(Stmt::Edge(edge));
        }
    }

    stmts
}

fn generate_quotient_stmts<O: Clone>(
    graph: &OpenHypergraph<O, CommandEdge>,
    prefix: &str,
) -> Vec<Stmt> {
    let mut stmts = Vec::new();
    let (lefts, rights) = &graph.hypergraph.quotient;
    let mut unified_nodes = std::collections::BTreeSet::new();

    for (left, right) in lefts.iter().zip(rights.iter()) {
        let left_idx = left.0;
        let right_idx = right.0;
        let pair_key = if left_idx < right_idx {
            (left_idx, right_idx)
        } else {
            (right_idx, left_idx)
        };

        if unified_nodes.insert(pair_key) {
            let edge = Edge {
                ty: EdgeTy::Pair(
                    Vertex::N(NodeId(Id::Plain(format!("{}n_{}", prefix, left_idx)), None)),
                    Vertex::N(NodeId(
                        Id::Plain(format!("{}n_{}", prefix, right_idx)),
                        None,
                    )),
                ),
                attributes: vec![Attribute(
                    Id::Plain(String::from("style")),
                    Id::Plain(String::from("dotted")),
                )],
            };
            stmts.push(Stmt::Edge(edge));
        }
    }

    stmts
}

fn generate_edge_clusters<O: Clone>(
    graph: &OpenHypergraph<O, CommandEdge>,
    opts: &Options<O, CommandEdge>,
    prefix: &str,
) -> Vec<Stmt> {
    let mut stmts = Vec::new();
    let theme = &opts.theme;

    for (edge_idx, edge_label) in graph.hypergraph.edges.iter().enumerate() {
        match edge_label {
            CommandEdge::Atom(_) => continue,
            CommandEdge::Embedded(child) => {
                let child_opts = child_opts_from_parent(opts);
                let cluster_id = format!("cluster_{}e_{}", prefix, edge_idx);
                let mut cluster = Subgraph {
                    id: Id::Plain(cluster_id),
                    stmts: vec![
                        Stmt::Attribute(Attribute(
                            Id::Plain(String::from("label")),
                            Id::Plain(String::from("\"\"")),
                        )),
                        Stmt::Attribute(Attribute(
                            Id::Plain(String::from("color")),
                            Id::Plain(format!("\"{}\"", theme.color.clone())),
                        )),
                        Stmt::Attribute(Attribute(
                            Id::Plain(String::from("fontcolor")),
                            Id::Plain(format!("\"{}\"", theme.fontcolor.clone())),
                        )),
                        Stmt::Attribute(Attribute(
                            Id::Plain(String::from("style")),
                            Id::Plain(String::from("rounded")),
                        )),
                    ],
                };

                let parent_sources_len = graph.hypergraph.adjacency[edge_idx].sources.len();
                let parent_targets_len = graph.hypergraph.adjacency[edge_idx].targets.len();
                let child = child.as_ref().clone();
                let child = ensure_interface_lengths(child, parent_sources_len, parent_targets_len);
                let child_prefix = format!("{}e_{}_c0_", prefix, edge_idx);
                for (j, &child_source) in child.sources.iter().enumerate() {
                    let parent_node = graph.hypergraph.adjacency[edge_idx].sources[j];
                    let edge = Edge {
                        ty: EdgeTy::Pair(
                            Vertex::N(NodeId(
                                Id::Plain(format!("{}n_{}", prefix, parent_node.0)),
                                None,
                            )),
                            Vertex::N(NodeId(
                                Id::Plain(format!("{}n_{}", child_prefix, child_source.0)),
                                None,
                            )),
                        ),
                        attributes: vec![Attribute(
                            Id::Plain(String::from("style")),
                            Id::Plain(String::from("dotted")),
                        )],
                    };
                    stmts.push(Stmt::Edge(edge));
                }
                for (j, &child_target) in child.targets.iter().enumerate() {
                    let parent_node = graph.hypergraph.adjacency[edge_idx].targets[j];
                    let edge = Edge {
                        ty: EdgeTy::Pair(
                            Vertex::N(NodeId(
                                Id::Plain(format!("{}n_{}", child_prefix, child_target.0)),
                                None,
                            )),
                            Vertex::N(NodeId(
                                Id::Plain(format!("{}n_{}", prefix, parent_node.0)),
                                None,
                            )),
                        ),
                        attributes: vec![Attribute(
                            Id::Plain(String::from("style")),
                            Id::Plain(String::from("dotted")),
                        )],
                    };
                    stmts.push(Stmt::Edge(edge));
                }
                let components = connected_components_with_edges(&child);
                let mut anchors = Vec::new();
                for (component_idx, component) in components.iter().enumerate() {
                    let component_id =
                        format!("cluster_{}e_{}_cc_{}", prefix, edge_idx, component_idx);
                    let mut component_cluster = Subgraph {
                        id: Id::Plain(component_id),
                        stmts: vec![
                            Stmt::Attribute(Attribute(
                                Id::Plain(String::from("label")),
                                Id::Plain(String::from("\"\"")),
                            )),
                            Stmt::Attribute(Attribute(
                                Id::Plain(String::from("style")),
                                Id::Plain(String::from("dashed")),
                            )),
                        ],
                    };

                    let node_set: BTreeSet<usize> =
                        component.nodes.iter().map(|node| node.0).collect();
                    let edge_set: BTreeSet<usize> = component.edges.iter().cloned().collect();

                    for stmt in
                        generate_node_stmts_subset(&child, &child_opts, &child_prefix, &node_set)
                    {
                        component_cluster.add_stmt(stmt);
                    }
                    for stmt in
                        generate_edge_stmts_subset(&child, &child_opts, &child_prefix, &edge_set)
                    {
                        component_cluster.add_stmt(stmt);
                    }
                    for stmt in generate_connection_stmts_subset(&child, &child_prefix, &edge_set) {
                        component_cluster.add_stmt(stmt);
                    }
                    for stmt in generate_quotient_stmts_subset(&child, &child_prefix, &node_set) {
                        component_cluster.add_stmt(stmt);
                    }

                    if let Some(node) = component.nodes.first() {
                        anchors.push(format!("{}n_{}", child_prefix, node.0));
                    } else if let Some(edge_idx) = component.edges.first() {
                        anchors.push(format!("{}e_{}", child_prefix, edge_idx));
                    }

                    cluster.add_stmt(Stmt::Subgraph(component_cluster));
                }

                for window in anchors.windows(2) {
                    let edge = Edge {
                        ty: EdgeTy::Pair(
                            Vertex::N(NodeId(Id::Plain(window[0].clone()), None)),
                            Vertex::N(NodeId(Id::Plain(window[1].clone()), None)),
                        ),
                        attributes: vec![
                            Attribute(
                                Id::Plain(String::from("style")),
                                Id::Plain(String::from("invis")),
                            ),
                            Attribute(
                                Id::Plain(String::from("constraint")),
                                Id::Plain(String::from("true")),
                            ),
                        ],
                    };
                    cluster.add_stmt(Stmt::Edge(edge));
                }
                stmts.push(Stmt::Subgraph(cluster));
            }
        }
    }

    stmts
}

fn escape_dot_label(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            '\\' => Some("\\\\".to_string()),
            '"' => Some("\\\"".to_string()),
            '{' => Some("\\{".to_string()),
            '}' => Some("\\}".to_string()),
            '|' => Some("\\|".to_string()),
            '<' => Some("\\<".to_string()),
            '>' => Some("\\>".to_string()),
            _ => Some(c.to_string()),
        })
        .collect()
}

fn ensure_interface_lengths_generic<O, E>(
    graph: OpenHypergraph<O, E>,
    parent_sources_len: usize,
    parent_targets_len: usize,
) -> OpenHypergraph<O, E> {
    if graph.sources.len() != parent_sources_len || graph.targets.len() != parent_targets_len {
        panic!(
            "subgraph interface mismatch: expected {} -> {}, got {} -> {}",
            parent_sources_len,
            parent_targets_len,
            graph.sources.len(),
            graph.targets.len()
        );
    }

    graph
}

fn ensure_interface_lengths<O>(
    graph: OpenHypergraph<O, CommandEdge>,
    parent_sources_len: usize,
    parent_targets_len: usize,
) -> OpenHypergraph<O, CommandEdge> {
    ensure_interface_lengths_generic(graph, parent_sources_len, parent_targets_len)
}
