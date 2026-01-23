use std::fmt;

use crate::tape_language::{Circuit, GeneratorShape, GeneratorTypes, Monomial};
use crate::types::TypeExpr;
use crate::typing::{DeductionTree, ExprForm};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprGenerator {
    Function {
        name: String,
        input_types: Vec<TypeExpr>,
        output_types: Vec<TypeExpr>,
    },
    Predicate {
        name: String,
        input_types: Vec<TypeExpr>,
        negated: bool,
    },
}

impl ExprGenerator {
    pub fn function(
        name: impl Into<String>,
        input_types: Vec<TypeExpr>,
        output_types: Vec<TypeExpr>,
    ) -> Self {
        Self::Function {
            name: name.into(),
            input_types,
            output_types,
        }
    }

    pub fn predicate(name: impl Into<String>, input_types: Vec<TypeExpr>, negated: bool) -> Self {
        Self::Predicate {
            name: name.into(),
            input_types,
            negated,
        }
    }
}

impl fmt::Display for ExprGenerator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExprGenerator::Function { name, .. } => write!(f, "{}", name),
            ExprGenerator::Predicate {
                name, negated, ..
            } => {
                if *negated {
                    write!(f, "not {}", name)
                } else {
                    write!(f, "{}", name)
                }
            }
        }
    }
}

impl GeneratorShape for ExprGenerator {
    fn arity(&self) -> usize {
        match self {
            ExprGenerator::Function { input_types, .. } => input_types.len(),
            ExprGenerator::Predicate { input_types, .. } => input_types.len(),
        }
    }

    fn coarity(&self) -> usize {
        match self {
            ExprGenerator::Function { output_types, .. } => output_types.len(),
            ExprGenerator::Predicate { .. } => 1,
        }
    }
}

impl GeneratorTypes<TypeExpr> for ExprGenerator {
    fn input_types(&self) -> Option<Vec<TypeExpr>> {
        match self {
            ExprGenerator::Function { input_types, .. } => Some(input_types.clone()),
            ExprGenerator::Predicate { input_types, .. } => Some(input_types.clone()),
        }
    }

    fn output_types(&self) -> Option<Vec<TypeExpr>> {
        match self {
            ExprGenerator::Function { output_types, .. } => Some(output_types.clone()),
            ExprGenerator::Predicate { .. } => Some(vec![TypeExpr::Bool]),
        }
    }
}

impl GeneratorTypes<Monomial<TypeExpr>> for ExprGenerator {
    fn input_types(&self) -> Option<Vec<Monomial<TypeExpr>>> {
        match self {
            ExprGenerator::Function { input_types, .. } => Some(
                input_types
                    .iter()
                    .cloned()
                    .map(Monomial::atom)
                    .collect(),
            ),
            ExprGenerator::Predicate { input_types, .. } => Some(
                input_types
                    .iter()
                    .cloned()
                    .map(Monomial::atom)
                    .collect(),
            ),
        }
    }

    fn output_types(&self) -> Option<Vec<Monomial<TypeExpr>>> {
        match self {
            ExprGenerator::Function { output_types, .. } => Some(
                output_types
                    .iter()
                    .cloned()
                    .map(Monomial::atom)
                    .collect(),
            ),
            ExprGenerator::Predicate { .. } => Some(vec![Monomial::atom(TypeExpr::Bool)]),
        }
    }
}

pub fn circuit_from_expr(tree: &DeductionTree) -> Circuit<TypeExpr, ExprGenerator> {
    match tree.form() {
        ExprForm::Var(_) => Circuit::Id(tree.judgment().ty().clone()),
        ExprForm::Const(label) => Circuit::Generator(ExprGenerator::function(
            label,
            Vec::new(),
            vec![tree.judgment().ty().clone()],
        )),
        ExprForm::UnaryOp(op) => {
            tree.assert_child_count(1, "UnaryOp");
            let child = circuit_from_expr(&tree.children()[0]);
            let gen = Circuit::Generator(ExprGenerator::function(
                op,
                vec![tree.children()[0].judgment().ty().clone()],
                vec![tree.judgment().ty().clone()],
            ));
            Circuit::Seq(Box::new(child), Box::new(gen))
        }
        ExprForm::BinOp(op) => {
            tree.assert_child_count(2, "BinOp");
            let left = circuit_from_expr(&tree.children()[0]);
            let right = circuit_from_expr(&tree.children()[1]);
            let inputs = Circuit::Product(Box::new(left), Box::new(right));
            let gen = Circuit::Generator(ExprGenerator::function(
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
            let inputs =
                Circuit::product_many(tree.children().iter().map(circuit_from_expr).collect());
            let gen = Circuit::Generator(ExprGenerator::function(
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
            tree.assert_child_count(2, "Compare");
            let left = circuit_from_expr(&tree.children()[0]);
            let right = circuit_from_expr(&tree.children()[1]);
            let inputs = Circuit::Product(Box::new(left), Box::new(right));
            let gen = Circuit::Generator(ExprGenerator::function(
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
            let inputs =
                Circuit::product_many(tree.children().iter().map(circuit_from_expr).collect());
            let gen = Circuit::Generator(ExprGenerator::function(
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

fn wiring_circuit_for_expression(
    tree: &DeductionTree,
    context_entries: &[(String, TypeExpr)],
) -> Circuit<TypeExpr, ExprGenerator> {
    // Determine the exact order and multiplicity of variable uses in the expression.
    let input_vars = tree.expr_input_vars();
    Circuit::wiring_circuit_for_context(context_entries, &input_vars)
}
