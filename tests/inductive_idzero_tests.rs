use tapepy::expression_circuit::ExprGenerator;
use tapepy::tape_language::{Monomial, Polynomial, Tape, Whisker};
use tapepy::types::TypeExpr;

#[test]
fn right_whisk_poly_inductive_step_keeps_idzero_in_seq() {
    let a_sort = TypeExpr::Named("A".to_string());
    let b_sort = TypeExpr::Named("B".to_string());

    let poly = Polynomial::sum(
        Polynomial::monomial(Monomial::atom(a_sort)),
        Polynomial::monomial(Monomial::atom(b_sort)),
    );

    let tape: Tape<TypeExpr, ExprGenerator> = Tape::IdZero;
    let whisked = tape.right_whisk(&poly);
    assert_eq!(whisked, Tape::IdZero);
}
