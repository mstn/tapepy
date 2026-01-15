use std::sync::atomic::{AtomicUsize, Ordering};

use open_hypergraphs::lax::OpenHypergraph;

use crate::tape_language::{Circuit, GeneratorShape};
use crate::types::{TypeExpr, TypeVar};
use crate::typing::{DeductionTree, ExprForm};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprGenerator {
    pub name: String,
    pub arity: usize,
    pub coarity: usize,
}

impl ExprGenerator {
    pub fn new(name: impl Into<String>, arity: usize, coarity: usize) -> Self {
        Self {
            name: name.into(),
            arity,
            coarity,
        }
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

/// Builds a circuit skeleton from an expression derivation tree.
/// Note: this ignores context wiring and variable sharing; composition is length-only.
pub fn circuit_from_expr(tree: &DeductionTree) -> Circuit<TypeExpr, ExprGenerator> {
    match tree.form() {
        ExprForm::Var(_) => Circuit::Id(tree.judgment().ty().clone()),
        ExprForm::Const(label) => {
            Circuit::Generator(ExprGenerator::new(label, 0, 1))
        }
        ExprForm::UnaryOp(op) => {
            assert_child_count(tree, 1, "UnaryOp");
            let child = circuit_from_expr(&tree.children()[0]);
            let gen = Circuit::Generator(ExprGenerator::new(op, 1, 1));
            Circuit::Seq(Box::new(child), Box::new(gen))
        }
        ExprForm::BinOp(op) => {
            assert_child_count(tree, 2, "BinOp");
            let left = circuit_from_expr(&tree.children()[0]);
            let right = circuit_from_expr(&tree.children()[1]);
            let inputs = Circuit::Product(Box::new(left), Box::new(right));
            let gen = Circuit::Generator(ExprGenerator::new(op, 2, 1));
            Circuit::Seq(Box::new(inputs), Box::new(gen))
        }
        ExprForm::BoolOp(op) => {
            let inputs = product_many(tree.children().iter().map(circuit_from_expr).collect());
            let gen = Circuit::Generator(ExprGenerator::new(op, tree.children().len(), 1));
            Circuit::Seq(Box::new(inputs), Box::new(gen))
        }
        ExprForm::Compare(op) => {
            assert_child_count(tree, 2, "Compare");
            let left = circuit_from_expr(&tree.children()[0]);
            let right = circuit_from_expr(&tree.children()[1]);
            let inputs = Circuit::Product(Box::new(left), Box::new(right));
            let gen = Circuit::Generator(ExprGenerator::new(op, 2, 1));
            Circuit::Seq(Box::new(inputs), Box::new(gen))
        }
        ExprForm::Call(name) => {
            if tree.children().is_empty() {
                panic!("Call expects at least 1 child");
            }
            let inputs = product_many(tree.children().iter().map(circuit_from_expr).collect());
            let gen = Circuit::Generator(ExprGenerator::new(name, tree.children().len(), 1));
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
