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
        CommandForm::If => if_graph(tree),
        CommandForm::While => {
            panic!("while hypergraphs not implemented yet")
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

fn if_graph(tree: &CommandDerivationTree) -> OpenHypergraph<TypeExpr, CommandEdge> {
    if tree.children().len() != 3 {
        panic!("if expects predicate, then, else");
    }
    let (pred_tree, then_tree, else_tree) = match (
        &tree.children()[0],
        &tree.children()[1],
        &tree.children()[2],
    ) {
        (
            CommandChild::Predicate(pred),
            CommandChild::Command(then_cmd),
            CommandChild::Command(else_cmd),
        ) => (pred, then_cmd, else_cmd),
        _ => panic!("if expects predicate, then, else"),
    };

    let context_entries = tree.judgment().context().entries();
    let context_types: Vec<TypeExpr> = context_entries
        .iter()
        .map(|(_, ty)| ty.clone())
        .collect();

    let pred_graph = predicate_graph(pred_tree, context_entries);
    let neg_pred_graph = negate_predicate_graph(pred_graph.clone());

    let then_graph = from_command_tree(then_tree);
    let else_graph = from_command_tree(else_tree);

    let then_guard =
        compose_lax_unchecked(&lift_predicate_graph(pred_graph, &context_types), &then_graph);
    let else_guard =
        compose_lax_unchecked(&lift_predicate_graph(neg_pred_graph, &context_types), &else_graph);

    OpenHypergraph::singleton(
        CommandEdge::Convolution(vec![then_guard, else_guard]),
        context_types.clone(),
        context_types,
    )
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

fn predicate_graph(
    pred_tree: &crate::typing::DeductionTree,
    context_entries: &[(String, TypeExpr)],
) -> OpenHypergraph<TypeExpr, CommandEdge> {
    hypergraph::from_deduction_tree_with_context(pred_tree, context_entries)
        .map_edges(CommandEdge::Atom)
}

fn negate_predicate_graph(
    pred_graph: OpenHypergraph<TypeExpr, CommandEdge>,
) -> OpenHypergraph<TypeExpr, CommandEdge> {
    let not_graph = OpenHypergraph::singleton(
        CommandEdge::Atom("not".to_string()),
        vec![TypeExpr::Unit],
        vec![TypeExpr::Unit],
    );
    compose_lax_unchecked(&pred_graph, &not_graph)
}

fn lift_predicate_graph(
    pred_graph: OpenHypergraph<TypeExpr, CommandEdge>,
    context_types: &[TypeExpr],
) -> OpenHypergraph<TypeExpr, CommandEdge> {
    let copy = copy_n(context_types, 2);
    let passthrough = OpenHypergraph::identity(context_types.to_vec());
    let tensor = tensor_many(vec![passthrough, pred_graph]);
    let composed = compose_lax_unchecked(&copy, &tensor);
    let drop = drop_last_graph(context_types);
    compose_lax_unchecked(&composed, &drop)
}

fn drop_last_graph(context_types: &[TypeExpr]) -> OpenHypergraph<TypeExpr, CommandEdge> {
    let mut graph = OpenHypergraph::empty();
    let mut nodes = Vec::with_capacity(context_types.len() + 1);
    for ty in context_types {
        nodes.push(graph.new_node(ty.clone()));
    }
    nodes.push(graph.new_node(TypeExpr::Unit));
    graph.sources = nodes.clone();
    graph.targets = nodes[..context_types.len()].to_vec();
    graph
}
