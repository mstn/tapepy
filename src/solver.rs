use std::collections::{HashMap, HashSet};
use std::fmt;

use open_hypergraphs::lax::OpenHypergraph;

use crate::types::{TypeExpr, TypeVar};

#[derive(Debug, Clone)]
pub struct TypeSubstitution {
    mapping: HashMap<TypeVar, TypeExpr>,
}

impl TypeSubstitution {
    pub fn apply(&self, expr: &TypeExpr) -> TypeExpr {
        match expr {
            TypeExpr::Bool => TypeExpr::Bool,
            TypeExpr::Unit => TypeExpr::Unit,
            TypeExpr::Int => TypeExpr::Int,
            TypeExpr::Float => TypeExpr::Float,
            TypeExpr::Var(var) => self
                .mapping
                .get(var)
                .cloned()
                .unwrap_or_else(|| TypeExpr::Var(var.clone())),
            TypeExpr::Lub(left, right) => {
                let lhs = self.apply(left);
                let rhs = self.apply(right);
                TypeExpr::lub(lhs, rhs)
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

    let mut assignment = HashMap::new();
    if backtrack_solve(&vars_list, 0, &mut assignment, &constraints, graph) {
        Ok(TypeSubstitution { mapping: assignment })
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

fn collect_vars<A>(graph: &OpenHypergraph<TypeExpr, A>) -> HashSet<TypeVar> {
    let mut vars = HashSet::new();
    for label in &graph.hypergraph.nodes {
        collect_vars_expr(label, &mut vars);
    }
    vars
}

fn collect_vars_expr(expr: &TypeExpr, vars: &mut HashSet<TypeVar>) {
    match expr {
        TypeExpr::Bool | TypeExpr::Unit | TypeExpr::Int | TypeExpr::Float => {}
        TypeExpr::Var(var) => {
            vars.insert(var.clone());
        }
        TypeExpr::Lub(left, right) => {
            collect_vars_expr(left, vars);
            collect_vars_expr(right, vars);
        }
    }
}

fn collect_constraints<A>(graph: &OpenHypergraph<TypeExpr, A>) -> Vec<(TypeExpr, TypeExpr)> {
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
        constraints.push((lhs, rhs));
    }
    constraints
}

fn backtrack_solve<A>(
    vars: &[TypeVar],
    idx: usize,
    assignment: &mut HashMap<TypeVar, TypeExpr>,
    constraints: &[(TypeExpr, TypeExpr)],
    graph: &OpenHypergraph<TypeExpr, A>,
) -> bool {
    if idx == vars.len() {
        let constraints_ok = constraints
            .iter()
            .all(|(lhs, rhs)| eval_expr(lhs, assignment) == eval_expr(rhs, assignment));
        if !constraints_ok {
            return false;
        }
        return primitives_ok(graph, assignment);
    }

    let var = vars[idx].clone();
    for choice in [TypeExpr::Bool, TypeExpr::Unit, TypeExpr::Int, TypeExpr::Float] {
        assignment.insert(var.clone(), choice.clone());
        if backtrack_solve(vars, idx + 1, assignment, constraints, graph) {
            return true;
        }
    }
    assignment.remove(&var);
    false
}

fn eval_expr(expr: &TypeExpr, assignment: &HashMap<TypeVar, TypeExpr>) -> TypeExpr {
    match expr {
        TypeExpr::Bool => TypeExpr::Bool,
        TypeExpr::Unit => TypeExpr::Unit,
        TypeExpr::Int => TypeExpr::Int,
        TypeExpr::Float => TypeExpr::Float,
        TypeExpr::Var(var) => assignment
            .get(var)
            .cloned()
            .unwrap_or(TypeExpr::Int),
        TypeExpr::Lub(left, right) => {
            let lhs = eval_expr(left, assignment);
            let rhs = eval_expr(right, assignment);
            TypeExpr::lub(lhs, rhs)
        }
    }
}

fn primitives_ok<A>(
    graph: &OpenHypergraph<TypeExpr, A>,
    assignment: &HashMap<TypeVar, TypeExpr>,
) -> bool {
    graph.hypergraph.nodes.iter().all(|label| {
        let resolved = eval_expr(label, assignment);
        matches!(
            resolved,
            TypeExpr::Bool | TypeExpr::Unit | TypeExpr::Int | TypeExpr::Float
        )
    })
}
