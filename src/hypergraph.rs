use open_hypergraphs::category::Arrow;
use open_hypergraphs::lax::{NodeId, OpenHypergraph};

use crate::types::TypeExpr;
use crate::typing::{ContextSnapshot, DeductionTree, ExprForm};

pub fn from_deduction_tree(tree: &DeductionTree) -> OpenHypergraph<TypeExpr, String> {
    match tree.form() {
        ExprForm::Var(name) => var_graph(name, tree.judgment().context(), tree.judgment().ty()),
        ExprForm::Const(label) => constant_graph(label, tree.judgment().ty()),
        ExprForm::UnaryOp(op) => unary_graph(op, tree),
        ExprForm::BinOp(op) => binop_graph(op, tree),
        ExprForm::Call(name) => call_graph(name, tree),
    }
}

pub fn format_hypergraph(graph: &OpenHypergraph<TypeExpr, String>) -> String {
    let mut out = String::new();
    out.push_str("OpenHypergraph\n");

    out.push_str("  sources: [");
    for (idx, node) in graph.sources.iter().enumerate() {
        if idx > 0 {
            out.push_str(", ");
        }
        out.push_str(&format!("n{}", node.0));
    }
    out.push_str("]\n");

    out.push_str("  targets: [");
    for (idx, node) in graph.targets.iter().enumerate() {
        if idx > 0 {
            out.push_str(", ");
        }
        out.push_str(&format!("n{}", node.0));
    }
    out.push_str("]\n");

    out.push_str("  nodes:\n");
    for (idx, label) in graph.hypergraph.nodes.iter().enumerate() {
        out.push_str(&format!("    n{}: {}\n", idx, label));
    }

    out.push_str("  edges:\n");
    for (idx, (label, interface)) in graph
        .hypergraph
        .edges
        .iter()
        .zip(graph.hypergraph.adjacency.iter())
        .enumerate()
    {
        let sources = format_nodes(&interface.sources);
        let targets = format_nodes(&interface.targets);
        out.push_str(&format!(
            "    e{}: {} ({} -> {})\n",
            idx, label, sources, targets
        ));
    }

    if !graph.hypergraph.quotient.0.is_empty() {
        out.push_str("  quotient:\n");
        for (from, to) in graph
            .hypergraph
            .quotient
            .0
            .iter()
            .zip(graph.hypergraph.quotient.1.iter())
        {
            let from_label = &graph.hypergraph.nodes[from.0];
            let to_label = &graph.hypergraph.nodes[to.0];
            out.push_str(&format!(
                "    n{}:{} ~ n{}:{}\n",
                from.0, from_label, to.0, to_label
            ));
        }
    }

    out
}

fn var_graph(
    name: &str,
    context: &ContextSnapshot,
    ty: &TypeExpr,
) -> OpenHypergraph<TypeExpr, String> {
    let entries = context.entries();
    let index = entries
        .iter()
        .position(|(var, _)| var == name)
        .unwrap_or_else(|| panic!("variable `{}` not found in context", name));

    let left = discard(&entries[..index]);
    let id = OpenHypergraph::identity(vec![ty.clone()]);
    let right = discard(&entries[index + 1..]);

    tensor_many(vec![left, id, right])
}

fn constant_graph(label: &str, ty: &TypeExpr) -> OpenHypergraph<TypeExpr, String> {
    OpenHypergraph::singleton(label.to_string(), vec![], vec![ty.clone()])
}

fn unary_graph(op: &str, tree: &DeductionTree) -> OpenHypergraph<TypeExpr, String> {
    let child = tree
        .children()
        .get(0)
        .unwrap_or_else(|| panic!("unary node missing child"));
    let child_graph = from_deduction_tree(child);
    let source_type = vec![child.judgment().ty().clone()];
    let target_type = vec![tree.judgment().ty().clone()];
    let op_graph = OpenHypergraph::singleton(op.to_string(), source_type, target_type);

    compose_lax(&child_graph, &op_graph)
}

fn binop_graph(op: &str, tree: &DeductionTree) -> OpenHypergraph<TypeExpr, String> {
    if tree.children().len() != 2 {
        panic!("binary node expects 2 children");
    }

    let left = from_deduction_tree(&tree.children()[0]);
    let right = from_deduction_tree(&tree.children()[1]);
    let tensor = tensor_many(vec![left, right]);

    let source_type = tree
        .children()
        .iter()
        .map(|child| child.judgment().ty().clone())
        .collect();
    let target_type = vec![tree.judgment().ty().clone()];
    let op_graph = OpenHypergraph::singleton(op.to_string(), source_type, target_type);

    compose_lax(&tensor, &op_graph)
}

fn call_graph(name: &str, tree: &DeductionTree) -> OpenHypergraph<TypeExpr, String> {
    if tree.children().len() != 1 {
        panic!("function `{}` expects 1 argument", name);
    }
    let child = &tree.children()[0];
    let child_graph = from_deduction_tree(child);

    let source_type = vec![child.judgment().ty().clone()];
    let target_type = vec![tree.judgment().ty().clone()];
    let op_graph = OpenHypergraph::singleton(name.to_string(), source_type, target_type);

    compose_lax(&child_graph, &op_graph)
}

fn discard(entries: &[(String, TypeExpr)]) -> OpenHypergraph<TypeExpr, String> {
    let mut graph = OpenHypergraph::empty();
    let sources = entries
        .iter()
        .map(|(_, ty)| graph.new_node(ty.clone()))
        .collect();
    graph.sources = sources;
    graph.targets = Vec::new();
    graph
}

fn tensor_many(
    mut graphs: Vec<OpenHypergraph<TypeExpr, String>>,
) -> OpenHypergraph<TypeExpr, String> {
    if graphs.is_empty() {
        return OpenHypergraph::empty();
    }

    let mut acc = graphs.remove(0);
    for graph in graphs.iter() {
        acc = &acc | graph;
    }
    acc
}

fn compose_lax(
    lhs: &OpenHypergraph<TypeExpr, String>,
    rhs: &OpenHypergraph<TypeExpr, String>,
) -> OpenHypergraph<TypeExpr, String> {
    (lhs >> rhs).unwrap_or_else(|| {
        panic!(
            "lax composition failed: {:?} -> {:?} cannot compose with {:?} -> {:?}",
            lhs.source(),
            lhs.target(),
            rhs.source(),
            rhs.target()
        )
    })
}

fn format_nodes(nodes: &[NodeId]) -> String {
    let mut out = String::from("[");
    for (idx, node) in nodes.iter().enumerate() {
        if idx > 0 {
            out.push_str(", ");
        }
        out.push_str(&format!("n{}", node.0));
    }
    out.push(']');
    out
}
