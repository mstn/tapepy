use std::fmt;

use crate::tape_language::{product_many, Circuit, GeneratorShape, GeneratorTypes, Monomial};
use crate::types::TypeExpr;
use crate::typing::{DeductionTree, ExprForm};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprGenerator {
    pub name: String,
    pub arity: usize,
    pub coarity: usize,
    pub input_types: Option<Vec<TypeExpr>>,
    pub output_types: Option<Vec<TypeExpr>>,
}

impl ExprGenerator {
    pub fn typed(
        name: impl Into<String>,
        input_types: Vec<TypeExpr>,
        output_types: Vec<TypeExpr>,
    ) -> Self {
        let arity = input_types.len();
        let coarity = output_types.len();
        Self {
            name: name.into(),
            arity,
            coarity,
            input_types: Some(input_types),
            output_types: Some(output_types),
        }
    }
}

impl fmt::Display for ExprGenerator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl GeneratorShape for ExprGenerator {
    fn arity(&self) -> usize {
        self.arity
    }

    fn coarity(&self) -> usize {
        self.coarity
    }
}

impl GeneratorTypes<TypeExpr> for ExprGenerator {
    fn input_types(&self) -> Option<Vec<TypeExpr>> {
        self.input_types.clone()
    }

    fn output_types(&self) -> Option<Vec<TypeExpr>> {
        self.output_types.clone()
    }
}

impl GeneratorTypes<Monomial<TypeExpr>> for ExprGenerator {
    fn input_types(&self) -> Option<Vec<Monomial<TypeExpr>>> {
        self.input_types
            .as_ref()
            .map(|inputs| inputs.iter().cloned().map(Monomial::atom).collect())
    }

    fn output_types(&self) -> Option<Vec<Monomial<TypeExpr>>> {
        self.output_types
            .as_ref()
            .map(|outputs| outputs.iter().cloned().map(Monomial::atom).collect())
    }
}

pub fn circuit_from_expr(tree: &DeductionTree) -> Circuit<TypeExpr, ExprGenerator> {
    match tree.form() {
        ExprForm::Var(_) => Circuit::Id(tree.judgment().ty().clone()),
        ExprForm::Const(label) => Circuit::Generator(ExprGenerator::typed(
            label,
            Vec::new(),
            vec![tree.judgment().ty().clone()],
        )),
        ExprForm::UnaryOp(op) => {
            assert_child_count(tree, 1, "UnaryOp");
            let child = circuit_from_expr(&tree.children()[0]);
            let gen = Circuit::Generator(ExprGenerator::typed(
                op,
                vec![tree.children()[0].judgment().ty().clone()],
                vec![tree.judgment().ty().clone()],
            ));
            Circuit::Seq(Box::new(child), Box::new(gen))
        }
        ExprForm::BinOp(op) => {
            assert_child_count(tree, 2, "BinOp");
            let left = circuit_from_expr(&tree.children()[0]);
            let right = circuit_from_expr(&tree.children()[1]);
            let inputs = Circuit::Product(Box::new(left), Box::new(right));
            let gen = Circuit::Generator(ExprGenerator::typed(
                op,
                vec![
                    tree.children()[0].judgment().ty().clone(),
                    tree.children()[1].judgment().ty().clone(),
                ],
                vec![tree.judgment().ty().clone()],
            ));
            Circuit::Seq(Box::new(inputs), Box::new(gen))
        }
        ExprForm::BoolOp(op) => {
            let inputs = product_many(tree.children().iter().map(circuit_from_expr).collect());
            let gen = Circuit::Generator(ExprGenerator::typed(
                op,
                tree.children()
                    .iter()
                    .map(|child| child.judgment().ty().clone())
                    .collect(),
                vec![tree.judgment().ty().clone()],
            ));
            Circuit::Seq(Box::new(inputs), Box::new(gen))
        }
        ExprForm::Compare(op) => {
            assert_child_count(tree, 2, "Compare");
            let left = circuit_from_expr(&tree.children()[0]);
            let right = circuit_from_expr(&tree.children()[1]);
            let inputs = Circuit::Product(Box::new(left), Box::new(right));
            let gen = Circuit::Generator(ExprGenerator::typed(
                op,
                vec![
                    tree.children()[0].judgment().ty().clone(),
                    tree.children()[1].judgment().ty().clone(),
                ],
                vec![tree.judgment().ty().clone()],
            ));
            Circuit::Seq(Box::new(inputs), Box::new(gen))
        }
        ExprForm::Call(name) => {
            if tree.children().is_empty() {
                panic!("Call expects at least 1 child");
            }
            let inputs = product_many(tree.children().iter().map(circuit_from_expr).collect());
            let gen = Circuit::Generator(ExprGenerator::typed(
                name,
                tree.children()
                    .iter()
                    .map(|child| child.judgment().ty().clone())
                    .collect(),
                vec![tree.judgment().ty().clone()],
            ));
            Circuit::Seq(Box::new(inputs), Box::new(gen))
        }
    }
}

pub fn circuit_from_expr_with_context(
    tree: &DeductionTree,
    context_entries: &[(String, TypeExpr)],
) -> Circuit<TypeExpr, ExprGenerator> {
    // Build the input wiring (copy/discard/permute) from the context, then
    // feed the resulting wires into the expression body.
    let wiring = wiring_circuit_for_expression(tree, context_entries);
    let expr = circuit_from_expr(tree);
    Circuit::Seq(Box::new(wiring), Box::new(expr))
}

fn assert_child_count(tree: &DeductionTree, expected: usize, label: &str) {
    let actual = tree.children().len();
    if actual != expected {
        panic!("{} expects {} children, got {}", label, expected, actual);
    }
}

fn expr_input_vars(tree: &DeductionTree) -> Vec<String> {
    match tree.form() {
        ExprForm::Var(name) => vec![name.clone()],
        ExprForm::Const(_) => Vec::new(),
        ExprForm::UnaryOp(_) => tree
            .children()
            .get(0)
            .map(expr_input_vars)
            .unwrap_or_default(),
        ExprForm::Call(_) | ExprForm::BoolOp(_) => {
            let mut vars = Vec::new();
            for child in tree.children() {
                vars.extend(expr_input_vars(child));
            }
            vars
        }
        ExprForm::BinOp(_) | ExprForm::Compare(_) => {
            if tree.children().len() != 2 {
                return Vec::new();
            }
            let mut left = expr_input_vars(&tree.children()[0]);
            let mut right = expr_input_vars(&tree.children()[1]);
            left.append(&mut right);
            left
        }
    }
}

fn wiring_circuit_for_expression(
    tree: &DeductionTree,
    context_entries: &[(String, TypeExpr)],
) -> Circuit<TypeExpr, ExprGenerator> {
    // Determine the exact order and multiplicity of variable uses in the expression.
    let input_vars = expr_input_vars(tree);
    let mut counts = Vec::with_capacity(context_entries.len());
    for (name, _) in context_entries {
        let count = input_vars.iter().filter(|var| *var == name).count();
        counts.push(count);
    }

    // For each context entry, build the required fanout (or discard) circuit.
    let mut var_circuits = Vec::with_capacity(context_entries.len());
    for ((_, ty), count) in context_entries.iter().zip(counts.iter().copied()) {
        var_circuits.push(copy_n(ty.clone(), count));
    }
    let grouped = product_many(var_circuits);

    // Reorder grouped wires to match the expression's traversal order.
    let grouped_types = grouped_types(context_entries, &counts);
    let permutation = permutation_for_inputs(context_entries, &input_vars, &counts);
    if permutation.is_identity() {
        grouped
    } else {
        let perm = permute_circuit(&grouped_types, &permutation);
        Circuit::Seq(Box::new(grouped), Box::new(perm))
    }
}

fn grouped_types(context_entries: &[(String, TypeExpr)], counts: &[usize]) -> Vec<TypeExpr> {
    let mut types = Vec::new();
    for ((_, ty), count) in context_entries.iter().zip(counts.iter().copied()) {
        for _ in 0..count {
            types.push(ty.clone());
        }
    }
    types
}

fn permutation_for_inputs(
    context_entries: &[(String, TypeExpr)],
    input_vars: &[String],
    counts: &[usize],
) -> Permutation {
    let mut offsets = Vec::with_capacity(counts.len());
    let mut running = 0;
    for count in counts {
        offsets.push(running);
        running += *count;
    }

    let mut seen = vec![0usize; counts.len()];
    let mut permutation = Vec::with_capacity(input_vars.len());
    for name in input_vars {
        let idx = context_entries
            .iter()
            .position(|(var, _)| var == name)
            .unwrap_or_else(|| panic!("variable `{}` not in context", name));
        let offset = offsets[idx];
        let use_idx = offset + seen[idx];
        seen[idx] += 1;
        permutation.push(use_idx);
    }
    Permutation(permutation)
}

fn copy_n(ty: TypeExpr, count: usize) -> Circuit<TypeExpr, ExprGenerator> {
    match count {
        0 => Circuit::Discard(ty),
        1 => Circuit::Id(ty),
        2 => Circuit::Copy(ty),
        _ => {
            // Expand fanout by one wire at a time.
            let left = Circuit::Id(ty.clone());
            let right = copy_n(ty.clone(), count - 1);
            let prod = Circuit::Product(Box::new(left), Box::new(right));
            Circuit::Seq(Box::new(Circuit::Copy(ty)), Box::new(prod))
        }
    }
}

fn permute_circuit(
    types: &[TypeExpr],
    permutation: &Permutation,
) -> Circuit<TypeExpr, ExprGenerator> {
    let mut current: Vec<usize> = (0..types.len()).collect();
    let mut current_types: Vec<TypeExpr> = types.to_vec();
    let mut swaps = Vec::new();

    for target_idx in 0..permutation.0.len() {
        let desired = permutation.0[target_idx];
        let mut pos = current
            .iter()
            .position(|idx| *idx == desired)
            .unwrap_or_else(|| panic!("permutation missing index {}", desired));
        while pos > target_idx {
            swaps.push(swap_adjacent(&current_types, pos - 1));
            current.swap(pos - 1, pos);
            current_types.swap(pos - 1, pos);
            pos -= 1;
        }
    }

    if swaps.is_empty() {
        identity_for_types(types)
    } else {
        swaps
            .into_iter()
            .fold(identity_for_types(types), |acc, swap| {
                Circuit::Seq(Box::new(acc), Box::new(swap))
            })
    }
}

fn identity_for_types(types: &[TypeExpr]) -> Circuit<TypeExpr, ExprGenerator> {
    if types.is_empty() {
        return Circuit::IdOne;
    }
    let mut circuits = Vec::with_capacity(types.len());
    for ty in types {
        circuits.push(Circuit::Id(ty.clone()));
    }
    product_many(circuits)
}

fn swap_adjacent(types: &[TypeExpr], index: usize) -> Circuit<TypeExpr, ExprGenerator> {
    let left = identity_for_types(&types[..index]);
    let mid = Circuit::Swap {
        left: types[index].clone(),
        right: types[index + 1].clone(),
    };
    let right = identity_for_types(&types[index + 2..]);

    match (left, right) {
        (Circuit::IdOne, Circuit::IdOne) => mid,
        (Circuit::IdOne, right) => Circuit::Product(Box::new(mid), Box::new(right)),
        (left, Circuit::IdOne) => Circuit::Product(Box::new(left), Box::new(mid)),
        (left, right) => {
            let mid_right = Circuit::Product(Box::new(mid), Box::new(right));
            Circuit::Product(Box::new(left), Box::new(mid_right))
        }
    }
}

struct Permutation(Vec<usize>);

impl Permutation {
    fn is_identity(&self) -> bool {
        self.0.iter().enumerate().all(|(idx, val)| idx == *val)
    }
}

fn lookup_var_type(name: &str, context_entries: &[(String, TypeExpr)]) -> TypeExpr {
    context_entries
        .iter()
        .find(|(var, _)| var == name)
        .map(|(_, ty)| ty.clone())
        .unwrap_or_else(|| panic!("variable `{}` not in context", name))
}
