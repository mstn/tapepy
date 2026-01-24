use rustpython_parser::{ast, Parse};

use tapepy::expression_circuit::ExprGenerator;
use tapepy::predicate_tape::tape_from_predicate;
use tapepy::tape_language::tape::Tape;
use tapepy::tape_language::{Circuit, Monomial};
use tapepy::types::TypeExpr;
use tapepy::typing::infer_predicate;

fn parse_expr(source: &str) -> ast::Expr {
    ast::Expr::parse(source, "<test>").expect("parse expression")
}

#[test]
fn predicate_true_is_discard() {
    let expr = parse_expr("True");
    let tree = infer_predicate(&expr);
    let tape = tape_from_predicate(&tree);

    assert!(matches!(tape, Tape::Discard(_)));
}

#[test]
fn predicate_false_creates_after_discard() {
    let expr = parse_expr("False");
    let tree = infer_predicate(&expr);
    let tape = tape_from_predicate(&tree);

    match tape {
        Tape::Seq(left, right) => {
            assert!(matches!(*left, Tape::Discard(_)));
            assert!(matches!(*right, Tape::Create(Monomial::One)));
        }
        _ => panic!("expected discard then create for False"),
    }
}

#[test]
fn not_comparison_sets_negated_flag() {
    let expr = parse_expr("not (x == y)");
    let tree = infer_predicate(&expr);
    let tape = tape_from_predicate(&tree);

    match tape {
        Tape::EmbedCircuit(circuit) => match *circuit {
            Circuit::Seq(_, op) => match *op {
                Circuit::Generator(ExprGenerator::Predicate { negated, .. }) => {
                    assert!(negated);
                }
                _ => panic!("expected predicate generator"),
            },
            _ => panic!("expected seq in embedded relation circuit"),
        },
        _ => panic!("expected embedded circuit for negated relation"),
    }
}

#[test]
fn and_predicate_uses_copy_then_product() {
    let expr = parse_expr("x > 0 and y > 0");
    let tree = infer_predicate(&expr);
    let tape = tape_from_predicate(&tree);

    match tape {
        Tape::Seq(embed, prod) => {
            match *embed {
                Tape::EmbedCircuit(circuit) => {
                    let context_entries = tree.judgment().context().entries();
                    let types: Vec<TypeExpr> =
                        context_entries.iter().map(|(_, ty)| ty.clone()).collect();
                    let expected = Circuit::copy_wires(types);
                    assert_eq!(*circuit, expected);
                }
                _ => panic!("expected embedded circuit copy for and predicate"),
            }
            assert!(matches!(*prod, Tape::Product(_, _)));
        }
        _ => panic!("expected embed then product for and predicate"),
    }
}

#[test]
fn or_predicate_splits_sums_then_merges() {
    let expr = parse_expr("x > 0 or y > 0");
    let tree = infer_predicate(&expr);
    let tape = tape_from_predicate(&tree);

    match tape {
        Tape::Seq(merged, tail) => {
            assert!(matches!(*tail, Tape::Merge(Monomial::One)));
            match *merged {
                Tape::Seq(split, sum) => {
                    assert!(matches!(*split, Tape::Split(_)));
                    assert!(matches!(*sum, Tape::Sum(_, _)));
                }
                _ => panic!("expected split then sum before merge"),
            }
        }
        _ => panic!("expected merged seq for or predicate"),
    }
}

#[test]
fn compare_uses_predicate_generator_with_unit_output() {
    let expr = parse_expr("x == y");
    let tree = infer_predicate(&expr);
    let tape = tape_from_predicate(&tree);

    match tape {
        Tape::EmbedCircuit(circuit) => match *circuit {
            Circuit::Seq(_, op) => match *op {
                Circuit::Generator(ExprGenerator::Predicate { .. }) => {}
                _ => panic!("expected predicate generator"),
            },
            _ => panic!("expected seq in embedded relation circuit"),
        },
        _ => panic!("expected embedded circuit for compare"),
    }
}

#[test]
fn negated_relation_sets_negated_flag() {
    let expr = parse_expr("not (x == y)");
    let tree = infer_predicate(&expr);
    let tape = tape_from_predicate(&tree);

    match tape {
        Tape::EmbedCircuit(circuit) => match *circuit {
            Circuit::Seq(_, op) => match *op {
                Circuit::Generator(ExprGenerator::Predicate { negated, .. }) => {
                    assert!(negated);
                }
                _ => panic!("expected predicate generator"),
            },
            _ => panic!("expected seq in embedded relation circuit"),
        },
        _ => panic!("expected embedded circuit for negated relation"),
    }
}

#[test]
fn predicate_call_uses_predicate_generator() {
    let expr = parse_expr("bool(x)");
    let tree = infer_predicate(&expr);
    let tape = tape_from_predicate(&tree);

    match tape {
        Tape::EmbedCircuit(circuit) => match *circuit {
            Circuit::Seq(_, op) => match *op {
                Circuit::Generator(ExprGenerator::Predicate { name, .. }) => {
                    assert_eq!(name, "bool");
                }
                _ => panic!("expected predicate generator"),
            },
            _ => panic!("expected seq in embedded relation circuit"),
        },
        _ => panic!("expected embedded circuit for predicate call"),
    }
}

#[test]
fn complex_predicate_combination_builds_nested_tape() {
    let expr = parse_expr("(x > 0 and y > 0) or not (z == 0)");
    let tree = infer_predicate(&expr);
    let tape = tape_from_predicate(&tree);

    match tape {
        Tape::Seq(merged, tail) => {
            assert!(matches!(*tail, Tape::Merge(Monomial::One)));
            match *merged {
                Tape::Seq(split, sum) => {
                    assert!(matches!(*split, Tape::Split(_)));
                    assert!(matches!(*sum, Tape::Sum(_, _)));
                }
                _ => panic!("expected split then sum before merge"),
            }
        }
        _ => panic!("expected merged seq for complex predicate"),
    }
}
