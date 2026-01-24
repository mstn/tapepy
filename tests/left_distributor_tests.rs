use tapepy::expression_circuit::ExprGenerator;
use tapepy::tape_language::{left_distributor, Monomial, Polynomial, Tape};
use tapepy::types::TypeExpr;

fn atom(name: &str) -> Monomial<TypeExpr> {
    Monomial::atom(TypeExpr::Named(name.to_string()))
}

#[test]
fn left_distributor_zero_is_idzero() {
    let p: Polynomial<TypeExpr> = Polynomial::zero();
    let q = Polynomial::monomial(atom("V"));
    let r = Polynomial::monomial(atom("W"));

    let actual: Tape<TypeExpr, ExprGenerator> = left_distributor(&p, &q, &r);

    assert_eq!(actual, Tape::IdZero);
}

#[test]
fn left_distributor_on_simple_polys() {
    let u = atom("U");
    let u2 = atom("U2");
    let v = atom("V");
    let w = atom("W");

    let p = Polynomial::sum(
        Polynomial::monomial(u.clone()),
        Polynomial::monomial(u2.clone()),
    );
    let q = Polynomial::monomial(v.clone());
    let r = Polynomial::monomial(w.clone());

    let actual: Tape<TypeExpr, ExprGenerator> = left_distributor(&p, &q, &r);
    let (inputs, outputs) = actual.io_types().expect("expected io types");

    let expected_inputs = vec![
        u.clone(),
        v.clone(),
        u.clone(),
        w.clone(),
        u2.clone(),
        v.clone(),
        u2.clone(),
        w.clone(),
    ];
    let expected_outputs = vec![
        u.clone(),
        v.clone(),
        u2.clone(),
        v.clone(),
        u.clone(),
        w.clone(),
        u2,
        w,
    ];

    assert_eq!(inputs, expected_inputs);
    assert_eq!(outputs, expected_outputs);
}

#[test]
fn left_distributor_reorders_expanded_terms() {
    let a = atom("A");
    let b = atom("B");
    let c = atom("C");
    let d = atom("D");

    let p = Polynomial::sum(
        Polynomial::monomial(a.clone()),
        Polynomial::monomial(b.clone()),
    );
    let q = Polynomial::monomial(c.clone());
    let r = Polynomial::monomial(d.clone());

    let tape: Tape<TypeExpr, ExprGenerator> = left_distributor(&p, &q, &r);
    let (inputs, outputs) = tape.io_types().expect("expected io types");

    let expected_inputs = vec![
        a.clone(),
        c.clone(),
        a.clone(),
        d.clone(),
        b.clone(),
        c.clone(),
        b.clone(),
        d.clone(),
    ];
    let expected_outputs = vec![
        a.clone(),
        c.clone(),
        b.clone(),
        c.clone(),
        a.clone(),
        d.clone(),
        b,
        d,
    ];

    assert_eq!(inputs, expected_inputs);
    assert_eq!(outputs, expected_outputs);
}
