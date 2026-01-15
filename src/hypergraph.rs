use std::collections::HashMap;

use open_hypergraphs::category::Arrow;
use open_hypergraphs::lax::{Monoidal, NodeId, OpenHypergraph};

use crate::types::TypeExpr;
use crate::typing::{ContextSnapshot, DeductionTree, ExprForm};

pub fn from_deduction_tree(tree: &DeductionTree) -> OpenHypergraph<TypeExpr, String> {
    match tree.form() {
        ExprForm::Var(name) => {
            let ty = lookup_var_type(name, tree.judgment().context());
            var_graph(&ty)
        }
        ExprForm::Const(label) => constant_graph(label, tree.judgment().ty()),
        ExprForm::UnaryOp(op) => unary_graph(op, tree),
        ExprForm::BinOp(op) => binop_graph(op, tree),
        ExprForm::Call(name) => call_graph(name, tree),
        ExprForm::BoolOp(op) => boolop_graph(op, tree),
        ExprForm::Compare(op) => compare_graph(op, tree),
    }
}

pub fn from_deduction_tree_with_context(
    tree: &DeductionTree,
    context_entries: &[(String, TypeExpr)],
) -> OpenHypergraph<TypeExpr, String> {
    let input_vars = expr_input_vars(tree);
    let wiring = wiring_for_expression(context_entries, &input_vars);
    let expr_graph = from_deduction_tree(tree);
    compose_lax_unchecked(&wiring, &expr_graph)
}

pub fn format_hypergraph<A: std::fmt::Display>(
    graph: &OpenHypergraph<TypeExpr, A>,
) -> String {
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

fn var_graph(ty: &TypeExpr) -> OpenHypergraph<TypeExpr, String> {
    OpenHypergraph::identity(vec![ty.clone()])
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

    compose_lax_unchecked(&child_graph, &op_graph)
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

    compose_lax_unchecked(&tensor, &op_graph)
}

fn call_graph(name: &str, tree: &DeductionTree) -> OpenHypergraph<TypeExpr, String> {
    let mut graphs = Vec::with_capacity(tree.children().len());
    for child in tree.children() {
        graphs.push(from_deduction_tree(child));
    }
    let tensor = if graphs.is_empty() {
        OpenHypergraph::empty()
    } else {
        tensor_many(graphs)
    };

    let source_type = tree
        .children()
        .iter()
        .map(|child| child.judgment().ty().clone())
        .collect();
    let target_type = vec![tree.judgment().ty().clone()];
    let op_graph = OpenHypergraph::singleton(name.to_string(), source_type, target_type);

    compose_lax_unchecked(&tensor, &op_graph)
}

fn boolop_graph(op: &str, tree: &DeductionTree) -> OpenHypergraph<TypeExpr, String> {
    if tree.children().is_empty() {
        panic!("boolean operation expects at least 1 operand");
    }

    let mut graphs = Vec::with_capacity(tree.children().len());
    for child in tree.children() {
        graphs.push(from_deduction_tree(child));
    }
    let tensor = tensor_many(graphs);

    let source_type = tree
        .children()
        .iter()
        .map(|child| child.judgment().ty().clone())
        .collect();
    let target_type = vec![tree.judgment().ty().clone()];
    let op_graph = OpenHypergraph::singleton(op.to_string(), source_type, target_type);

    compose_lax_unchecked(&tensor, &op_graph)
}

fn compare_graph(op: &str, tree: &DeductionTree) -> OpenHypergraph<TypeExpr, String> {
    if tree.children().len() != 2 {
        panic!("comparison expects 2 operands");
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

    compose_lax_unchecked(&tensor, &op_graph)
}
fn expr_input_vars(tree: &DeductionTree) -> Vec<String> {
    match tree.form() {
        ExprForm::Var(name) => vec![name.clone()],
        ExprForm::Const(_) => Vec::new(),
        ExprForm::UnaryOp(_) | ExprForm::Call(_) => tree
            .children()
            .get(0)
            .map(expr_input_vars)
            .unwrap_or_default(),
        ExprForm::BinOp(_) | ExprForm::Compare(_) => {
            if tree.children().len() != 2 {
                return Vec::new();
            }
            let mut left = expr_input_vars(&tree.children()[0]);
            let mut right = expr_input_vars(&tree.children()[1]);
            left.append(&mut right);
            left
        }
        ExprForm::BoolOp(_) => {
            let mut vars = Vec::new();
            for child in tree.children() {
                vars.extend(expr_input_vars(child));
            }
            vars
        }
    }
}

fn wiring_for_expression(
    context_entries: &[(String, TypeExpr)],
    input_vars: &[String],
) -> OpenHypergraph<TypeExpr, String> {
    let mut graph = OpenHypergraph::empty();
    let mut input_nodes = Vec::with_capacity(context_entries.len());
    let mut type_map = HashMap::new();
    for (name, ty) in context_entries {
        input_nodes.push(graph.new_node(ty.clone()));
        type_map.insert(name.clone(), ty.clone());
    }
    graph.sources = input_nodes.clone();

    let mut outputs = Vec::with_capacity(input_vars.len());
    for name in input_vars {
        let index = context_entries
            .iter()
            .position(|(var, _)| var == name)
            .unwrap_or_else(|| panic!("variable `{}` not in context", name));
        outputs.push(input_nodes[index]);
    }
    graph.targets = outputs;

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

fn compose_lax_unchecked(
    lhs: &OpenHypergraph<TypeExpr, String>,
    rhs: &OpenHypergraph<TypeExpr, String>,
) -> OpenHypergraph<TypeExpr, String> {
    if lhs.targets.len() != rhs.sources.len() {
        panic!(
            "unchecked composition requires same arity, got {} vs {}",
            lhs.targets.len(),
            rhs.sources.len()
        );
    }

    let n = lhs.hypergraph.nodes.len();
    let mut composed = lhs.tensor(rhs);

    for (u, v) in lhs.targets.iter().zip(rhs.sources.iter()) {
        composed.unify(*u, open_hypergraphs::lax::NodeId(v.0 + n));
    }

    composed.sources = composed.sources[..lhs.sources.len()].to_vec();
    composed.targets = composed.targets[lhs.targets.len()..].to_vec();
    composed
}

fn lookup_var_type(name: &str, context: &ContextSnapshot) -> TypeExpr {
    context
        .entries()
        .iter()
        .find(|(var, _)| var == name)
        .map(|(_, ty)| ty.clone())
        .unwrap_or_else(|| panic!("variable `{}` not found in context", name))
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
