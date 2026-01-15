use crate::expression_circuit::circuit_from_expr;
use crate::expression_circuit::ExprGenerator;
use crate::tape_language::{Circuit, Monomial, Tape};
use crate::types::TypeExpr;
use crate::typing::{DeductionTree, ExprForm};

pub fn tape_from_predicate(tree: &DeductionTree) -> Tape<TypeExpr, ExprGenerator> {
    match tree.form() {
        ExprForm::Const(label) => match label.as_str() {
            "Top" => {
                let context = context_monomial(tree);
                Tape::Discard(context)
            }
            "Bot" => {
                let context = context_monomial(tree);
                let discard = Tape::Discard(context);
                Tape::Seq(Box::new(discard), Box::new(Tape::Create(Monomial::one())))
            }
            _ => panic!("unsupported predicate constant `{}`", label),
        },
        ExprForm::BoolOp(op) => match op.as_str() {
            "and" => {
                assert_child_count(tree, 2, "and");
                let left = tape_from_predicate(&tree.children()[0]);
                let right = tape_from_predicate(&tree.children()[1]);
                let copy = Tape::Split(context_monomial(tree));
                let tensor = Tape::Sum(Box::new(left), Box::new(right));
                Tape::Seq(Box::new(copy), Box::new(tensor))
            }
            "or" => {
                assert_child_count(tree, 2, "or");
                let left = tape_from_predicate(&tree.children()[0]);
                let right = tape_from_predicate(&tree.children()[1]);
                let copy = Tape::Split(context_monomial(tree));
                let tensor = Tape::Sum(Box::new(left), Box::new(right));
                let merged = Tape::Seq(Box::new(copy), Box::new(tensor));
                Tape::Seq(Box::new(merged), Box::new(Tape::Merge(Monomial::one())))
            }
            _ => panic!("unsupported predicate boolop `{}`", op),
        },
        ExprForm::UnaryOp(op) if op == "not" => {
            let relation = predicate_relation(tree);
            tape_from_relation(relation.name, relation.args, true)
        }
        ExprForm::Call(_) | ExprForm::Compare(_) => {
            let relation = predicate_relation(tree);
            tape_from_relation(relation.name, relation.args, false)
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
    let context = context_monomial_from_args(&args);
    let circuit = circuit_from_relation(name, &args, negated);
    let embed = Tape::EmbedCircuit(Box::new(circuit));
    if args.len() > 1 {
        Tape::Seq(Box::new(Tape::Split(context)), Box::new(embed))
    } else {
        embed
    }
}

fn circuit_from_relation(
    name: String,
    args: &[&DeductionTree],
    negated: bool,
) -> Circuit<TypeExpr, ExprGenerator> {
    let inputs = product_many(args.iter().map(|arg| circuit_from_expr(arg)).collect());
    let op = Circuit::Generator(ExprGenerator::new(name, args.len(), 1));
    let base = Circuit::Seq(Box::new(inputs), Box::new(op));
    if negated {
        let not = Circuit::Generator(ExprGenerator::new("not", 1, 1));
        Circuit::Seq(Box::new(base), Box::new(not))
    } else {
        base
    }
}

fn context_monomial(tree: &DeductionTree) -> Monomial<TypeExpr> {
    context_monomial_from_entries(tree.judgment().context().entries())
}

fn context_monomial_from_args(args: &[&DeductionTree]) -> Monomial<TypeExpr> {
    if let Some(first) = args.first() {
        context_monomial_from_entries(first.judgment().context().entries())
    } else {
        Monomial::one()
    }
}

fn context_monomial_from_entries(entries: &[(String, TypeExpr)]) -> Monomial<TypeExpr> {
    entries.iter().fold(Monomial::one(), |acc, (_, ty)| {
        Monomial::product(acc, Monomial::atom(ty.clone()))
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
