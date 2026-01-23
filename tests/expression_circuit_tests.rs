use rustpython_parser::{ast, Parse};

use tapepy::context::Context;
use tapepy::expression_circuit::{circuit_from_expr, circuit_from_expr_with_context};
use tapepy::tape_language::circuit::Circuit;
use tapepy::typing::{infer_expression, infer_expression_in_context};
use tapepy::types::TypeExpr;

fn parse_expr(source: &str) -> ast::Expr {
    ast::Expr::parse(source, "<test>").expect("parse expression")
}

#[test]
fn var_expression_is_identity_circuit() {
    let expr = parse_expr("x");
    let tree = infer_expression(&expr);
    let circuit = circuit_from_expr(&tree);

    assert!(matches!(circuit, Circuit::Id(_)));
}

#[test]
fn const_expression_is_generator_with_zero_inputs() {
    let expr = parse_expr("1");
    let tree = infer_expression(&expr);
    let circuit = circuit_from_expr(&tree);

    match circuit {
        Circuit::Generator(gen) => {
            assert_eq!(gen.input_types.as_ref().unwrap().len(), 0);
            assert_eq!(gen.output_types.as_ref().unwrap().len(), 1);
        }
        _ => panic!("expected generator circuit for constant expression"),
    }
}

#[test]
fn binop_expression_builds_seq_with_binary_generator() {
    let expr = parse_expr("x + y");
    let tree = infer_expression(&expr);
    let circuit = circuit_from_expr(&tree);

    match circuit {
        Circuit::Seq(inputs, gen) => {
            assert!(matches!(*inputs, Circuit::Product(_, _)));
            match *gen {
                Circuit::Generator(gen) => {
                    assert_eq!(gen.input_types.as_ref().unwrap().len(), 2);
                    assert_eq!(gen.output_types.as_ref().unwrap().len(), 1);
                }
                _ => panic!("expected generator at end of binary op circuit"),
            }
        }
        _ => panic!("expected sequential circuit for binary op"),
    }
}

#[test]
fn wiring_with_context_uses_copy_for_repeated_variable() {
    let expr = parse_expr("x + x");
    let mut context = Context::default();
    context.set_var("x", TypeExpr::Int);
    let tree = infer_expression_in_context(&expr, &context);

    let circuit = circuit_from_expr_with_context(&tree, &context.entries());
    match circuit {
        Circuit::Seq(wiring, _) => {
            assert_eq!(*wiring, Circuit::Copy(TypeExpr::Int));
        }
        _ => panic!("expected wiring followed by expression circuit"),
    }
}
