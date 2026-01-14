use graphviz_rust::dot_structures::{
    Attribute, Edge, EdgeTy, Graph, Id, Node, NodeId, Port, Stmt, Subgraph, Vertex,
};
use graphviz_rust::{
    cmd::{CommandArg, Format},
    exec,
    printer::PrinterContext,
};
use open_hypergraphs::lax::OpenHypergraph;
use open_hypergraphs_dot::Options;

use crate::command_edge::CommandEdge;
use crate::solver::{apply_substitution, solve_hypergraph_types};
use crate::types::TypeExpr;

pub fn to_svg_with_clusters(
    graph: &OpenHypergraph<TypeExpr, CommandEdge>,
    opts: &Options<TypeExpr, CommandEdge>,
) -> Result<Vec<u8>, std::io::Error> {
    let dot_graph = generate_dot_with_clusters(graph, opts);
    exec(
        dot_graph,
        &mut PrinterContext::default(),
        vec![CommandArg::Format(Format::Svg)],
    )
}

pub fn generate_dot_with_clusters(
    graph: &OpenHypergraph<TypeExpr, CommandEdge>,
    opts: &Options<TypeExpr, CommandEdge>,
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

fn extend_graph(
    dot_graph: &mut Graph,
    graph: &OpenHypergraph<TypeExpr, CommandEdge>,
    opts: &Options<TypeExpr, CommandEdge>,
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

fn edge_label(edge: &CommandEdge, opts: &Options<TypeExpr, CommandEdge>) -> String {
    match edge {
        CommandEdge::Atom(_) => (opts.edge_label)(edge),
        CommandEdge::Convolution(_) => "Convolution".to_string(),
        CommandEdge::Kleene(_) => "Kleene".to_string(),
    }
}

fn generate_node_stmts(
    graph: &OpenHypergraph<TypeExpr, CommandEdge>,
    opts: &Options<TypeExpr, CommandEdge>,
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

fn generate_edge_stmts(
    graph: &OpenHypergraph<TypeExpr, CommandEdge>,
    opts: &Options<TypeExpr, CommandEdge>,
    prefix: &str,
) -> Vec<Stmt> {
    let mut stmts = Vec::new();
    for i in 0..graph.hypergraph.edges.len() {
        let hyperedge = &graph.hypergraph.adjacency[i];
        match &graph.hypergraph.edges[i] {
            CommandEdge::Atom(_) => {
                let label = edge_label(&graph.hypergraph.edges[i], opts);
                let label = escape_dot_label(&label);

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

                stmts.push(Stmt::Node(Node {
                    id: NodeId(Id::Plain(format!("{}e_{}", prefix, i)), None),
                    attributes: vec![
                        Attribute(Id::Plain(String::from("label")), Id::Plain(record_label)),
                        Attribute(
                            Id::Plain(String::from("shape")),
                            Id::Plain(String::from("record")),
                        ),
                    ],
                }));
            }
            CommandEdge::Convolution(children) => {
                for (src_idx, &node_id) in hyperedge.sources.iter().enumerate() {
                    let label = (opts.node_label)(&graph.hypergraph.nodes[node_id.0]);
                    let label = escape_dot_label(&label);
                    stmts.push(Stmt::Node(Node {
                        id: NodeId(Id::Plain(format!("{}e_{}_split_n_{}", prefix, i, src_idx)), None),
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
                for (tgt_idx, &node_id) in hyperedge.targets.iter().enumerate() {
                    let label = (opts.node_label)(&graph.hypergraph.nodes[node_id.0]);
                    let label = escape_dot_label(&label);
                    stmts.push(Stmt::Node(Node {
                        id: NodeId(Id::Plain(format!("{}e_{}_join_n_{}", prefix, i, tgt_idx)), None),
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
            }
            CommandEdge::Kleene(_) => {
                for (src_idx, &node_id) in hyperedge.sources.iter().enumerate() {
                    let label = (opts.node_label)(&graph.hypergraph.nodes[node_id.0]);
                    let label = escape_dot_label(&label);
                    stmts.push(Stmt::Node(Node {
                        id: NodeId(Id::Plain(format!("{}e_{}_split_n_{}", prefix, i, src_idx)), None),
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
                for (tgt_idx, &node_id) in hyperedge.targets.iter().enumerate() {
                    let label = (opts.node_label)(&graph.hypergraph.nodes[node_id.0]);
                    let label = escape_dot_label(&label);
                    stmts.push(Stmt::Node(Node {
                        id: NodeId(Id::Plain(format!("{}e_{}_join_n_{}", prefix, i, tgt_idx)), None),
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
            }
        }
    }
    stmts
}

fn generate_connection_stmts(
    graph: &OpenHypergraph<TypeExpr, CommandEdge>,
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

fn generate_interface_stmts(
    graph: &OpenHypergraph<TypeExpr, CommandEdge>,
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

fn generate_quotient_stmts(
    graph: &OpenHypergraph<TypeExpr, CommandEdge>,
    prefix: &str,
) -> Vec<Stmt> {
    let mut stmts = Vec::new();
    let (lefts, rights) = &graph.hypergraph.quotient;
    let mut unified_nodes = std::collections::HashMap::new();

    for (left, right) in lefts.iter().zip(rights.iter()) {
        let left_idx = left.0;
        let right_idx = right.0;
        let pair_key = if left_idx < right_idx {
            (left_idx, right_idx)
        } else {
            (right_idx, left_idx)
        };

        if unified_nodes.insert(pair_key, true).is_none() {
            let edge = Edge {
                ty: EdgeTy::Pair(
                    Vertex::N(NodeId(Id::Plain(format!("{}n_{}", prefix, left_idx)), None)),
                    Vertex::N(NodeId(
                        Id::Plain(format!("{}n_{}", prefix, right_idx)),
                        None,
                    )),
                ),
                attributes: vec![
                    Attribute(
                        Id::Plain(String::from("style")),
                        Id::Plain(String::from("dotted")),
                    ),
                    Attribute(
                        Id::Plain(String::from("dir")),
                        Id::Plain(String::from("none")),
                    ),
                ],
            };
            stmts.push(Stmt::Edge(edge));
        }
    }

    stmts
}

fn generate_edge_clusters(
    graph: &OpenHypergraph<TypeExpr, CommandEdge>,
    opts: &Options<TypeExpr, CommandEdge>,
    prefix: &str,
) -> Vec<Stmt> {
    let mut stmts = Vec::new();
    let theme = &opts.theme;

    for (edge_idx, edge_label) in graph.hypergraph.edges.iter().enumerate() {
        match edge_label {
            CommandEdge::Atom(_) => continue,
            CommandEdge::Convolution(children) => {
                let cluster_id = format!("cluster_{}e_{}", prefix, edge_idx);
                let mut cluster = Subgraph {
                    id: Id::Plain(cluster_id),
                    stmts: vec![
                        Stmt::Attribute(Attribute(
                            Id::Plain(String::from("label")),
                            Id::Plain(String::from("\"Convolution\"")),
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

                let parent_sources = graph.hypergraph.adjacency[edge_idx]
                    .sources
                    .iter()
                    .map(|id| graph.hypergraph.nodes[id.0].clone())
                    .collect::<Vec<_>>();
                let parent_targets = graph.hypergraph.adjacency[edge_idx]
                    .targets
                    .iter()
                    .map(|id| graph.hypergraph.nodes[id.0].clone())
                    .collect::<Vec<_>>();
                for (j, &node_id) in graph.hypergraph.adjacency[edge_idx]
                    .sources
                    .iter()
                    .enumerate()
                {
                    let edge = Edge {
                        ty: EdgeTy::Pair(
                            Vertex::N(NodeId(
                                Id::Plain(format!("{}n_{}", prefix, node_id.0)),
                                None,
                            )),
                            Vertex::N(NodeId(
                                Id::Plain(format!("{}e_{}_split_n_{}", prefix, edge_idx, j)),
                                None,
                            )),
                        ),
                        attributes: vec![],
                    };
                    stmts.push(Stmt::Edge(edge));
                }
                for (j, &node_id) in graph.hypergraph.adjacency[edge_idx]
                    .targets
                    .iter()
                    .enumerate()
                {
                    let edge = Edge {
                        ty: EdgeTy::Pair(
                            Vertex::N(NodeId(
                                Id::Plain(format!("{}e_{}_join_n_{}", prefix, edge_idx, j)),
                                None,
                            )),
                            Vertex::N(NodeId(
                                Id::Plain(format!("{}n_{}", prefix, node_id.0)),
                                None,
                            )),
                        ),
                        attributes: vec![],
                    };
                    stmts.push(Stmt::Edge(edge));
                }

                for (child_idx, child) in children.iter().enumerate() {
                    let child = strictify(child);
                    let child = normalize_interface_labels(child, &parent_sources, &parent_targets);
                    let child_prefix = format!("{}e_{}_c{}_", prefix, edge_idx, child_idx);
                    for (j, &child_source) in child.sources.iter().enumerate() {
                        let edge = Edge {
                            ty: EdgeTy::Pair(
                                Vertex::N(NodeId(
                                    Id::Plain(format!("{}e_{}_split_n_{}", prefix, edge_idx, j)),
                                    None,
                                )),
                                Vertex::N(NodeId(
                                    Id::Plain(format!("{}n_{}", child_prefix, child_source.0)),
                                    None,
                                )),
                            ),
                            attributes: vec![
                                Attribute(
                                    Id::Plain(String::from("style")),
                                    Id::Plain(String::from("dotted")),
                                ),
                                Attribute(
                                    Id::Plain(String::from("dir")),
                                    Id::Plain(String::from("none")),
                                ),
                            ],
                        };
                        stmts.push(Stmt::Edge(edge));
                    }
                    for (j, &child_target) in child.targets.iter().enumerate() {
                        let edge = Edge {
                            ty: EdgeTy::Pair(
                                Vertex::N(NodeId(
                                    Id::Plain(format!("{}n_{}", child_prefix, child_target.0)),
                                    None,
                                )),
                                Vertex::N(NodeId(
                                    Id::Plain(format!("{}e_{}_join_n_{}", prefix, edge_idx, j)),
                                    None,
                                )),
                            ),
                            attributes: vec![
                                Attribute(
                                    Id::Plain(String::from("style")),
                                    Id::Plain(String::from("dotted")),
                                ),
                                Attribute(
                                    Id::Plain(String::from("dir")),
                                    Id::Plain(String::from("none")),
                                ),
                            ],
                        };
                        stmts.push(Stmt::Edge(edge));
                    }
                    let mut child_cluster = Subgraph {
                        id: Id::Plain(format!("cluster_{}", child_prefix)),
                        stmts: vec![Stmt::Attribute(Attribute(
                            Id::Plain(String::from("label")),
                            Id::Plain(format!("\"alt {}\"", child_idx)),
                        ))],
                    };
                    for stmt in generate_node_stmts(&child, opts, &child_prefix) {
                        child_cluster.add_stmt(stmt);
                    }
                    for stmt in generate_edge_stmts(&child, opts, &child_prefix) {
                        child_cluster.add_stmt(stmt);
                    }
                    for stmt in generate_interface_stmts(&child, &child_prefix) {
                        child_cluster.add_stmt(stmt);
                    }
                    for stmt in generate_connection_stmts(&child, &child_prefix) {
                        child_cluster.add_stmt(stmt);
                    }
                    for stmt in generate_quotient_stmts(&child, &child_prefix) {
                        child_cluster.add_stmt(stmt);
                    }
                    cluster.add_stmt(Stmt::Subgraph(child_cluster));
                }

                stmts.push(Stmt::Subgraph(cluster));
            }
            CommandEdge::Kleene(child) => {
                let cluster_id = format!("cluster_{}e_{}", prefix, edge_idx);
                let mut cluster = Subgraph {
                    id: Id::Plain(cluster_id),
                    stmts: vec![
                        Stmt::Attribute(Attribute(
                            Id::Plain(String::from("label")),
                            Id::Plain(String::from("\"Kleene\"")),
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

                let parent_sources = graph.hypergraph.adjacency[edge_idx]
                    .sources
                    .iter()
                    .map(|id| graph.hypergraph.nodes[id.0].clone())
                    .collect::<Vec<_>>();
                let parent_targets = graph.hypergraph.adjacency[edge_idx]
                    .targets
                    .iter()
                    .map(|id| graph.hypergraph.nodes[id.0].clone())
                    .collect::<Vec<_>>();
                for (j, &node_id) in graph.hypergraph.adjacency[edge_idx]
                    .sources
                    .iter()
                    .enumerate()
                {
                    let edge = Edge {
                        ty: EdgeTy::Pair(
                            Vertex::N(NodeId(
                                Id::Plain(format!("{}n_{}", prefix, node_id.0)),
                                None,
                            )),
                            Vertex::N(NodeId(
                                Id::Plain(format!("{}e_{}_split_n_{}", prefix, edge_idx, j)),
                                None,
                            )),
                        ),
                        attributes: vec![],
                    };
                    stmts.push(Stmt::Edge(edge));
                }
                for (j, &node_id) in graph.hypergraph.adjacency[edge_idx]
                    .targets
                    .iter()
                    .enumerate()
                {
                    let edge = Edge {
                        ty: EdgeTy::Pair(
                            Vertex::N(NodeId(
                                Id::Plain(format!("{}e_{}_join_n_{}", prefix, edge_idx, j)),
                                None,
                            )),
                            Vertex::N(NodeId(
                                Id::Plain(format!("{}n_{}", prefix, node_id.0)),
                                None,
                            )),
                        ),
                        attributes: vec![],
                    };
                    stmts.push(Stmt::Edge(edge));
                }
                let child = strictify(child);
                let child = normalize_interface_labels(child, &parent_sources, &parent_targets);
                let child_prefix = format!("{}e_{}_k_", prefix, edge_idx);
                for (j, &child_source) in child.sources.iter().enumerate() {
                    let edge = Edge {
                        ty: EdgeTy::Pair(
                            Vertex::N(NodeId(
                                Id::Plain(format!("{}e_{}_split_n_{}", prefix, edge_idx, j)),
                                None,
                            )),
                            Vertex::N(NodeId(
                                Id::Plain(format!("{}n_{}", child_prefix, child_source.0)),
                                None,
                            )),
                        ),
                        attributes: vec![
                            Attribute(
                                Id::Plain(String::from("style")),
                                Id::Plain(String::from("dotted")),
                            ),
                            Attribute(
                                Id::Plain(String::from("dir")),
                                Id::Plain(String::from("none")),
                            ),
                        ],
                    };
                    stmts.push(Stmt::Edge(edge));
                }
                for (j, &child_target) in child.targets.iter().enumerate() {
                    let edge = Edge {
                        ty: EdgeTy::Pair(
                            Vertex::N(NodeId(
                                Id::Plain(format!("{}n_{}", child_prefix, child_target.0)),
                                None,
                            )),
                            Vertex::N(NodeId(
                                Id::Plain(format!("{}e_{}_join_n_{}", prefix, edge_idx, j)),
                                None,
                            )),
                        ),
                        attributes: vec![
                            Attribute(
                                Id::Plain(String::from("style")),
                                Id::Plain(String::from("dotted")),
                            ),
                            Attribute(
                                Id::Plain(String::from("dir")),
                                Id::Plain(String::from("none")),
                            ),
                        ],
                    };
                    stmts.push(Stmt::Edge(edge));
                }
                let mut child_cluster = Subgraph {
                    id: Id::Plain(format!("cluster_{}", child_prefix)),
                    stmts: vec![Stmt::Attribute(Attribute(
                        Id::Plain(String::from("label")),
                        Id::Plain(String::from("\"body\"")),
                    ))],
                };
                for stmt in generate_node_stmts(&child, opts, &child_prefix) {
                    child_cluster.add_stmt(stmt);
                }
                for stmt in generate_edge_stmts(&child, opts, &child_prefix) {
                    child_cluster.add_stmt(stmt);
                }
                for stmt in generate_interface_stmts(&child, &child_prefix) {
                    child_cluster.add_stmt(stmt);
                }
                for stmt in generate_connection_stmts(&child, &child_prefix) {
                    child_cluster.add_stmt(stmt);
                }
                for stmt in generate_quotient_stmts(&child, &child_prefix) {
                    child_cluster.add_stmt(stmt);
                }
                cluster.add_stmt(Stmt::Subgraph(child_cluster));

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

fn strictify(
    graph: &OpenHypergraph<TypeExpr, CommandEdge>,
) -> OpenHypergraph<TypeExpr, CommandEdge> {
    let resolved = match solve_hypergraph_types(graph) {
        Ok(subst) => apply_substitution(graph, &subst),
        Err(_) => graph.clone(),
    };
    let strict = resolved.to_strict();
    OpenHypergraph::from_strict(strict)
}

fn normalize_interface_labels(
    mut graph: OpenHypergraph<TypeExpr, CommandEdge>,
    parent_sources: &[TypeExpr],
    parent_targets: &[TypeExpr],
) -> OpenHypergraph<TypeExpr, CommandEdge> {
    if graph.sources.len() != parent_sources.len() || graph.targets.len() != parent_targets.len() {
        panic!(
            "subgraph interface mismatch: expected {} -> {}, got {} -> {}",
            parent_sources.len(),
            parent_targets.len(),
            graph.sources.len(),
            graph.targets.len()
        );
    }

    for (node_id, ty) in graph.sources.iter().zip(parent_sources.iter()) {
        graph.hypergraph.nodes[node_id.0] = ty.clone();
    }
    for (node_id, ty) in graph.targets.iter().zip(parent_targets.iter()) {
        graph.hypergraph.nodes[node_id.0] = ty.clone();
    }
    graph
}
