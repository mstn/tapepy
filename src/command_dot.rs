use std::fmt;

use graphviz_rust::dot_structures::{
    Attribute, Edge, EdgeTy, Graph, Id, Node, NodeId, Port, Stmt, Vertex,
};
use open_hypergraphs::lax::OpenHypergraph;
use open_hypergraphs_dot::Options;

use crate::tape_language::monomial_tape::TensorKind;
use crate::tape_language::{MonomialHyperNode, MonomialTapeEdge};
use crate::types::TypeExpr;

#[derive(Debug, Clone, PartialEq)]
pub enum CommandEdge {
    Atom(String),
}

impl fmt::Display for CommandEdge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandEdge::Atom(label) => write!(f, "{}", label),
        }
    }
}

pub fn generate_dot_with_monomial_tape_clusters<G: Clone + fmt::Display>(
    graph: &OpenHypergraph<MonomialHyperNode<TypeExpr>, MonomialTapeEdge<TypeExpr, G>>,
    opts: &Options<MonomialHyperNode<TypeExpr>, CommandEdge>,
) -> Graph {
    let theme = &opts.theme;
    let graph = graph
        .clone()
        .map_edges(|edge| CommandEdge::Atom(edge.to_string()));

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

    for stmt in generate_monomial_node_stmts(&graph, opts) {
        dot_graph.add_stmt(stmt);
    }
    for stmt in generate_edge_stmts(&graph, opts) {
        dot_graph.add_stmt(stmt);
    }
    for stmt in generate_interface_stmts(&graph) {
        dot_graph.add_stmt(stmt);
    }
    for stmt in generate_connection_stmts(&graph) {
        dot_graph.add_stmt(stmt);
    }
    for stmt in generate_quotient_stmts(&graph) {
        dot_graph.add_stmt(stmt);
    }

    dot_graph
}

fn generate_monomial_node_stmts(
    graph: &OpenHypergraph<MonomialHyperNode<TypeExpr>, CommandEdge>,
    opts: &Options<MonomialHyperNode<TypeExpr>, CommandEdge>,
) -> Vec<Stmt> {
    let mut stmts = Vec::new();
    for (i, node) in graph.hypergraph.nodes.iter().enumerate() {
        let label = escape_dot_label(&(opts.node_label)(node));
        let (fillcolor, fontcolor) = match node.tensor_kind {
            TensorKind::Multiplicative => ("#000000", "#ffffff"),
            TensorKind::Additive => ("#ffffff", "#000000"),
        };
        stmts.push(Stmt::Node(Node {
            id: NodeId(Id::Plain(format!("n_{}", i)), None),
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

fn generate_edge_stmts(
    graph: &OpenHypergraph<MonomialHyperNode<TypeExpr>, CommandEdge>,
    opts: &Options<MonomialHyperNode<TypeExpr>, CommandEdge>,
) -> Vec<Stmt> {
    let mut stmts = Vec::new();
    for i in 0..graph.hypergraph.edges.len() {
        let hyperedge = &graph.hypergraph.adjacency[i];
        let raw_label = (opts.edge_label)(&graph.hypergraph.edges[i]);
        let label = escape_dot_label(&raw_label);

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
            id: NodeId(Id::Plain(format!("e_{}", i)), None),
            attributes: vec![
                Attribute(Id::Plain(String::from("label")), Id::Plain(record_label)),
                Attribute(
                    Id::Plain(String::from("shape")),
                    Id::Plain(String::from("record")),
                ),
            ],
        }));
    }
    stmts
}

fn generate_interface_stmts(
    graph: &OpenHypergraph<MonomialHyperNode<TypeExpr>, CommandEdge>,
) -> Vec<Stmt> {
    let mut stmts = Vec::new();
    for (idx, node) in graph.sources.iter().enumerate() {
        stmts.push(Stmt::Node(Node {
            id: NodeId(Id::Plain(format!("in_{}", idx)), None),
            attributes: vec![Attribute(
                Id::Plain(String::from("shape")),
                Id::Plain(String::from("none")),
            )],
        }));
        stmts.push(Stmt::Edge(Edge {
            ty: EdgeTy::Pair(
                Vertex::N(NodeId(Id::Plain(format!("in_{}", idx)), None)),
                Vertex::N(NodeId(Id::Plain(format!("n_{}", node.0)), None)),
            ),
            attributes: vec![],
        }));
    }
    for (idx, node) in graph.targets.iter().enumerate() {
        stmts.push(Stmt::Node(Node {
            id: NodeId(Id::Plain(format!("out_{}", idx)), None),
            attributes: vec![Attribute(
                Id::Plain(String::from("shape")),
                Id::Plain(String::from("none")),
            )],
        }));
        stmts.push(Stmt::Edge(Edge {
            ty: EdgeTy::Pair(
                Vertex::N(NodeId(Id::Plain(format!("n_{}", node.0)), None)),
                Vertex::N(NodeId(Id::Plain(format!("out_{}", idx)), None)),
            ),
            attributes: vec![],
        }));
    }
    stmts
}

fn generate_connection_stmts(
    graph: &OpenHypergraph<MonomialHyperNode<TypeExpr>, CommandEdge>,
) -> Vec<Stmt> {
    let mut stmts = Vec::new();
    for (i, hyperedge) in graph.hypergraph.adjacency.iter().enumerate() {
        for (j, &node_id) in hyperedge.sources.iter().enumerate() {
            stmts.push(Stmt::Edge(Edge {
                ty: EdgeTy::Pair(
                    Vertex::N(NodeId(Id::Plain(format!("n_{}", node_id.0)), None)),
                    Vertex::N(NodeId(
                        Id::Plain(format!("e_{}", i)),
                        Some(Port(None, Some(format!("s_{}", j)))),
                    )),
                ),
                attributes: vec![],
            }));
        }
        for (j, &node_id) in hyperedge.targets.iter().enumerate() {
            stmts.push(Stmt::Edge(Edge {
                ty: EdgeTy::Pair(
                    Vertex::N(NodeId(
                        Id::Plain(format!("e_{}", i)),
                        Some(Port(None, Some(format!("t_{}", j)))),
                    )),
                    Vertex::N(NodeId(Id::Plain(format!("n_{}", node_id.0)), None)),
                ),
                attributes: vec![],
            }));
        }
    }
    stmts
}

fn generate_quotient_stmts(
    graph: &OpenHypergraph<MonomialHyperNode<TypeExpr>, CommandEdge>,
) -> Vec<Stmt> {
    let mut stmts = Vec::new();
    for (left, right) in graph
        .hypergraph
        .quotient
        .0
        .iter()
        .zip(graph.hypergraph.quotient.1.iter())
    {
        stmts.push(Stmt::Edge(Edge {
            ty: EdgeTy::Pair(
                Vertex::N(NodeId(Id::Plain(format!("n_{}", left.0)), None)),
                Vertex::N(NodeId(Id::Plain(format!("n_{}", right.0)), None)),
            ),
            attributes: vec![Attribute(
                Id::Plain(String::from("style")),
                Id::Plain(String::from("dotted")),
            )],
        }));
    }
    stmts
}

fn escape_dot_label(label: &str) -> String {
    let mut out = String::with_capacity(label.len());
    for ch in label.chars() {
        match ch {
            '\\' | '"' | '{' | '}' | '|' | '<' | '>' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}
