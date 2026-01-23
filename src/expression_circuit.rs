use std::fmt;

use crate::tape_language::{Circuit, GeneratorShape, GeneratorTypes, Monomial};
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
            tree.assert_child_count(1, "UnaryOp");
            let child = circuit_from_expr(&tree.children()[0]);
            let gen = Circuit::Generator(ExprGenerator::typed(
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
            let inputs =
                Circuit::product_many(tree.children().iter().map(circuit_from_expr).collect());
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
            tree.assert_child_count(2, "Compare");
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
            let inputs =
                Circuit::product_many(tree.children().iter().map(circuit_from_expr).collect());
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

fn wiring_circuit_for_expression(
    tree: &DeductionTree,
    context_entries: &[(String, TypeExpr)],
) -> Circuit<TypeExpr, ExprGenerator> {
    // Determine the exact order and multiplicity of variable uses in the expression.
    let input_vars = tree.expr_input_vars();
    Circuit::wiring_circuit_for_context(context_entries, &input_vars)
}
