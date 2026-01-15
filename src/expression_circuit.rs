use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};

use open_hypergraphs::lax::OpenHypergraph;

use crate::tape_language::{Circuit, GeneratorShape, GeneratorTypes, Monomial};
use crate::types::{TypeExpr, TypeVar};
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
    pub fn new(name: impl Into<String>, arity: usize, coarity: usize) -> Self {
        Self {
            name: name.into(),
            arity,
            coarity,
            input_types: None,
            output_types: None,
        }
    }

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

/// Builds a circuit skeleton from an expression derivation tree.
/// Note: this ignores context wiring and variable sharing; composition is length-only.
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

pub fn hypergraph_from_circuit(
    circuit: &Circuit<TypeExpr, ExprGenerator>,
) -> OpenHypergraph<TypeExpr, ExprGenerator> {
    circuit.to_hypergraph(&mut || {
        let id = NEXT_TYPE_VAR.fetch_add(1, Ordering::Relaxed);
        TypeExpr::Var(TypeVar(id))
    })
}

fn product_many<S, G>(mut circuits: Vec<Circuit<S, G>>) -> Circuit<S, G> {
    if circuits.is_empty() {
        return Circuit::IdOne;
    }
    let mut acc = circuits.remove(0);
    for circuit in circuits {
        acc = Circuit::Product(Box::new(acc), Box::new(circuit));
    }
    acc
}

fn assert_child_count(tree: &DeductionTree, expected: usize, label: &str) {
    let actual = tree.children().len();
    if actual != expected {
        panic!("{} expects {} children, got {}", label, expected, actual);
    }
}

static NEXT_TYPE_VAR: AtomicUsize = AtomicUsize::new(0);

#[cfg(test)]
mod tests {
    use super::*;
    use rustpython_parser::{ast, Parse};

    #[test]
    fn complex_expression_hypergraph_contains_expected_ops() {
        let source =
            "(abs(x + 2) * float(y) + max(3, int(z))) > 0 and not (x < 1)";
        let expr = ast::Expr::parse(source, "<test>").expect("parse expression");
        let tree = crate::typing::infer_expression(&expr);
        let circuit = circuit_from_expr(&tree);
        let graph = hypergraph_from_circuit(&circuit);

        let labels: Vec<String> = graph
            .hypergraph
            .edges
            .iter()
            .map(|edge| edge.name.clone())
            .collect();

        for expected in ["abs", "+", "*", "float", "max", "int", ">", "<", "not", "and"] {
            assert!(
                labels.iter().any(|label| label == expected),
                "missing edge label `{}`",
                expected
            );
        }
    }
}
