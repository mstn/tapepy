use tapepy::expression_circuit::ExprGenerator;
use tapepy::tape_language::tape::Tape;
use tapepy::tape_language::Monomial;
use tapepy::types::TypeExpr;

type TapeTy = Tape<TypeExpr, ExprGenerator>;

#[test]
fn arity_id_and_id_zero() {
    let mono = Monomial::product(
        Monomial::atom(TypeExpr::Int),
        Monomial::atom(TypeExpr::Bool),
    );
    let tape: TapeTy = Tape::Id(mono);
    let arity = tape.arity();
    assert_eq!(arity.inputs, 2);
    assert_eq!(arity.outputs, 2);

    let zero: TapeTy = Tape::IdZero;
    let arity = zero.arity();
    assert_eq!(arity.inputs, 0);
    assert_eq!(arity.outputs, 0);
}

#[test]
fn arity_embed_circuit_uses_circuit_arity() {
    let circuit: tapepy::tape_language::circuit::Circuit<TypeExpr, ExprGenerator> =
        tapepy::tape_language::circuit::Circuit::Copy(TypeExpr::Int);
    let tape: TapeTy = Tape::EmbedCircuit(Box::new(circuit));
    let arity = tape.arity();
    assert_eq!(arity.inputs, 1);
    assert_eq!(arity.outputs, 2);
}

#[test]
fn arity_seq_and_product() {
    let left: TapeTy = Tape::Discard(Monomial::atom(TypeExpr::Int));
    let right: TapeTy = Tape::Create(Monomial::atom(TypeExpr::Bool));
    let seq: TapeTy = Tape::Seq(Box::new(left), Box::new(right));
    let arity = seq.arity();
    assert_eq!(arity.inputs, 1);
    assert_eq!(arity.outputs, 1);

    let left: TapeTy = Tape::Id(Monomial::atom(TypeExpr::Int));
    let right: TapeTy = Tape::Id(Monomial::atom(TypeExpr::Bool));
    let prod: TapeTy = Tape::Product(Box::new(left), Box::new(right));
    let arity = prod.arity();
    assert_eq!(arity.inputs, 2);
    assert_eq!(arity.outputs, 2);
}

#[test]
fn arity_sum_requires_matching_arity() {
    let left: TapeTy = Tape::Id(Monomial::atom(TypeExpr::Int));
    let right: TapeTy = Tape::Id(Monomial::atom(TypeExpr::Int));
    let sum: TapeTy = Tape::Sum(Box::new(left), Box::new(right));
    let arity = sum.arity();
    assert_eq!(arity.inputs, 1);
    assert_eq!(arity.outputs, 1);
}

#[test]
fn arity_split_create_merge_discard() {
    let mono = Monomial::product(Monomial::atom(TypeExpr::Int), Monomial::atom(TypeExpr::Int));

    let split: TapeTy = Tape::Split(mono.clone());
    let arity = split.arity();
    assert_eq!(arity.inputs, 2);
    assert_eq!(arity.outputs, 2);

    let create: TapeTy = Tape::Create(mono.clone());
    let arity = create.arity();
    assert_eq!(arity.inputs, 0);
    assert_eq!(arity.outputs, 2);

    let merge: TapeTy = Tape::Merge(mono.clone());
    let arity = merge.arity();
    assert_eq!(arity.inputs, 2);
    assert_eq!(arity.outputs, 2);

    let discard: TapeTy = Tape::Discard(mono);
    let arity = discard.arity();
    assert_eq!(arity.inputs, 2);
    assert_eq!(arity.outputs, 0);
}
