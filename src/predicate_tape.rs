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
            ("Top", false) | ("Bot", true) => {
                let context = context_monomial(tree);
                Tape::Discard(context)
            }
            ("Bot", false) | ("Top", true) => {
                let context = context_monomial(tree);
                let discard = Tape::Discard(context);
                Tape::Seq(Box::new(discard), Box::new(Tape::Create(Monomial::one())))
            }
            _ => panic!("unsupported predicate constant `{}`", label),
        },
        ExprForm::BoolOp(op) => match (op.as_str(), negated) {
            ("and", false) | ("or", true) => {
                assert_child_count(tree, 2, op.as_str());
                let left = tape_from_predicate_with_negation(&tree.children()[0], negated);
                let right = tape_from_predicate_with_negation(&tree.children()[1], negated);
                let copy = Tape::Split(context_monomial(tree));
                let tensor = Tape::Sum(Box::new(left), Box::new(right));
                Tape::Seq(Box::new(copy), Box::new(tensor))
            }
            ("or", false) | ("and", true) => {
                assert_child_count(tree, 2, op.as_str());
                let left = tape_from_predicate_with_negation(&tree.children()[0], negated);
                let right = tape_from_predicate_with_negation(&tree.children()[1], negated);
                let copy = Tape::Split(context_monomial(tree));
                let tensor = Tape::Sum(Box::new(left), Box::new(right));
                let merged = Tape::Seq(Box::new(copy), Box::new(tensor));
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
        ExprForm::Call(_) | ExprForm::Compare(_) => {
            let relation = predicate_relation(tree);
            tape_from_relation(relation.name, relation.args, negated)
        }
        _ => panic!("unsupported predicate form {:?}", tree.form()),
    }
}

struct Relation<'a> {
    name: String,
    args: Vec<&'a DeductionTree>,
}

fn predicate_relation(tree: &DeductionTree) -> Relation<'_> {
    match tree.form() {
        ExprForm::UnaryOp(op) if op == "not" => {
            assert_child_count(tree, 1, "not");
            predicate_relation(&tree.children()[0])
        }
        ExprForm::Call(name) => Relation {
            name: name.clone(),
            args: tree.children().iter().collect(),
        },
        ExprForm::Compare(op) => Relation {
            name: op.clone(),
            args: tree.children().iter().collect(),
        },
        _ => panic!("predicate relation expects a call or comparison"),
    }
}

fn tape_from_relation(
    name: String,
    args: Vec<&DeductionTree>,
    negated: bool,
) -> Tape<TypeExpr, ExprGenerator> {
    let context_entries = context_entries_from_args(&args);
    let context = context_monomial_from_entries(&context_entries);
    let circuit = circuit_from_relation(name, &args, &context_entries, negated);
    let embed = Tape::EmbedCircuit(Box::new(circuit));
    let discard = Tape::Discard(Monomial::atom(TypeExpr::Unit));
    let embed = Tape::Seq(Box::new(embed), Box::new(discard));
    if args.len() > 1 {
        Tape::Seq(Box::new(Tape::Split(context)), Box::new(embed))
    } else {
        embed
    }
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
            let copy = Circuit::copy_n(context_types);
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

fn context_monomial(tree: &DeductionTree) -> Monomial<TypeExpr> {
    context_monomial_from_entries(tree.judgment().context().entries())
}

fn context_entries_from_args(args: &[&DeductionTree]) -> Vec<(String, TypeExpr)> {
    if let Some(first) = args.first() {
        first.judgment().context().entries().to_vec()
    } else {
        Vec::new()
    }
}

fn context_monomial_from_entries(entries: &[(String, TypeExpr)]) -> Monomial<TypeExpr> {
    entries.iter().fold(Monomial::one(), |acc, (_, ty)| {
        Monomial::product(acc, Monomial::atom(ty.clone()))
    })
}

fn assert_child_count(tree: &DeductionTree, expected: usize, label: &str) {
    let actual = tree.children().len();
    if actual != expected {
        panic!("{} expects {} children, got {}", label, expected, actual);
    }
}
