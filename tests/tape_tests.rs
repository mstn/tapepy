use tapepy::expression_circuit::ExprGenerator;
use tapepy::tape_language::circuit::Circuit;
use tapepy::tape_language::tape::Tape;
use tapepy::tape_language::Monomial;
use tapepy::types::TypeExpr;

#[test]
fn tape_copy_wires_embeds_copy_circuit() {
    let mono = Monomial::product(Monomial::atom(TypeExpr::Int), Monomial::atom(TypeExpr::Bool));
    let tape: Tape<TypeExpr, ExprGenerator> = Tape::copy_wires(mono.clone());

    let expected_circuit = Circuit::copy_wires(vec![TypeExpr::Int, TypeExpr::Bool]);
    match tape {
        Tape::EmbedCircuit(circuit) => assert_eq!(*circuit, expected_circuit),
        _ => panic!("expected embedded circuit from copy_wires"),
    }
}

#[test]
fn tape_embed_circuit_roundtrip_id() {
    let tape: Tape<TypeExpr, ExprGenerator> =
        Tape::EmbedCircuit(Box::new(Circuit::Id(TypeExpr::Int)));

    match tape {
        Tape::EmbedCircuit(circuit) => assert_eq!(*circuit, Circuit::Id(TypeExpr::Int)),
        _ => panic!("expected embedded circuit"),
    }
}

#[test]
fn tape_sum_builds_tensor_like_structure() {
    let left: Tape<TypeExpr, ExprGenerator> = Tape::Id(Monomial::atom(TypeExpr::Int));
    let right: Tape<TypeExpr, ExprGenerator> = Tape::Id(Monomial::atom(TypeExpr::Bool));

    let sum = Tape::Sum(Box::new(left), Box::new(right));
    match sum {
        Tape::Sum(_, _) => {}
        _ => panic!("expected sum tape"),
    }
}

#[test]
fn tape_seq_composes_two_tapes() {
    let left: Tape<TypeExpr, ExprGenerator> = Tape::Discard(Monomial::atom(TypeExpr::Int));
    let right: Tape<TypeExpr, ExprGenerator> = Tape::Create(Monomial::atom(TypeExpr::Bool));

    let seq = Tape::Seq(Box::new(left), Box::new(right));
    match seq {
        Tape::Seq(_, _) => {}
        _ => panic!("expected seq tape"),
    }
}
