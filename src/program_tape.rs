use open_hypergraphs::lax::OpenHypergraph;

use crate::expression_circuit::ExprGenerator;
use crate::solver::{apply_substitution, solve_type_equations, TypeSubstitution};
use crate::tape_language::{Monomial, TapeEdge};
use crate::types::{TypeConstraint, TypeExpr};

pub fn solve_program_tape_with_subst(
    term: &OpenHypergraph<Monomial<TypeExpr>, TapeEdge<TypeExpr, ExprGenerator>>,
    constraints: &[TypeConstraint],
) -> (
    OpenHypergraph<Monomial<TypeExpr>, TapeEdge<TypeExpr, ExprGenerator>>,
    TypeSubstitution,
) {
    let (nodes, mut equations) = build_program_type_equations(term);
    equations.extend_from_slice(constraints);
    let subst = solve_type_equations(&nodes, &equations)
        .unwrap_or_else(|err| panic!("type solving failed for program tape: {}", err));
    let solved = apply_substitution_to_tape(term, &subst);
    let strict_inner = solved.map_edges(|edge| strictify_tape_edge(&edge));
    (
        OpenHypergraph::from_strict(strict_inner.to_strict()),
        subst,
    )
}

fn build_program_type_equations(
    term: &OpenHypergraph<Monomial<TypeExpr>, TapeEdge<TypeExpr, ExprGenerator>>,
) -> (Vec<TypeExpr>, Vec<TypeConstraint>) {
    let mut nodes = Vec::new();
    let mut constraints = Vec::new();

    collect_tape_constraints(term, &mut nodes, &mut constraints);

    (nodes, constraints)
}

fn apply_substitution_to_tape(
    term: &OpenHypergraph<Monomial<TypeExpr>, TapeEdge<TypeExpr, ExprGenerator>>,
    subst: &TypeSubstitution,
) -> OpenHypergraph<Monomial<TypeExpr>, TapeEdge<TypeExpr, ExprGenerator>> {
    term.clone()
        .map_nodes(|mono| apply_substitution_to_monomial(&mono, subst))
        .map_edges(|edge| apply_substitution_to_edge(&edge, subst))
}

pub(crate) fn apply_substitution_to_monomial(
    monomial: &Monomial<TypeExpr>,
    subst: &TypeSubstitution,
) -> Monomial<TypeExpr> {
    match monomial {
        Monomial::One => Monomial::one(),
        Monomial::Atom(ty) => Monomial::atom(subst.apply(ty)),
        Monomial::Product(left, right) => Monomial::product(
            apply_substitution_to_monomial(left, subst),
            apply_substitution_to_monomial(right, subst),
        ),
    }
}

fn monomial_atom_type(monomial: &Monomial<TypeExpr>) -> TypeExpr {
    match monomial {
        Monomial::Atom(ty) => ty.clone(),
        _ => panic!("program tape expects monomial atoms as nodes"),
    }
}

fn collect_tape_constraints(
    term: &OpenHypergraph<Monomial<TypeExpr>, TapeEdge<TypeExpr, ExprGenerator>>,
    nodes: &mut Vec<TypeExpr>,
    constraints: &mut Vec<TypeConstraint>,
) {
    for mono in &term.hypergraph.nodes {
        nodes.push(monomial_atom_type(mono));
    }
    for (from, to) in term
        .hypergraph
        .quotient
        .0
        .iter()
        .zip(term.hypergraph.quotient.1.iter())
    {
        let lhs = monomial_atom_type(&term.hypergraph.nodes[from.0]);
        let rhs = monomial_atom_type(&term.hypergraph.nodes[to.0]);
        constraints.push(TypeConstraint::Equal(lhs, rhs));
    }

    for (edge_idx, edge) in term.hypergraph.edges.iter().enumerate() {
        let outer_edge = &term.hypergraph.adjacency[edge_idx];
        match edge {
            TapeEdge::Embedded(child) => {
                nodes.extend(child.hypergraph.nodes.iter().cloned());
                for (from, to) in child
                    .hypergraph
                    .quotient
                    .0
                    .iter()
                    .zip(child.hypergraph.quotient.1.iter())
                {
                    let lhs = child.hypergraph.nodes[from.0].clone();
                    let rhs = child.hypergraph.nodes[to.0].clone();
                    constraints.push(TypeConstraint::Equal(lhs, rhs));
                }
                for (outer_node, &child_node) in outer_edge.sources.iter().zip(child.sources.iter())
                {
                    let outer_ty = monomial_atom_type(&term.hypergraph.nodes[outer_node.0]);
                    let inner_ty = child.hypergraph.nodes[child_node.0].clone();
                    constraints.push(TypeConstraint::Equal(outer_ty, inner_ty));
                }
                for (outer_node, &child_node) in outer_edge.targets.iter().zip(child.targets.iter())
                {
                    let outer_ty = monomial_atom_type(&term.hypergraph.nodes[outer_node.0]);
                    let inner_ty = child.hypergraph.nodes[child_node.0].clone();
                    constraints.push(TypeConstraint::Equal(outer_ty, inner_ty));
                }
            }
            TapeEdge::Product(left, right) => {
                let left_source_len = left.sources.len();
                let left_target_len = left.targets.len();
                let (outer_left_sources, outer_right_sources) =
                    outer_edge.sources.split_at(left_source_len);
                let (outer_left_targets, outer_right_targets) =
                    outer_edge.targets.split_at(left_target_len);

                for (outer_node, &child_node) in outer_left_sources.iter().zip(left.sources.iter())
                {
                    let outer_ty = monomial_atom_type(&term.hypergraph.nodes[outer_node.0]);
                    let inner_ty = monomial_atom_type(&left.hypergraph.nodes[child_node.0]);
                    constraints.push(TypeConstraint::Equal(outer_ty, inner_ty));
                }
                for (outer_node, &child_node) in outer_left_targets.iter().zip(left.targets.iter())
                {
                    let outer_ty = monomial_atom_type(&term.hypergraph.nodes[outer_node.0]);
                    let inner_ty = monomial_atom_type(&left.hypergraph.nodes[child_node.0]);
                    constraints.push(TypeConstraint::Equal(outer_ty, inner_ty));
                }
                for (outer_node, &child_node) in
                    outer_right_sources.iter().zip(right.sources.iter())
                {
                    let outer_ty = monomial_atom_type(&term.hypergraph.nodes[outer_node.0]);
                    let inner_ty = monomial_atom_type(&right.hypergraph.nodes[child_node.0]);
                    constraints.push(TypeConstraint::Equal(outer_ty, inner_ty));
                }
                for (outer_node, &child_node) in
                    outer_right_targets.iter().zip(right.targets.iter())
                {
                    let outer_ty = monomial_atom_type(&term.hypergraph.nodes[outer_node.0]);
                    let inner_ty = monomial_atom_type(&right.hypergraph.nodes[child_node.0]);
                    constraints.push(TypeConstraint::Equal(outer_ty, inner_ty));
                }

                collect_tape_constraints(left, nodes, constraints);
                collect_tape_constraints(right, nodes, constraints);
            }
        }
    }
}

fn apply_substitution_to_edge(
    edge: &TapeEdge<TypeExpr, ExprGenerator>,
    subst: &TypeSubstitution,
) -> TapeEdge<TypeExpr, ExprGenerator> {
    match edge {
        TapeEdge::Embedded(child) => TapeEdge::Embedded(apply_substitution(child, subst)),
        TapeEdge::Product(left, right) => TapeEdge::Product(
            Box::new(apply_substitution_to_tape(left, subst)),
            Box::new(apply_substitution_to_tape(right, subst)),
        ),
    }
}

fn strictify_tape_edge(
    edge: &TapeEdge<TypeExpr, ExprGenerator>,
) -> TapeEdge<TypeExpr, ExprGenerator> {
    match edge {
        TapeEdge::Embedded(child) => {
            TapeEdge::Embedded(OpenHypergraph::from_strict(child.clone().to_strict()))
        }
        TapeEdge::Product(left, right) => TapeEdge::Product(
            Box::new(strictify_tape_graph(left)),
            Box::new(strictify_tape_graph(right)),
        ),
    }
}

fn strictify_tape_graph(
    graph: &OpenHypergraph<Monomial<TypeExpr>, TapeEdge<TypeExpr, ExprGenerator>>,
) -> OpenHypergraph<Monomial<TypeExpr>, TapeEdge<TypeExpr, ExprGenerator>> {
    let strict = graph.clone().map_edges(|edge| strictify_tape_edge(&edge));
    OpenHypergraph::from_strict(strict.to_strict())
}
