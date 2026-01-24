use crate::expression_circuit::circuit_from_expr_with_context;
use crate::expression_circuit::ExprGenerator;
use crate::tape_language::{Circuit, Monomial, Tape};
use crate::types::TypeExpr;
use crate::typing::{DeductionTree, ExprForm};

pub fn tape_from_predicate(tree: &DeductionTree) -> Tape<TypeExpr, ExprGenerator> {
    tape_from_predicate_with_negation(tree, false)
}

pub fn tape_from_predicate_with_negation(
    tree: &DeductionTree,
    negated: bool,
) -> Tape<TypeExpr, ExprGenerator> {
    match tree.form() {
        ExprForm::Const(label) => match (label.as_str(), negated) {
            ("True", false) | ("False", true) => {
                let context = Monomial::from_context(&tree.judgment().context().entries());
                Tape::Discard(context)
            }
            ("False", false) | ("True", true) => {
                let context = Monomial::from_context(&tree.judgment().context().entries());
                let discard = Tape::Discard(context);
                Tape::Seq(Box::new(discard), Box::new(Tape::Create(Monomial::one())))
            }
            _ => panic!("unsupported predicate constant `{}`", label),
        },
        ExprForm::BoolOp(op) => match (op.as_str(), negated) {
            ("and", false) | ("or", true) => {
                tree.assert_child_count(2, op.as_str());
                let left = tape_from_predicate_with_negation(&tree.children()[0], negated);
                let right = tape_from_predicate_with_negation(&tree.children()[1], negated);
                let copy = Tape::copy_wires(Monomial::from_context(
                    &tree.judgment().context().entries(),
                ));
                let tensor = Tape::Product(Box::new(left), Box::new(right));
                Tape::Seq(Box::new(copy), Box::new(tensor))
            }
            ("or", false) | ("and", true) => {
                tree.assert_child_count(2, op.as_str());
                let left = tape_from_predicate_with_negation(&tree.children()[0], negated);
                let right = tape_from_predicate_with_negation(&tree.children()[1], negated);
                let split = Tape::Split(Monomial::from_context(
                    &tree.judgment().context().entries(),
                ));
                let tensor = Tape::Sum(Box::new(left), Box::new(right));
                let merged = Tape::Seq(Box::new(split), Box::new(tensor));
                Tape::Seq(Box::new(merged), Box::new(Tape::Merge(Monomial::one())))
            }
            _ => panic!("unsupported predicate boolop `{}`", op),
        },
        ExprForm::UnaryOp(op) if op == "not" => {
            let child = tree
                .children()
                .get(0)
                .unwrap_or_else(|| panic!("not expects a child"));
            tape_from_predicate_with_negation(child, !negated)
        }
        ExprForm::Call(name) => {
            // Predicate judgments are typed as Unit; the type checker already enforces Bool output.
            if *tree.judgment().ty() != TypeExpr::Bool {
                panic!(
                    "predicate call `{}` must be a Bool-typed predicate, got {}",
                    name,
                    tree.judgment().ty()
                );
            }
            let args = tree.children().iter().collect();
            tape_from_relation(name.clone(), args, negated)
        }
        ExprForm::Compare(op) => {
            let args = tree.children().iter().collect();
            tape_from_relation(op.clone(), args, negated)
        }
        _ => panic!("unsupported predicate form {:?}", tree.form()),
    }
}

fn tape_from_relation(
    name: String,
    args: Vec<&DeductionTree>,
    negated: bool,
) -> Tape<TypeExpr, ExprGenerator> {
    // Predicates are represented as relation generators returning Bool, then discarded to Unit in
    // the tape language. This avoids treating them as ordinary boolean expressions.
    let context_entries = if let Some(first) = args.first() {
        let expected = first.judgment().context().entries();
        for arg in &args {
            if arg.judgment().context().entries() != expected {
                panic!("predicate relation arguments have different contexts");
            }
        }
        expected.to_vec()
    } else {
        Vec::new()
    };
    let circuit = circuit_from_relation(name, &args, &context_entries, negated);
    Tape::EmbedCircuit(Box::new(circuit))
}

fn circuit_from_relation(
    name: String,
    args: &[&DeductionTree],
    context_entries: &[(String, TypeExpr)],
    negated: bool,
) -> Circuit<TypeExpr, ExprGenerator> {
    // Build the relation circuit by wiring argument expressions from the shared context, then
    // sequencing into a relation generator (named by the predicate) that outputs Unit.
    let inputs = match args.len() {
        0 => Circuit::IdOne,
        _ => {
            let arg_circuits: Vec<_> = args
                .iter()
                .map(|arg| circuit_from_expr_with_context(arg, context_entries))
                .collect();
            if arg_circuits.len() == 1 {
                arg_circuits
                    .into_iter()
                    .next()
                    .expect("single argument circuit missing")
            } else {
                let mut input_vars = Vec::with_capacity(context_entries.len() * arg_circuits.len());
                for _ in 0..arg_circuits.len() {
                    for (name, _) in context_entries {
                        input_vars.push(name.clone());
                    }
                }
                let copy = Circuit::wiring_circuit_for_context(context_entries, &input_vars);
                let args_product = Circuit::product_many(arg_circuits);
                Circuit::seq(copy, args_product)
            }
        }
    };
    let op = Circuit::Generator(ExprGenerator::predicate(
        name,
        args.iter().map(|arg| arg.judgment().ty().clone()).collect(),
        negated,
    ));
    let base = Circuit::seq(inputs, op);
    base
}
