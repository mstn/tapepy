use open_hypergraphs::category::Arrow;
use open_hypergraphs::lax::{Monoidal, OpenHypergraph};

use crate::command_edge::CommandEdge;
use crate::command_typing::{CommandChild, CommandDerivationTree, CommandForm};
use crate::hypergraph;
use crate::types::TypeExpr;

pub fn from_command_tree(tree: &CommandDerivationTree) -> OpenHypergraph<TypeExpr, CommandEdge> {
    match tree.form() {
        CommandForm::Abort => discard_context(tree),
        CommandForm::Skip => identity_context(tree),
        CommandForm::Assign(name) => assignment_graph(tree, name),
        CommandForm::Seq => sequence_graph(tree),
        CommandForm::If | CommandForm::While => {
            panic!("if/while hypergraphs not implemented yet")
        }
    }
}

fn discard_context(tree: &CommandDerivationTree) -> OpenHypergraph<TypeExpr, CommandEdge> {
    let mut graph = OpenHypergraph::empty();
    let sources = tree
        .judgment()
        .context()
        .entries()
        .iter()
        .map(|(_, ty)| graph.new_node(ty.clone()))
        .collect();
    graph.sources = sources;
    graph.targets = Vec::new();
    graph
}

fn identity_context(tree: &CommandDerivationTree) -> OpenHypergraph<TypeExpr, CommandEdge> {
    let types = tree
        .judgment()
        .context()
        .entries()
        .iter()
        .map(|(_, ty)| ty.clone())
        .collect();
    OpenHypergraph::identity(types)
}

fn assignment_graph(
    tree: &CommandDerivationTree,
    name: &str,
) -> OpenHypergraph<TypeExpr, CommandEdge> {
    let context_entries = tree.judgment().context().entries();
    let index = context_entries
        .iter()
        .position(|(var, _)| var == name)
        .unwrap_or_else(|| panic!("assignment target `{}` not in context", name));

    let left_types: Vec<TypeExpr> = context_entries[..index]
        .iter()
        .map(|(_, ty)| ty.clone())
        .collect();
    let right_types: Vec<TypeExpr> = context_entries[index + 1..]
        .iter()
        .map(|(_, ty)| ty.clone())
        .collect();

    let expr_tree = match tree.children().get(0) {
        Some(CommandChild::Expression(expr)) => expr,
        _ => panic!("assignment expects an expression child"),
    };

    let expr_graph = hypergraph::from_deduction_tree_with_context(expr_tree, &context_entries)
        .map_edges(CommandEdge::Atom);

    let left_id = OpenHypergraph::identity(left_types.clone());
    let right_id = OpenHypergraph::identity(right_types.clone());

    let split = split_context_for_assignment(&context_entries, index);
    let updated = tensor_many(vec![left_id, expr_graph, right_id]);
    compose_lax_unchecked(&split, &updated)
}

fn sequence_graph(tree: &CommandDerivationTree) -> OpenHypergraph<TypeExpr, CommandEdge> {
    if tree.children().len() != 2 {
        panic!("sequence expects two command children");
    }

    let left = match &tree.children()[0] {
        CommandChild::Command(cmd) => from_command_tree(cmd),
        _ => panic!("sequence expects command children"),
    };
    let right = match &tree.children()[1] {
        CommandChild::Command(cmd) => from_command_tree(cmd),
        _ => panic!("sequence expects command children"),
    };

    compose_lax_unchecked(&left, &right)
}

fn tensor_many(
    mut graphs: Vec<OpenHypergraph<TypeExpr, CommandEdge>>,
) -> OpenHypergraph<TypeExpr, CommandEdge> {
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
    lhs: &OpenHypergraph<TypeExpr, CommandEdge>,
    rhs: &OpenHypergraph<TypeExpr, CommandEdge>,
) -> OpenHypergraph<TypeExpr, CommandEdge> {
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

fn split_context_for_assignment(
    context_entries: &[(String, TypeExpr)],
    x_index: usize,
) -> OpenHypergraph<TypeExpr, CommandEdge> {
    let left_types: Vec<TypeExpr> = context_entries[..x_index]
        .iter()
        .map(|(_, ty)| ty.clone())
        .collect();
    let right_types: Vec<TypeExpr> = context_entries[x_index + 1..]
        .iter()
        .map(|(_, ty)| ty.clone())
        .collect();
    let x_type = context_entries[x_index].1.clone();

    let left_copy = copy_n(&left_types, 2);
    let right_copy = copy_n(&right_types, 2);
    let x_id = OpenHypergraph::identity(vec![x_type]);

    tensor_many(vec![left_copy, x_id, right_copy])
}

fn copy_n(types: &[TypeExpr], copies: usize) -> OpenHypergraph<TypeExpr, CommandEdge> {
    let mut graph = OpenHypergraph::empty();
    let mut nodes = Vec::with_capacity(types.len());
    for ty in types {
        nodes.push(graph.new_node(ty.clone()));
    }
    graph.sources = nodes.clone();
    let mut targets = Vec::with_capacity(types.len() * copies);
    for _ in 0..copies {
        targets.extend(nodes.iter().copied());
    }
    graph.targets = targets;
    graph
}
