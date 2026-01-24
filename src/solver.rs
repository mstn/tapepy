use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use open_hypergraphs::lax::OpenHypergraph;

use crate::types::{TypeConstraint, TypeExpr, TypeVar};

#[derive(Debug, Clone)]
pub struct TypeSubstitution {
    mapping: BTreeMap<TypeVar, TypeExpr>,
}

impl TypeSubstitution {
    pub fn apply(&self, expr: &TypeExpr) -> TypeExpr {
        match expr {
            TypeExpr::Bool => TypeExpr::Bool,
            TypeExpr::Unit => TypeExpr::Unit,
            TypeExpr::Int => TypeExpr::Int,
            TypeExpr::Float => TypeExpr::Float,
            TypeExpr::Named(name) => TypeExpr::Named(name.clone()),
            TypeExpr::Var(var) => self
                .mapping
                .get(var)
                .cloned()
                .unwrap_or_else(|| TypeExpr::Var(var.clone())),
            TypeExpr::Union(left, right) => {
                let lhs = self.apply(left);
                let rhs = self.apply(right);
                if lhs == rhs {
                    lhs
                } else {
                    TypeExpr::Union(Box::new(lhs), Box::new(rhs))
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum SolveError {
    NoSolution,
    UnresolvedType(TypeExpr),
}

impl fmt::Display for SolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SolveError::NoSolution => write!(f, "no type assignment satisfies constraints"),
            SolveError::UnresolvedType(expr) => write!(f, "unresolved type expression: {}", expr),
        }
    }
}

impl std::error::Error for SolveError {}

pub fn solve_hypergraph_types<A: Clone>(
    graph: &OpenHypergraph<TypeExpr, A>,
) -> Result<TypeSubstitution, SolveError> {
    let vars = collect_vars(graph);
    let constraints = collect_constraints(graph);
    let vars_list: Vec<TypeVar> = vars.into_iter().collect();
    let choices = collect_choices(graph);

    let mut assignment = BTreeMap::new();
    if backtrack_solve(
        &vars_list,
        0,
        &mut assignment,
        &constraints,
        graph,
        &choices,
    ) {
        Ok(TypeSubstitution {
            mapping: assignment,
        })
    } else {
        Err(SolveError::NoSolution)
    }
}

pub fn solve_type_equations(
    nodes: &[TypeExpr],
    constraints: &[TypeConstraint],
) -> Result<TypeSubstitution, SolveError> {
    let mut vars = BTreeSet::new();
    for expr in nodes {
        collect_vars_expr(expr, &mut vars);
    }
    for constraint in constraints {
        match constraint {
            TypeConstraint::Equal(lhs, rhs) => {
                collect_vars_expr(lhs, &mut vars);
                collect_vars_expr(rhs, &mut vars);
            }
            TypeConstraint::Numeric(expr)
            | TypeConstraint::Iterable(expr)
            | TypeConstraint::Sequence(expr) => {
                collect_vars_expr(expr, &mut vars);
            }
            TypeConstraint::Mapping(key, value) => {
                collect_vars_expr(key, &mut vars);
                collect_vars_expr(value, &mut vars);
            }
        }
    }
    let vars_list: Vec<TypeVar> = vars.into_iter().collect();

    let mut choices = vec![
        TypeExpr::Bool,
        TypeExpr::Unit,
        TypeExpr::Int,
        TypeExpr::Float,
    ];
    let mut named = BTreeSet::new();
    for expr in nodes {
        collect_named_expr(expr, &mut named);
    }
    for constraint in constraints {
        match constraint {
            TypeConstraint::Equal(lhs, rhs) => {
                collect_named_expr(lhs, &mut named);
                collect_named_expr(rhs, &mut named);
            }
            TypeConstraint::Numeric(expr)
            | TypeConstraint::Iterable(expr)
            | TypeConstraint::Sequence(expr) => {
                collect_named_expr(expr, &mut named);
            }
            TypeConstraint::Mapping(key, value) => {
                collect_named_expr(key, &mut named);
                collect_named_expr(value, &mut named);
            }
        }
    }
    for name in named {
        choices.push(TypeExpr::Named(name));
    }

    let mut graph: OpenHypergraph<TypeExpr, ()> = OpenHypergraph::empty();
    graph.hypergraph.nodes = nodes.to_vec();

    let mut assignment = BTreeMap::new();
    if backtrack_solve(
        &vars_list,
        0,
        &mut assignment,
        constraints,
        &graph,
        &choices,
    ) {
        Ok(TypeSubstitution {
            mapping: assignment,
        })
    } else {
        Err(SolveError::NoSolution)
    }
}

pub fn apply_substitution<A: Clone>(
    graph: &OpenHypergraph<TypeExpr, A>,
    subst: &TypeSubstitution,
) -> OpenHypergraph<TypeExpr, A> {
    graph.clone().map_nodes(|t| subst.apply(&t))
}

fn collect_vars<A>(graph: &OpenHypergraph<TypeExpr, A>) -> BTreeSet<TypeVar> {
    let mut vars = BTreeSet::new();
    for label in &graph.hypergraph.nodes {
        collect_vars_expr(label, &mut vars);
    }
    vars
}

fn collect_vars_expr(expr: &TypeExpr, vars: &mut BTreeSet<TypeVar>) {
    match expr {
        TypeExpr::Bool | TypeExpr::Unit | TypeExpr::Int | TypeExpr::Float | TypeExpr::Named(_) => {}
        TypeExpr::Var(var) => {
            vars.insert(var.clone());
        }
        TypeExpr::Union(left, right) => {
            collect_vars_expr(left, vars);
            collect_vars_expr(right, vars);
        }
    }
}

fn collect_constraints<A>(graph: &OpenHypergraph<TypeExpr, A>) -> Vec<TypeConstraint> {
    let mut constraints = Vec::new();
    for (from, to) in graph
        .hypergraph
        .quotient
        .0
        .iter()
        .zip(graph.hypergraph.quotient.1.iter())
    {
        let lhs = graph.hypergraph.nodes[from.0].clone();
        let rhs = graph.hypergraph.nodes[to.0].clone();
        constraints.push(TypeConstraint::Equal(lhs, rhs));
    }
    constraints
}

fn collect_choices<A>(graph: &OpenHypergraph<TypeExpr, A>) -> Vec<TypeExpr> {
    let mut choices = vec![
        TypeExpr::Bool,
        TypeExpr::Unit,
        TypeExpr::Int,
        TypeExpr::Float,
    ];
    let mut named = BTreeSet::new();
    for label in &graph.hypergraph.nodes {
        collect_named_expr(label, &mut named);
    }
    for name in named {
        choices.push(TypeExpr::Named(name));
    }
    choices
}

fn collect_named_expr(expr: &TypeExpr, names: &mut BTreeSet<String>) {
    match expr {
        TypeExpr::Named(name) => {
            names.insert(name.clone());
        }
        TypeExpr::Union(left, right) => {
            collect_named_expr(left, names);
            collect_named_expr(right, names);
        }
        _ => {}
    }
}

fn backtrack_solve<A>(
    vars: &[TypeVar],
    idx: usize,
    assignment: &mut BTreeMap<TypeVar, TypeExpr>,
    constraints: &[TypeConstraint],
    graph: &OpenHypergraph<TypeExpr, A>,
    choices: &[TypeExpr],
) -> bool {
    if idx == vars.len() {
        if !constraints_satisfied(constraints, assignment) {
            return false;
        }
        return primitives_ok(graph, assignment);
    }

    let var = vars[idx].clone();
    for choice in choices {
        assignment.insert(var.clone(), choice.clone());
        if backtrack_solve(vars, idx + 1, assignment, constraints, graph, choices) {
            return true;
        }
    }
    assignment.remove(&var);
    false
}

fn eval_expr(expr: &TypeExpr, assignment: &BTreeMap<TypeVar, TypeExpr>) -> TypeExpr {
    match expr {
        TypeExpr::Bool => TypeExpr::Bool,
        TypeExpr::Unit => TypeExpr::Unit,
        TypeExpr::Int => TypeExpr::Int,
        TypeExpr::Float => TypeExpr::Float,
        TypeExpr::Named(name) => TypeExpr::Named(name.clone()),
        TypeExpr::Var(var) => assignment.get(var).cloned().unwrap_or(TypeExpr::Int),
        TypeExpr::Union(left, right) => {
            let lhs = eval_expr(left, assignment);
            let rhs = eval_expr(right, assignment);
            if lhs == rhs {
                lhs
            } else {
                TypeExpr::Union(Box::new(lhs), Box::new(rhs))
            }
        }
    }
}

fn primitives_ok<A>(
    graph: &OpenHypergraph<TypeExpr, A>,
    assignment: &BTreeMap<TypeVar, TypeExpr>,
) -> bool {
    graph.hypergraph.nodes.iter().all(|label| {
        let resolved = eval_expr(label, assignment);
        matches!(
            resolved,
            TypeExpr::Bool | TypeExpr::Unit | TypeExpr::Int | TypeExpr::Float | TypeExpr::Named(_)
        )
    })
}

fn constraints_satisfied(
    constraints: &[TypeConstraint],
    assignment: &BTreeMap<TypeVar, TypeExpr>,
) -> bool {
    for constraint in constraints {
        match constraint {
            TypeConstraint::Equal(lhs, rhs) => {
                if eval_expr(lhs, assignment) != eval_expr(rhs, assignment) {
                    return false;
                }
            }
            TypeConstraint::Numeric(expr) => {
                let resolved = eval_expr(expr, assignment);
                if !matches!(resolved, TypeExpr::Int | TypeExpr::Float) {
                    return false;
                }
            }
            TypeConstraint::Iterable(_) | TypeConstraint::Sequence(_) | TypeConstraint::Mapping(_, _) => {
                // Keep these permissive for now; they can be enforced once richer types are modeled.
            }
        }
    }
    true
}
