use open_hypergraphs::lax::OpenHypergraph;

use crate::expression_circuit::ExprGenerator;
use crate::solver::{apply_substitution, solve_type_equations, TypeSubstitution};
use crate::tape_language::Monomial;
use crate::types::TypeExpr;

pub fn solve_and_strictify_program_tape(
    term: &OpenHypergraph<Monomial<TypeExpr>, OpenHypergraph<TypeExpr, ExprGenerator>>,
) -> OpenHypergraph<Monomial<TypeExpr>, OpenHypergraph<TypeExpr, ExprGenerator>> {
    let (nodes, constraints) = build_program_type_equations(term);
    let subst = solve_type_equations(&nodes, &constraints)
        .unwrap_or_else(|err| panic!("type solving failed for program tape: {}", err));
    let solved = apply_substitution_to_tape(term, &subst);
    let strict_inner = solved.map_edges(|edge| OpenHypergraph::from_strict(edge.to_strict()));
    OpenHypergraph::from_strict(strict_inner.to_strict())
}

fn build_program_type_equations(
    term: &OpenHypergraph<Monomial<TypeExpr>, OpenHypergraph<TypeExpr, ExprGenerator>>,
) -> (Vec<TypeExpr>, Vec<(TypeExpr, TypeExpr)>) {
    let mut nodes = Vec::new();
    let mut constraints = Vec::new();

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
        constraints.push((lhs, rhs));
    }

    for (edge_idx, child) in term.hypergraph.edges.iter().enumerate() {
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
            constraints.push((lhs, rhs));
        }

        let outer_edge = &term.hypergraph.adjacency[edge_idx];
        for (outer_node, &child_node) in outer_edge.sources.iter().zip(child.sources.iter()) {
            let outer_ty = monomial_atom_type(&term.hypergraph.nodes[outer_node.0]);
            let inner_ty = child.hypergraph.nodes[child_node.0].clone();
            constraints.push((outer_ty, inner_ty));
        }
        for (outer_node, &child_node) in outer_edge.targets.iter().zip(child.targets.iter()) {
            let outer_ty = monomial_atom_type(&term.hypergraph.nodes[outer_node.0]);
            let inner_ty = child.hypergraph.nodes[child_node.0].clone();
            constraints.push((outer_ty, inner_ty));
        }
    }

    (nodes, constraints)
}

fn apply_substitution_to_tape(
    term: &OpenHypergraph<Monomial<TypeExpr>, OpenHypergraph<TypeExpr, ExprGenerator>>,
    subst: &TypeSubstitution,
) -> OpenHypergraph<Monomial<TypeExpr>, OpenHypergraph<TypeExpr, ExprGenerator>> {
    term.clone()
        .map_nodes(|mono| apply_substitution_to_monomial(&mono, subst))
        .map_edges(|edge| apply_substitution(&edge, subst))
}

fn apply_substitution_to_monomial(
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
