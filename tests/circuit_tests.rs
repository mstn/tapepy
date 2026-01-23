use tapepy::tape_language::circuit::{
    identity_for_types, permute_circuit, product_many, wiring_circuit_for_context, Circuit,
    Permutation,
};

#[test]
fn identity_for_types_empty_is_id_one() {
    let circuit: Circuit<char, ()> = identity_for_types(&[]);
    assert_eq!(circuit, Circuit::IdOne);
}

#[test]
fn identity_for_types_multiple_builds_products() {
    let circuit: Circuit<char, ()> = identity_for_types(&['a', 'b']);
    let expected = Circuit::Product(Box::new(Circuit::Id('a')), Box::new(Circuit::Id('b')));
    assert_eq!(circuit, expected);
}

#[test]
fn product_many_handles_empty_and_single() {
    let empty: Circuit<char, ()> = product_many(Vec::new());
    assert_eq!(empty, Circuit::IdOne);

    let single: Circuit<char, ()> = product_many(vec![Circuit::Id('x')]);
    assert_eq!(single, Circuit::Id('x'));
}

#[test]
fn product_many_builds_nested_products() {
    let circuit: Circuit<char, ()> =
        product_many(vec![Circuit::Id('a'), Circuit::Id('b'), Circuit::Id('c')]);
    let expected = Circuit::Product(
        Box::new(Circuit::Product(
            Box::new(Circuit::Id('a')),
            Box::new(Circuit::Id('b')),
        )),
        Box::new(Circuit::Id('c')),
    );
    assert_eq!(circuit, expected);
}

#[test]
fn copy_wire_n_times_handles_base_cases() {
    let zero: Circuit<char, ()> = Circuit::copy_wire_n_times('a', 0);
    let one: Circuit<char, ()> = Circuit::copy_wire_n_times('a', 1);
    let two: Circuit<char, ()> = Circuit::copy_wire_n_times('a', 2);

    assert_eq!(zero, Circuit::Discard('a'));
    assert_eq!(one, Circuit::Id('a'));
    assert_eq!(two, Circuit::Copy('a'));
}

#[test]
fn copy_wire_n_times_expands_fanout() {
    let circuit: Circuit<char, ()> = Circuit::copy_wire_n_times('a', 3);
    let expected = Circuit::Seq(
        Box::new(Circuit::Copy('a')),
        Box::new(Circuit::Product(
            Box::new(Circuit::Id('a')),
            Box::new(Circuit::Copy('a')),
        )),
    );
    assert_eq!(circuit, expected);
}

#[test]
fn permute_circuit_identity_returns_identity() {
    let types = ['a', 'b'];
    let permutation = Permutation(vec![0, 1]);
    let circuit: Circuit<char, ()> = permute_circuit(&types, &permutation);
    let expected: Circuit<char, ()> = identity_for_types(&types);
    assert_eq!(circuit, expected);
}

#[test]
fn permute_circuit_builds_swap_sequence() {
    let types = ['a', 'b'];
    let permutation = Permutation(vec![1, 0]);
    let circuit: Circuit<char, ()> = permute_circuit(&types, &permutation);
    let expected = Circuit::Seq(
        Box::new(identity_for_types(&types)),
        Box::new(Circuit::Swap {
            left: 'a',
            right: 'b',
        }),
    );
    assert_eq!(circuit, expected);
}

#[test]
fn wiring_circuit_for_context_reorders_inputs() {
    let context_entries = vec![("x".to_string(), 'A'), ("y".to_string(), 'B')];
    let input_vars = vec!["y".to_string(), "x".to_string(), "x".to_string()];

    let circuit: Circuit<char, ()> = wiring_circuit_for_context(&context_entries, &input_vars);

    let grouped = Circuit::Product(Box::new(Circuit::Copy('A')), Box::new(Circuit::Id('B')));
    let perm = Permutation(vec![2, 0, 1]);
    let expected = Circuit::Seq(
        Box::new(grouped),
        Box::new(permute_circuit(&['A', 'A', 'B'], &perm)),
    );

    assert_eq!(circuit, expected);
}

#[test]
fn wiring_circuit_for_context_empty_inputs_discards_all() {
    let context_entries = vec![("x".to_string(), 'A')];
    let input_vars: Vec<String> = Vec::new();

    let circuit: Circuit<char, ()> = wiring_circuit_for_context(&context_entries, &input_vars);

    assert_eq!(circuit, Circuit::Discard('A'));
}
