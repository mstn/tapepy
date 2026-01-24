use tapepy::expression_circuit::ExprGenerator;
use tapepy::tape_language::circuit::Circuit;
use tapepy::tape_language::{Monomial, Tape};
use tapepy::types::TypeExpr;

#[test]
fn left_whisk_builds_expected_structure_and_types() {
    let u_sort = TypeExpr::Named("U".to_string());
    let uprime_sort = TypeExpr::Named("Uprime".to_string());
    let vprime_sort = TypeExpr::Named("Vprime".to_string());
    let wprime_sort = vprime_sort.clone();
    let zprime_sort = TypeExpr::Named("Zprime".to_string());

    let u = Monomial::atom(u_sort.clone());
    let vprime = Monomial::atom(vprime_sort.clone());

    let c_gen = ExprGenerator::function("c", vec![uprime_sort.clone()], vec![wprime_sort.clone()]);
    let d_gen = ExprGenerator::function("d", vec![wprime_sort.clone()], vec![wprime_sort.clone()]);
    let e_gen = ExprGenerator::function("e", vec![wprime_sort.clone()], vec![zprime_sort.clone()]);

    let c_tape = Tape::EmbedCircuit(Box::new(Circuit::Generator(c_gen.clone())));
    let id_tape = Tape::Id(vprime.clone());
    let sum1 = Tape::Sum(Box::new(c_tape), Box::new(id_tape));
    let merge = Tape::Merge(wprime_sort.clone().into());
    let d_tape = Tape::EmbedCircuit(Box::new(Circuit::Generator(d_gen.clone())));
    let split = Tape::Split(wprime_sort.clone().into());
    let id_tape_2 = Tape::Id(wprime_sort.clone().into());
    let e_tape = Tape::EmbedCircuit(Box::new(Circuit::Generator(e_gen.clone())));
    let sum2 = Tape::Sum(Box::new(id_tape_2), Box::new(e_tape));

    let tape = Tape::Seq(
        Box::new(sum1),
        Box::new(Tape::Seq(
            Box::new(merge),
            Box::new(Tape::Seq(
                Box::new(d_tape),
                Box::new(Tape::Seq(Box::new(split), Box::new(sum2))),
            )),
        )),
    );

    let actual = tape.left_whisk(&u);

    let id_u = Circuit::Id(u_sort.clone());
    let c_whisk = Tape::EmbedCircuit(Box::new(Circuit::product(
        id_u.clone(),
        Circuit::Generator(c_gen),
    )));
    let id_whisk = Tape::Id(Monomial::product(u.clone(), vprime.clone()));
    let sum1_whisk = Tape::Sum(Box::new(c_whisk), Box::new(id_whisk));
    let merge_whisk = Tape::Merge(Monomial::product(u.clone(), vprime.clone()));
    let d_whisk = Tape::EmbedCircuit(Box::new(Circuit::product(
        id_u.clone(),
        Circuit::Generator(d_gen),
    )));
    let split_whisk = Tape::Split(Monomial::product(u.clone(), vprime.clone()));
    let id_whisk_2 = Tape::Id(Monomial::product(u.clone(), vprime.clone()));
    let e_whisk = Tape::EmbedCircuit(Box::new(Circuit::product(id_u, Circuit::Generator(e_gen))));
    let sum2_whisk = Tape::Sum(Box::new(id_whisk_2), Box::new(e_whisk));

    let expected = Tape::Seq(
        Box::new(sum1_whisk),
        Box::new(Tape::Seq(
            Box::new(merge_whisk),
            Box::new(Tape::Seq(
                Box::new(d_whisk),
                Box::new(Tape::Seq(Box::new(split_whisk), Box::new(sum2_whisk))),
            )),
        )),
    );

    assert_eq!(actual, expected);

    let (input_labels, output_labels) = actual
        .io_types()
        .expect("expected left-whisked tape io types");

    let expected_inputs = vec![
        Monomial::atom(u_sort.clone()),
        Monomial::atom(uprime_sort.clone()),
        Monomial::atom(u_sort.clone()),
        Monomial::atom(vprime_sort.clone()),
    ];
    let expected_outputs = vec![
        Monomial::atom(u_sort.clone()),
        Monomial::atom(wprime_sort.clone()),
        Monomial::atom(u_sort),
        Monomial::atom(zprime_sort),
    ];

    assert_eq!(input_labels, expected_inputs);
    assert_eq!(output_labels, expected_outputs);
}
