use crate::expression_circuit::circuit_from_expr_with_context;
use crate::expression_circuit::ExprGenerator;
use crate::tape_language::{monomial_from_entries, Circuit, Monomial, Tape};
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
                let context = monomial_from_entries(&tree.judgment().context().entries());
                Tape::Discard(context)
            }
            ("False", false) | ("True", true) => {
                let context = monomial_from_entries(&tree.judgment().context().entries());
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
                let copy =
                    Tape::copy_wires(monomial_from_entries(&tree.judgment().context().entries()));
                let tensor = Tape::Product(Box::new(left), Box::new(right));
                Tape::Seq(Box::new(copy), Box::new(tensor))
            }
            ("or", false) | ("and", true) => {
                tree.assert_child_count(2, op.as_str());
                let left = tape_from_predicate_with_negation(&tree.children()[0], negated);
                let right = tape_from_predicate_with_negation(&tree.children()[1], negated);
                let split =
                    Tape::Split(monomial_from_entries(&tree.judgment().context().entries()));
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
    let embed = Tape::EmbedCircuit(Box::new(circuit));
    let discard = Tape::Discard(Monomial::atom(TypeExpr::Unit));
    Tape::Seq(Box::new(embed), Box::new(discard))
}

fn circuit_from_relation(
    name: String,
    args: &[&DeductionTree],
    context_entries: &[(String, TypeExpr)],
    negated: bool,
) -> Circuit<TypeExpr, ExprGenerator> {
    let context_types: Vec<TypeExpr> = context_entries.iter().map(|(_, ty)| ty.clone()).collect();
    let inputs = match args.len() {
        0 => Circuit::IdOne,
        1 => circuit_from_expr_with_context(args[0], context_entries),
        2 => {
            let left = circuit_from_expr_with_context(args[0], context_entries);
            let right = circuit_from_expr_with_context(args[1], context_entries);
            let copy = Circuit::copy_wires(context_types);
            let pair = Circuit::Product(Box::new(left), Box::new(right));
            Circuit::Seq(Box::new(copy), Box::new(pair))
        }
        _ => {
            panic!("predicate relations with more than 2 arguments are not supported");
        }
    };
    let rel_name = if negated {
        format!("not {}", name)
    } else {
        name
    };
    let op = Circuit::Generator(ExprGenerator::typed(
        rel_name,
        args.iter().map(|arg| arg.judgment().ty().clone()).collect(),
        vec![TypeExpr::Unit],
    ));
    let base = Circuit::Seq(Box::new(inputs), Box::new(op));
    base
}
