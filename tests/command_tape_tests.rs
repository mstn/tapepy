use rustpython_parser::{ast, Parse};

use tapepy::command_tape::tape_from_command;
use tapepy::command_typing::infer_command_from_suite;
use tapepy::expression_circuit::ExprGenerator;
use tapepy::tape_language::circuit::Circuit;
use tapepy::tape_language::monomial_from_entries;
use tapepy::tape_language::tape::monomial_atoms;
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
                                assert_eq!(output_types, vec![TypeExpr::Int]);
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
                                            _ => {
                                                panic!("expected constant generator in assignment")
                                            }
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
                                            _ => {
                                                panic!("expected constant generator in assignment")
                                            }
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
    let (_, tape) = infer_tape("if x > 0:\n  y = 1\nelse:\n  y = 2");

    match tape {
        Tape::Seq(copy, tail) => {
            match *copy {
                Tape::EmbedCircuit(circuit) => {
                    assert_eq!(
                        *circuit,
                        Circuit::copy_wires(vec![
                            TypeExpr::Var(tapepy::types::TypeVar(0)),
                            TypeExpr::Var(tapepy::types::TypeVar(1)),
                        ])
                    );
                }
                _ => panic!("expected embedded copy circuit for if"),
            }
            match *tail {
                Tape::Seq(branches, join) => {
                    match *join {
                        Tape::EmbedCircuit(circuit) => {
                            assert_eq!(
                                *circuit,
                                Circuit::join_wires(vec![
                                    TypeExpr::Var(tapepy::types::TypeVar(0)),
                                    TypeExpr::Var(tapepy::types::TypeVar(1)),
                                ])
                            );
                        }
                        _ => panic!("expected embedded join circuit for if"),
                    }
                    match *branches {
                        Tape::Sum(left, right) => {
                            assert_eq!(tape_io_types(&left), tape_io_types(&right));
                            assert_eq!(
                                tape_io_types(&left),
                                Some((
                                    vec![
                                        TypeExpr::Var(tapepy::types::TypeVar(0)),
                                        TypeExpr::Var(tapepy::types::TypeVar(1)),
                                    ],
                                    vec![TypeExpr::Var(tapepy::types::TypeVar(0)), TypeExpr::Int]
                                ))
                            );
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

#[test]
fn three_assigns_embed_nested_seq_circuit() {
    let (_tree, tape) = infer_tape("x = 1\nx = 2\nx = 3");
    let x_ty = TypeExpr::Var(tapepy::types::TypeVar(0));

    let assign1 = Circuit::seq(
        Circuit::Id(x_ty.clone()),
        Circuit::seq(
            Circuit::Discard(x_ty.clone()),
            Circuit::Generator(ExprGenerator::Function {
                name: "1".to_string(),
                input_types: Vec::new(),
                output_types: vec![TypeExpr::Int],
            }),
        ),
    );
    let assign2 = Circuit::seq(
        Circuit::Id(x_ty.clone()),
        Circuit::seq(
            Circuit::Discard(x_ty.clone()),
            Circuit::Generator(ExprGenerator::Function {
                name: "2".to_string(),
                input_types: Vec::new(),
                output_types: vec![TypeExpr::Int],
            }),
        ),
    );
    let assign3 = Circuit::seq(
        Circuit::Id(x_ty.clone()),
        Circuit::seq(
            Circuit::Discard(x_ty.clone()),
            Circuit::Generator(ExprGenerator::Function {
                name: "3".to_string(),
                input_types: Vec::new(),
                output_types: vec![TypeExpr::Int],
            }),
        ),
    );

    let expected = Circuit::seq(Circuit::seq(assign1, assign2), assign3);

    match tape {
        Tape::EmbedCircuit(circuit) => {
            assert_eq!(*circuit, expected);
        }
        _ => panic!("expected embedded circuit for three assignments"),
    }
}

#[test]
fn nested_ifs_with_assignments_and_complex_conditions() {
    let (_tree, tape) = infer_tape(
        "if x > y:\n  if x + y > 0:\n    x = y + 1\n  else:\n    x = x + y\nelse:\n  if y > x:\n    x = x + 2\n  else:\n    x = y + 3",
    );

    let x_ty = TypeExpr::Var(tapepy::types::TypeVar(0));
    let y_ty = TypeExpr::Var(tapepy::types::TypeVar(1));
    let sum_xy_ty = TypeExpr::Var(tapepy::types::TypeVar(2));
    let x_plus_y_ty = TypeExpr::Var(tapepy::types::TypeVar(3));
    let y_plus_one_ty = TypeExpr::Int;
    let x_plus_two_ty = TypeExpr::Int;
    let y_plus_three_ty = TypeExpr::Int;
    let types = vec![x_ty.clone(), y_ty.clone()];

    let y_plus_one = expr_with_context(
        Circuit::product(Circuit::Discard(x_ty.clone()), Circuit::Id(y_ty.clone())),
        expr_binop(
            "+",
            Circuit::Id(y_ty.clone()),
            expr_const("1"),
            y_ty.clone(),
            TypeExpr::Int,
            y_plus_one_ty.clone(),
        ),
    );
    let x_plus_y = expr_with_context(
        Circuit::product(Circuit::Id(x_ty.clone()), Circuit::Id(y_ty.clone())),
        expr_binop(
            "+",
            Circuit::Id(x_ty.clone()),
            Circuit::Id(y_ty.clone()),
            x_ty.clone(),
            y_ty.clone(),
            x_plus_y_ty.clone(),
        ),
    );
    let x_plus_two = expr_with_context(
        Circuit::product(Circuit::Id(x_ty.clone()), Circuit::Discard(y_ty.clone())),
        expr_binop(
            "+",
            Circuit::Id(x_ty.clone()),
            expr_const("2"),
            x_ty.clone(),
            TypeExpr::Int,
            x_plus_two_ty.clone(),
        ),
    );
    let y_plus_three = expr_with_context(
        Circuit::product(Circuit::Discard(x_ty.clone()), Circuit::Id(y_ty.clone())),
        expr_binop(
            "+",
            Circuit::Id(y_ty.clone()),
            expr_const("3"),
            y_ty.clone(),
            TypeExpr::Int,
            y_plus_three_ty.clone(),
        ),
    );

    let assign_y_plus_one = assign_circuit_x(&x_ty, &y_ty, y_plus_one);
    let assign_x_plus_y = assign_circuit_x(&x_ty, &y_ty, x_plus_y);
    let assign_x_plus_two = assign_circuit_x(&x_ty, &y_ty, x_plus_two);
    let assign_y_plus_three = assign_circuit_x(&x_ty, &y_ty, y_plus_three);

    match tape {
        Tape::Seq(copy, tail) => {
            assert_copy_wires_tape(&copy, &types);
            match *tail {
                Tape::Seq(branches, join) => {
                    assert_join_wires_tape(&join, &types);
                    match *branches {
                        Tape::Sum(left, right) => {
                            assert_gate_tape(
                                &left,
                                &types,
                                vec![x_ty.clone(), y_ty.clone()],
                                false,
                            );
                            assert_gate_tape(
                                &right,
                                &types,
                                vec![x_ty.clone(), y_ty.clone()],
                                true,
                            );

                            assert_nested_if(
                                &left,
                                &types,
                                vec![sum_xy_ty.clone(), TypeExpr::Int],
                                assign_y_plus_one,
                                assign_x_plus_y,
                            );
                            assert_nested_if(
                                &right,
                                &types,
                                vec![y_ty.clone(), x_ty.clone()],
                                assign_x_plus_two,
                                assign_y_plus_three,
                            );
                        }
                        _ => panic!("expected sum of branches for outer if"),
                    }
                }
                _ => panic!("expected seq of branches and join for outer if"),
            }
        }
        _ => panic!("expected outer seq with copy for nested ifs"),
    }
}

fn assert_copy_wires_tape(tape: &Tape<TypeExpr, ExprGenerator>, types: &[TypeExpr]) {
    match tape {
        Tape::EmbedCircuit(circuit) => {
            assert_eq!(**circuit, Circuit::copy_wires(types.to_vec()));
        }
        _ => panic!("expected embedded copy circuit"),
    }
}

fn assert_join_wires_tape(tape: &Tape<TypeExpr, ExprGenerator>, types: &[TypeExpr]) {
    match tape {
        Tape::EmbedCircuit(circuit) => {
            assert_eq!(**circuit, Circuit::join_wires(types.to_vec()));
        }
        _ => panic!("expected embedded join circuit"),
    }
}

fn assert_gate_tape(
    tape: &Tape<TypeExpr, ExprGenerator>,
    types: &[TypeExpr],
    pred_inputs: Vec<TypeExpr>,
    negated: bool,
) {
    match tape {
        Tape::Seq(copy, prod) => {
            assert_copy_wires_tape(copy, types);
            match &**prod {
                Tape::Product(pred, _exec) => {
                    assert_predicate_compare_tape(pred, pred_inputs, negated);
                }
                _ => panic!("expected product in gate tape"),
            }
        }
        _ => panic!("expected seq in gate tape"),
    }
}

fn assert_predicate_compare_tape(
    tape: &Tape<TypeExpr, ExprGenerator>,
    expected_inputs: Vec<TypeExpr>,
    negated: bool,
) {
    match tape {
        Tape::Seq(embed, discard) => {
            assert!(matches!(
                **discard,
                Tape::Discard(Monomial::Atom(TypeExpr::Bool))
            ));
            match &**embed {
                Tape::EmbedCircuit(circuit) => match &**circuit {
                    Circuit::Seq(_, op) => match &**op {
                        Circuit::Generator(ExprGenerator::Predicate {
                            name,
                            input_types,
                            negated: gen_neg,
                        }) => {
                            assert_eq!(name, ">");
                            assert_eq!(*gen_neg, negated);
                            assert_eq!(*input_types, expected_inputs);
                        }
                        _ => panic!("expected predicate generator"),
                    },
                    _ => panic!("expected seq in predicate circuit"),
                },
                _ => panic!("expected embedded circuit in predicate tape"),
            }
        }
        _ => panic!("expected seq in predicate tape"),
    }
}

fn assert_nested_if(
    tape: &Tape<TypeExpr, ExprGenerator>,
    context_types: &[TypeExpr],
    pred_inputs: Vec<TypeExpr>,
    then_assign: Circuit<TypeExpr, ExprGenerator>,
    else_assign: Circuit<TypeExpr, ExprGenerator>,
) {
    let exec = match tape {
        Tape::Seq(_, prod) => match &**prod {
            Tape::Product(_, exec) => exec,
            _ => panic!("expected product in gate tape for nested if"),
        },
        _ => panic!("expected seq in gate tape for nested if"),
    };

    match &**exec {
        Tape::Seq(copy, tail) => {
            assert_copy_wires_tape(copy, context_types);
            match &**tail {
                Tape::Seq(branches, join) => {
                    assert_join_wires_tape(join, context_types);
                    match &**branches {
                        Tape::Sum(left, right) => {
                            assert_gate_tape(
                                left,
                                context_types,
                                pred_inputs.clone(),
                                false,
                            );
                            assert_gate_tape(
                                right,
                                context_types,
                                pred_inputs,
                                true,
                            );
                            assert_assign_tape(left, &then_assign);
                            assert_assign_tape(right, &else_assign);
                        }
                        _ => panic!("expected sum of branches in nested if"),
                    }
                }
                _ => panic!("expected seq of branches and join in nested if"),
            }
        }
        _ => panic!("expected seq in nested if tape"),
    }
}

fn assert_assign_tape(tape: &Tape<TypeExpr, ExprGenerator>, expected: &Circuit<TypeExpr, ExprGenerator>) {
    let exec = match tape {
        Tape::Seq(_, prod) => match &**prod {
            Tape::Product(_, exec) => exec,
            _ => panic!("expected product in gate tape for assignment"),
        },
        _ => panic!("expected seq in gate tape for assignment"),
    };
    match &**exec {
        Tape::EmbedCircuit(circuit) => assert_eq!(**circuit, *expected),
        _ => panic!("expected embedded circuit for assignment"),
    }
}

fn assign_circuit_x(
    x_ty: &TypeExpr,
    y_ty: &TypeExpr,
    expr_circuit: Circuit<TypeExpr, ExprGenerator>,
) -> Circuit<TypeExpr, ExprGenerator> {
    let split = Circuit::product(Circuit::Id(x_ty.clone()), Circuit::Copy(y_ty.clone()));
    let updated = Circuit::product(expr_circuit, Circuit::Id(y_ty.clone()));
    Circuit::seq(split, updated)
}

fn expr_const(label: &str) -> Circuit<TypeExpr, ExprGenerator> {
    Circuit::Generator(ExprGenerator::Function {
        name: label.to_string(),
        input_types: Vec::new(),
        output_types: vec![TypeExpr::Int],
    })
}

fn expr_binop(
    op: &str,
    left: Circuit<TypeExpr, ExprGenerator>,
    right: Circuit<TypeExpr, ExprGenerator>,
    left_ty: TypeExpr,
    right_ty: TypeExpr,
    output_ty: TypeExpr,
) -> Circuit<TypeExpr, ExprGenerator> {
    Circuit::seq(
        Circuit::product(left, right),
        Circuit::Generator(ExprGenerator::Function {
            name: op.to_string(),
            input_types: vec![left_ty, right_ty],
            output_types: vec![output_ty],
        }),
    )
}

fn expr_with_context(
    wiring: Circuit<TypeExpr, ExprGenerator>,
    expr: Circuit<TypeExpr, ExprGenerator>,
) -> Circuit<TypeExpr, ExprGenerator> {
    Circuit::seq(wiring, expr)
}

fn tape_io_types(tape: &Tape<TypeExpr, ExprGenerator>) -> Option<(Vec<TypeExpr>, Vec<TypeExpr>)> {
    match tape {
        Tape::Id(mono) => {
            let atoms = monomial_atoms(mono);
            Some((atoms_to_types(&atoms), atoms_to_types(&atoms)))
        }
        Tape::IdZero => Some((Vec::new(), Vec::new())),
        Tape::EmbedCircuit(circuit) => circuit.io_types(),
        Tape::Swap { left, right } => {
            let left_atoms = monomial_atoms(left);
            let right_atoms = monomial_atoms(right);
            let mut inputs = atoms_to_types(&left_atoms);
            inputs.extend(atoms_to_types(&right_atoms));
            let mut outputs = atoms_to_types(&right_atoms);
            outputs.extend(atoms_to_types(&left_atoms));
            Some((inputs, outputs))
        }
        Tape::Seq(left, right) => {
            let (left_in, left_out) = tape_io_types(left)?;
            let (right_in, right_out) = tape_io_types(right)?;
            if left_out != right_in {
                return None;
            }
            Some((left_in, right_out))
        }
        Tape::Product(left, right) => {
            let (left_in, left_out) = tape_io_types(left)?;
            let (right_in, right_out) = tape_io_types(right)?;
            let mut inputs = left_in;
            inputs.extend(right_in);
            let mut outputs = left_out;
            outputs.extend(right_out);
            Some((inputs, outputs))
        }
        Tape::Sum(left, right) => {
            let left_types = tape_io_types(left)?;
            let right_types = tape_io_types(right)?;
            if left_types != right_types {
                return None;
            }
            Some(left_types)
        }
        Tape::Discard(mono) => {
            let atoms = monomial_atoms(mono);
            Some((atoms_to_types(&atoms), Vec::new()))
        }
        Tape::Split(mono) => {
            let atoms = monomial_atoms(mono);
            Some((atoms_to_types(&atoms), atoms_to_types(&atoms)))
        }
        Tape::Create(mono) => {
            let atoms = monomial_atoms(mono);
            Some((Vec::new(), atoms_to_types(&atoms)))
        }
        Tape::Merge(mono) => {
            let atoms = monomial_atoms(mono);
            Some((atoms_to_types(&atoms), atoms_to_types(&atoms)))
        }
    }
}

fn atoms_to_types(atoms: &[Monomial<TypeExpr>]) -> Vec<TypeExpr> {
    atoms
        .iter()
        .map(|mono| match mono {
            Monomial::Atom(ty) => ty.clone(),
            Monomial::One | Monomial::Product(_, _) => {
                panic!("expected flat monomial atoms")
            }
        })
        .collect()
}

#[test]
fn nested_ifs_with_complex_conditions_and_assignments() {
    let (_tree, tape) = infer_tape(
        "if x > 0 and x > 1:\n  if x > 2:\n    x = 3\n  else:\n    x = 4\nelse:\n  if x > 5 or x > 6:\n    x = 7\n  else:\n    x = 8",
    );

    let x_ty = TypeExpr::Var(tapepy::types::TypeVar(0));
    let context = Monomial::Atom(x_ty.clone());

    let assign3 = assign_const(&x_ty, "3");
    let assign4 = assign_const(&x_ty, "4");
    let assign7 = assign_const(&x_ty, "7");
    let assign8 = assign_const(&x_ty, "8");

    let pred_x_gt_0 = compare_pred_tape(&x_ty, ">", "0", false);
    let pred_x_gt_1 = compare_pred_tape(&x_ty, ">", "1", false);
    let pred_x_gt_0_neg = compare_pred_tape(&x_ty, ">", "0", true);
    let pred_x_gt_1_neg = compare_pred_tape(&x_ty, ">", "1", true);

    let pred_outer = Tape::Seq(
        Box::new(Tape::copy_wires(context.clone())),
        Box::new(Tape::Product(
            Box::new(pred_x_gt_0),
            Box::new(pred_x_gt_1),
        )),
    );
    let pred_outer_neg = Tape::Seq(
        Box::new(Tape::Seq(
            Box::new(Tape::Split(context.clone())),
            Box::new(Tape::Sum(
                Box::new(pred_x_gt_0_neg),
                Box::new(pred_x_gt_1_neg),
            )),
        )),
        Box::new(Tape::Merge(Monomial::one())),
    );

    let pred_x_gt_2 = compare_pred_tape(&x_ty, ">", "2", false);
    let pred_x_gt_2_neg = compare_pred_tape(&x_ty, ">", "2", true);
    let inner_left = if_tape_with_predicate(
        &context,
        pred_x_gt_2,
        pred_x_gt_2_neg,
        Tape::EmbedCircuit(Box::new(assign3)),
        Tape::EmbedCircuit(Box::new(assign4)),
    );

    let pred_x_gt_5 = compare_pred_tape(&x_ty, ">", "5", false);
    let pred_x_gt_6 = compare_pred_tape(&x_ty, ">", "6", false);
    let pred_x_gt_5_neg = compare_pred_tape(&x_ty, ">", "5", true);
    let pred_x_gt_6_neg = compare_pred_tape(&x_ty, ">", "6", true);
    let pred_right = Tape::Seq(
        Box::new(Tape::Seq(
            Box::new(Tape::Split(context.clone())),
            Box::new(Tape::Sum(
                Box::new(pred_x_gt_5),
                Box::new(pred_x_gt_6),
            )),
        )),
        Box::new(Tape::Merge(Monomial::one())),
    );
    let pred_right_neg = Tape::Seq(
        Box::new(Tape::copy_wires(context.clone())),
        Box::new(Tape::Product(
            Box::new(pred_x_gt_5_neg),
            Box::new(pred_x_gt_6_neg),
        )),
    );
    let inner_right = if_tape_with_predicate(
        &context,
        pred_right,
        pred_right_neg,
        Tape::EmbedCircuit(Box::new(assign7)),
        Tape::EmbedCircuit(Box::new(assign8)),
    );

    let left = gate_tape_for_test(&context, pred_outer, inner_left);
    let right = gate_tape_for_test(&context, pred_outer_neg, inner_right);
    let expected = if_tape_with_branches(&context, left, right);

    assert_eq!(tape, expected);
}

fn assign_const(ty: &TypeExpr, value: &str) -> Circuit<TypeExpr, ExprGenerator> {
    Circuit::seq(
        Circuit::Id(ty.clone()),
        Circuit::seq(
            Circuit::Discard(ty.clone()),
            Circuit::Generator(ExprGenerator::Function {
                name: value.to_string(),
                input_types: Vec::new(),
                output_types: vec![TypeExpr::Int],
            }),
        ),
    )
}

fn compare_pred_tape(
    x_ty: &TypeExpr,
    op: &str,
    const_label: &str,
    negated: bool,
) -> Tape<TypeExpr, ExprGenerator> {
    let arg_x = Circuit::seq(Circuit::Id(x_ty.clone()), Circuit::Id(x_ty.clone()));
    let arg_const = Circuit::seq(
        Circuit::Discard(x_ty.clone()),
        Circuit::Generator(ExprGenerator::Function {
            name: const_label.to_string(),
            input_types: Vec::new(),
            output_types: vec![TypeExpr::Int],
        }),
    );
    let inputs = Circuit::seq(
        Circuit::Copy(x_ty.clone()),
        Circuit::product(arg_x, arg_const),
    );
    let gen = Circuit::Generator(ExprGenerator::predicate(
        op.to_string(),
        vec![x_ty.clone(), TypeExpr::Int],
        negated,
    ));
    let circuit = Circuit::seq(inputs, gen);
    Tape::Seq(
        Box::new(Tape::EmbedCircuit(Box::new(circuit))),
        Box::new(Tape::Discard(Monomial::Atom(TypeExpr::Bool))),
    )
}

fn gate_tape_for_test(
    context: &Monomial<TypeExpr>,
    pred_tape: Tape<TypeExpr, ExprGenerator>,
    exec_tape: Tape<TypeExpr, ExprGenerator>,
) -> Tape<TypeExpr, ExprGenerator> {
    let copy = Tape::copy_wires(context.clone());
    Tape::Seq(
        Box::new(copy),
        Box::new(Tape::Product(Box::new(pred_tape), Box::new(exec_tape))),
    )
}

fn if_tape_with_predicate(
    context: &Monomial<TypeExpr>,
    pred_tape: Tape<TypeExpr, ExprGenerator>,
    neg_pred_tape: Tape<TypeExpr, ExprGenerator>,
    then_tape: Tape<TypeExpr, ExprGenerator>,
    else_tape: Tape<TypeExpr, ExprGenerator>,
) -> Tape<TypeExpr, ExprGenerator> {
    let left = gate_tape_for_test(context, pred_tape, then_tape);
    let right = gate_tape_for_test(context, neg_pred_tape, else_tape);
    if_tape_with_branches(context, left, right)
}

fn if_tape_with_branches(
    context: &Monomial<TypeExpr>,
    left: Tape<TypeExpr, ExprGenerator>,
    right: Tape<TypeExpr, ExprGenerator>,
) -> Tape<TypeExpr, ExprGenerator> {
    let copy = Tape::copy_wires(context.clone());
    let join = Tape::EmbedCircuit(Box::new(Circuit::join_wires(
        monomial_atoms(context)
            .into_iter()
            .map(|mono| match mono {
                Monomial::Atom(ty) => ty,
                Monomial::One | Monomial::Product(_, _) => {
                    panic!("expected flat context monomial")
                }
            })
            .collect(),
    )));
    Tape::Seq(
        Box::new(copy),
        Box::new(Tape::Seq(
            Box::new(Tape::Sum(Box::new(left), Box::new(right))),
            Box::new(join),
        )),
    )
}
