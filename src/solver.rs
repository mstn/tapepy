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
            TypeExpr::Named(name) => TypeExpr::Named(name.clone()),
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

    let mut assignment = HashMap::new();
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
    constraints: &[(TypeExpr, TypeExpr)],
) -> Result<TypeSubstitution, SolveError> {
    let mut vars = HashSet::new();
    for expr in nodes {
        collect_vars_expr(expr, &mut vars);
    }
    for (lhs, rhs) in constraints {
        collect_vars_expr(lhs, &mut vars);
        collect_vars_expr(rhs, &mut vars);
    }
    let vars_list: Vec<TypeVar> = vars.into_iter().collect();

    let mut choices = vec![
        TypeExpr::Bool,
        TypeExpr::Unit,
        TypeExpr::Int,
        TypeExpr::Float,
    ];
    let mut named = HashSet::new();
    for expr in nodes {
        collect_named_expr(expr, &mut named);
    }
    for (lhs, rhs) in constraints {
        collect_named_expr(lhs, &mut named);
        collect_named_expr(rhs, &mut named);
    }
    for name in named {
        choices.push(TypeExpr::Named(name));
    }

    let mut graph: OpenHypergraph<TypeExpr, ()> = OpenHypergraph::empty();
    graph.hypergraph.nodes = nodes.to_vec();

    let mut assignment = HashMap::new();
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

fn collect_vars<A>(graph: &OpenHypergraph<TypeExpr, A>) -> HashSet<TypeVar> {
    let mut vars = HashSet::new();
    for label in &graph.hypergraph.nodes {
        collect_vars_expr(label, &mut vars);
    }
    vars
}

fn collect_vars_expr(expr: &TypeExpr, vars: &mut HashSet<TypeVar>) {
    match expr {
        TypeExpr::Bool | TypeExpr::Unit | TypeExpr::Int | TypeExpr::Float | TypeExpr::Named(_) => {}
        TypeExpr::Var(var) => {
            vars.insert(var.clone());
        }
        TypeExpr::Lub(left, right) => {
            collect_vars_expr(left, vars);
            collect_vars_expr(right, vars);
        }
        TypeExpr::Union(left, right) => {
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

fn collect_choices<A>(graph: &OpenHypergraph<TypeExpr, A>) -> Vec<TypeExpr> {
    let mut choices = vec![
        TypeExpr::Bool,
        TypeExpr::Unit,
        TypeExpr::Int,
        TypeExpr::Float,
    ];
    let mut named = HashSet::new();
    for label in &graph.hypergraph.nodes {
        collect_named_expr(label, &mut named);
    }
    for name in named {
        choices.push(TypeExpr::Named(name));
    }
    choices
}

fn collect_named_expr(expr: &TypeExpr, names: &mut HashSet<String>) {
    match expr {
        TypeExpr::Named(name) => {
            names.insert(name.clone());
        }
        TypeExpr::Lub(left, right) | TypeExpr::Union(left, right) => {
            collect_named_expr(left, names);
            collect_named_expr(right, names);
        }
        _ => {}
    }
}

fn backtrack_solve<A>(
    vars: &[TypeVar],
    idx: usize,
    assignment: &mut HashMap<TypeVar, TypeExpr>,
    constraints: &[(TypeExpr, TypeExpr)],
    graph: &OpenHypergraph<TypeExpr, A>,
    choices: &[TypeExpr],
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
    for choice in choices {
        assignment.insert(var.clone(), choice.clone());
        if backtrack_solve(vars, idx + 1, assignment, constraints, graph, choices) {
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
        TypeExpr::Named(name) => TypeExpr::Named(name.clone()),
        TypeExpr::Var(var) => assignment.get(var).cloned().unwrap_or(TypeExpr::Int),
        TypeExpr::Lub(left, right) => {
            let lhs = eval_expr(left, assignment);
            let rhs = eval_expr(right, assignment);
            TypeExpr::lub(lhs, rhs)
        }
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
    assignment: &HashMap<TypeVar, TypeExpr>,
) -> bool {
    graph.hypergraph.nodes.iter().all(|label| {
        let resolved = eval_expr(label, assignment);
        matches!(
            resolved,
            TypeExpr::Bool | TypeExpr::Unit | TypeExpr::Int | TypeExpr::Float | TypeExpr::Named(_)
        )
    })
}
