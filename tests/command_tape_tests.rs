use rustpython_parser::{ast, Parse};

use tapepy::command_tape::tape_from_command;
use tapepy::command_typing::infer_command_from_suite;
use tapepy::expression_circuit::ExprGenerator;
use tapepy::tape_language::circuit::Circuit;
use tapepy::tape_language::monomial_from_entries;
use tapepy::tape_language::tape::Tape;
use tapepy::tape_language::Monomial;
use tapepy::types::TypeExpr;

fn parse_suite(source: &str) -> Vec<ast::Stmt> {
    ast::Suite::parse(source, "<test>").expect("parse suite")
}

fn infer_tape(
    source: &str,
) -> (
    tapepy::command_typing::CommandDerivationTree,
    Tape<TypeExpr, ExprGenerator>,
) {
    let suite = parse_suite(source);
    let tree = infer_command_from_suite(&suite);
    let tape = tape_from_command(&tree);
    (tree, tape)
}

#[test]
fn skip_is_id_tape() {
    let (tree, tape) = infer_tape("pass");
    let context_entries = tree.judgment().context().entries();
    let expected = monomial_from_entries(context_entries);

    match tape {
        Tape::Id(mono) => assert_eq!(mono, expected),
        _ => panic!("expected id tape for pass"),
    }
}

#[test]
fn abort_is_discard_tape() {
    let (tree, tape) = infer_tape("raise");
    let context_entries = tree.judgment().context().entries();
    let expected = monomial_from_entries(context_entries);

    match tape {
        Tape::Discard(mono) => assert_eq!(mono, expected),
        _ => panic!("expected discard tape for raise"),
    }
}

#[test]
fn assign_is_embedded_seq_circuit() {
    let (tree, tape) = infer_tape("x = 1");
    let context_entries = tree.judgment().context().entries();
    assert_eq!(context_entries.len(), 1);
    let lhs_ty = context_entries[0].1.clone();

    match tape {
        Tape::EmbedCircuit(circuit) => match *circuit {
            Circuit::Seq(split, updated) => {
                assert_eq!(*split, Circuit::Id(lhs_ty.clone()));
                match *updated {
                    Circuit::Seq(wiring, expr) => {
                        assert_eq!(*wiring, Circuit::Discard(lhs_ty.clone()));
                        match *expr {
                            Circuit::Generator(ExprGenerator::Function {
                                name,
                                input_types,
                                output_types,
                            }) => {
                                assert_eq!(name, "1");
                                assert!(input_types.is_empty());
                                assert_eq!(output_types, vec![lhs_ty.clone()]);
                            }
                            _ => panic!("expected constant generator in assignment"),
                        }
                    }
                    _ => panic!("expected seq for updated circuit"),
                }
            }
            _ => panic!("expected seq circuit for assignment"),
        },
        _ => panic!("expected embedded circuit for assignment"),
    }
}

#[test]
fn seq_of_assigns_embeds_seq_circuit() {
    let (tree, tape) = infer_tape("x = 1\ny = 2");
    let mut iter = tree.children().iter().filter_map(|child| match child {
        tapepy::command_typing::CommandChild::Command(cmd) => Some(cmd),
        _ => None,
    });
    let left_cmd = iter.next().expect("sequence expects left command");
    let right_cmd = iter.next().expect("sequence expects right command");

    let left_entries = left_cmd.judgment().context().entries();
    let right_entries = right_cmd.judgment().context().entries();
    assert_eq!(left_entries.len(), 2);
    assert_eq!(right_entries.len(), 2);
    let x_ty = left_entries[0].1.clone();
    let y_ty_left = left_entries[1].1.clone();
    let y_ty_right = right_entries[1].1.clone();

    match tape {
        Tape::EmbedCircuit(circuit) => match *circuit {
            Circuit::Seq(left, right) => {
                match *left {
                    Circuit::Seq(split, updated) => {
                        assert_eq!(
                            *split,
                            Circuit::product(
                                Circuit::Id(x_ty.clone()),
                                Circuit::Copy(y_ty_left.clone())
                            )
                        );

                        match *updated {
                            Circuit::Product(expr, tail) => {
                                assert_eq!(*tail, Circuit::Id(y_ty_left.clone()));
                                match *expr {
                                    Circuit::Seq(wiring, gen) => {
                                        let expected_wiring = Circuit::product(
                                            Circuit::Discard(x_ty.clone()),
                                            Circuit::Discard(y_ty_left.clone()),
                                        );
                                        assert_eq!(*wiring, expected_wiring);
                                        match *gen {
                                            Circuit::Generator(ExprGenerator::Function {
                                                name,
                                                input_types,
                                                output_types,
                                            }) => {
                                                assert_eq!(name, "1");
                                                assert!(input_types.is_empty());
                                                assert_eq!(output_types, vec![TypeExpr::Int]);
                                            }
                                            _ => panic!("expected constant generator in assignment"),
                                        }
                                    }
                                    _ => panic!("expected seq for expression circuit"),
                                }
                            }
                            _ => panic!("expected product for updated circuit"),
                        }
                    }
                    _ => panic!("expected seq circuit for assignment"),
                }

                match *right {
                    Circuit::Seq(split, updated) => {
                        assert_eq!(
                            *split,
                            Circuit::product(
                                Circuit::Copy(x_ty.clone()),
                                Circuit::Id(y_ty_right.clone())
                            )
                        );

                        match *updated {
                            Circuit::Product(head, expr) => {
                                assert_eq!(*head, Circuit::Id(x_ty.clone()));
                                match *expr {
                                    Circuit::Seq(wiring, gen) => {
                                        let expected_wiring = Circuit::product(
                                            Circuit::Discard(x_ty.clone()),
                                            Circuit::Discard(y_ty_right.clone()),
                                        );
                                        assert_eq!(*wiring, expected_wiring);
                                        match *gen {
                                            Circuit::Generator(ExprGenerator::Function {
                                                name,
                                                input_types,
                                                output_types,
                                            }) => {
                                                assert_eq!(name, "2");
                                                assert!(input_types.is_empty());
                                                assert_eq!(output_types, vec![TypeExpr::Int]);
                                            }
                                            _ => panic!("expected constant generator in assignment"),
                                        }
                                    }
                                    _ => panic!("expected seq for expression circuit"),
                                }
                            }
                            _ => panic!("expected product for updated circuit"),
                        }
                    }
                    _ => panic!("expected seq circuit for assignment"),
                }
            }
            _ => panic!("expected seq circuit for command sequence"),
        },
        _ => panic!("expected embedded circuit for command sequence"),
    }
}

#[test]
fn if_builds_copy_branches_and_join() {
    let (tree, tape) = infer_tape("if x > 0:\n  y = 1\nelse:\n  y = 2");
    let context_entries = tree.judgment().context().entries();
    let expected_context = monomial_from_entries(context_entries);
    let expected_len = expected_context.len();
    let expected_types: Vec<TypeExpr> = context_entries.iter().map(|(_, ty)| ty.clone()).collect();

    match tape {
        Tape::Seq(copy, tail) => {
            match *copy {
                Tape::EmbedCircuit(circuit) => {
                    assert_eq!(*circuit, Circuit::copy_wires(expected_types.clone()));
                }
                _ => panic!("expected embedded copy circuit for if"),
            }
            match *tail {
                Tape::Seq(branches, join) => {
                    match *join {
                        Tape::EmbedCircuit(circuit) => {
                            assert_eq!(*circuit, Circuit::join_wires(expected_types.clone()));
                        }
                        _ => panic!("expected embedded join circuit for if"),
                    }
                    match *branches {
                        Tape::Sum(left, right) => {
                            assert_eq!(left.arity().inputs, expected_len);
                            assert_eq!(left.arity().outputs, expected_len);
                            assert_eq!(right.arity().inputs, expected_len);
                            assert_eq!(right.arity().outputs, expected_len);
                        }
                        _ => panic!("expected sum of branches"),
                    }
                }
                _ => panic!("expected seq of branches and join"),
            }
        }
        _ => panic!("expected outer seq with copy for if"),
    }
}

 
