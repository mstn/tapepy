use tapepy::tape_language::{Monomial, Polynomial};

#[test]
fn polynomial_product_single_terms_is_monomial_product() {
    let u = Monomial::atom(1);
    let v = Monomial::atom(2);
    let p = Polynomial::monomial(u.clone());
    let q = Polynomial::monomial(v.clone());

    let expected = Polynomial::monomial(Monomial::product(u, v));
    let actual = Polynomial::product(p, q);

    assert_eq!(actual, expected);
}
