use tapepy::expression_circuit::ExprGenerator;
use tapepy::tape_language::circuit::Circuit;
use tapepy::tape_language::{
    inverse_left_distributor, left_distributor, Monomial, Polynomial, Tape, Whisker,
};
use tapepy::types::TypeExpr;

#[test]
fn left_whisk_builds_expected_structure_and_types() {
    let u_sort = TypeExpr::Named("U".to_string());
    let uprime_sort = TypeExpr::Named("Uprime".to_string());
    let vprime_sort = TypeExpr::Named("Vprime".to_string());
    let wprime_sort = TypeExpr::Named("Wprime".to_string());
    let zprime_sort = TypeExpr::Named("Zprime".to_string());

    let u = Monomial::atom(u_sort.clone());
    let vprime = Monomial::atom(vprime_sort.clone());

    let c_gen = ExprGenerator::function("c", vec![uprime_sort.clone()], vec![vprime_sort.clone()]);
    let d_gen = ExprGenerator::function("d", vec![vprime_sort.clone()], vec![wprime_sort.clone()]);
    let e_gen = ExprGenerator::function("e", vec![wprime_sort.clone()], vec![zprime_sort.clone()]);

    let c_tape = Tape::EmbedCircuit(Box::new(Circuit::Generator(c_gen.clone())));
    let id_tape = Tape::Id(vprime.clone());
    let sum1 = Tape::Sum(Box::new(c_tape), Box::new(id_tape));
    let merge = Tape::Merge(vprime_sort.clone().into());
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
    let merge_whisk = Tape::Merge(Monomial::product(u.clone(), vprime_sort.clone().into()));
    let d_whisk = Tape::EmbedCircuit(Box::new(Circuit::product(
        id_u.clone(),
        Circuit::Generator(d_gen),
    )));
    let split_whisk = Tape::Split(Monomial::product(u.clone(), wprime_sort.clone().into()));
    let id_whisk_2 = Tape::Id(Monomial::product(u.clone(), wprime_sort.clone().into()));
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

#[test]
fn right_whisk_builds_expected_structure_and_types() {
    let u_sort = TypeExpr::Named("U".to_string());
    let v_sort = TypeExpr::Named("V".to_string());
    let z_sort = TypeExpr::Named("Z".to_string());
    let w_sort = TypeExpr::Named("W".to_string());
    let wprime_sort = TypeExpr::Named("Wprime".to_string());

    let wprime = Monomial::atom(wprime_sort.clone());
    let u = Monomial::atom(u_sort.clone());
    let v = Monomial::atom(v_sort.clone());

    let c_gen = ExprGenerator::function("c", vec![v_sort.clone()], vec![u_sort.clone()]);
    let d_gen = ExprGenerator::function("d", vec![u_sort.clone()], vec![z_sort.clone()]);
    let e_gen = ExprGenerator::function("e", vec![z_sort.clone()], vec![w_sort.clone()]);

    let id_tape = Tape::Id(u.clone());
    let c_tape = Tape::EmbedCircuit(Box::new(Circuit::Generator(c_gen.clone())));
    let sum1 = Tape::Sum(Box::new(id_tape), Box::new(c_tape));
    let merge = Tape::Merge(u_sort.clone().into());
    let d_tape = Tape::EmbedCircuit(Box::new(Circuit::Generator(d_gen.clone())));
    let split = Tape::Split(z_sort.clone().into());
    let e_tape = Tape::EmbedCircuit(Box::new(Circuit::Generator(e_gen.clone())));
    let id_tape_2 = Tape::Id(z_sort.clone().into());
    let sum2 = Tape::Sum(Box::new(e_tape), Box::new(id_tape_2));

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

    let actual = tape.right_whisk(&wprime);

    let id_wprime = Circuit::Id(wprime_sort.clone());
    let id_whisk = Tape::Id(Monomial::product(u.clone(), wprime.clone()));
    let c_whisk = Tape::EmbedCircuit(Box::new(Circuit::product(
        Circuit::Generator(c_gen),
        id_wprime.clone(),
    )));
    let sum1_whisk = Tape::Sum(Box::new(id_whisk), Box::new(c_whisk));
    let merge_whisk = Tape::Merge(Monomial::product(u.clone(), wprime.clone()));
    let d_whisk = Tape::EmbedCircuit(Box::new(Circuit::product(
        Circuit::Generator(d_gen),
        id_wprime.clone(),
    )));
    let split_whisk = Tape::Split(Monomial::product(z_sort.clone().into(), wprime.clone()));
    let e_whisk = Tape::EmbedCircuit(Box::new(Circuit::product(
        Circuit::Generator(e_gen),
        id_wprime,
    )));
    let id_whisk_2 = Tape::Id(Monomial::product(z_sort.clone().into(), wprime.clone()));
    let sum2_whisk = Tape::Sum(Box::new(e_whisk), Box::new(id_whisk_2));

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
        .expect("expected right-whisked tape io types");

    let expected_inputs = vec![
        Monomial::atom(u_sort.clone()),
        Monomial::atom(wprime_sort.clone()),
        Monomial::atom(v_sort),
        Monomial::atom(wprime_sort.clone()),
    ];
    let expected_outputs = vec![
        Monomial::atom(w_sort.clone()),
        Monomial::atom(wprime_sort.clone()),
        Monomial::atom(z_sort),
        Monomial::atom(wprime_sort.clone()),
    ];

    assert_eq!(input_labels, expected_inputs);
    assert_eq!(output_labels, expected_outputs);
}

#[test]
fn tape_product_whisk_builds_expected_tape() {
    let u_sort = TypeExpr::Named("U".to_string());
    let v_sort = TypeExpr::Named("V".to_string());
    let z_sort = TypeExpr::Named("Z".to_string());
    let w_sort = TypeExpr::Named("W".to_string());

    let uprime_sort = TypeExpr::Named("Uprime".to_string());
    let vprime_sort = TypeExpr::Named("Vprime".to_string());
    let wprime_sort = TypeExpr::Named("Wprime".to_string());
    let zprime_sort = TypeExpr::Named("Zprime".to_string());

    let u = Monomial::atom(u_sort.clone());
    let v = Monomial::atom(v_sort.clone());
    let w = Monomial::atom(w_sort.clone());
    let z = Monomial::atom(z_sort.clone());
    let vprime = Monomial::atom(vprime_sort.clone());
    let wprime = Monomial::atom(wprime_sort.clone());
    let zprime = Monomial::atom(zprime_sort.clone());

    let c1 = ExprGenerator::function("c", vec![v_sort.clone()], vec![u_sort.clone()]);
    let d1 = ExprGenerator::function("d", vec![u_sort.clone()], vec![z_sort.clone()]);
    let e1 = ExprGenerator::function("e", vec![z_sort.clone()], vec![w_sort.clone()]);

    let c2 = ExprGenerator::function("c'", vec![uprime_sort.clone()], vec![vprime_sort.clone()]);
    let d2 = ExprGenerator::function("d'", vec![vprime_sort.clone()], vec![wprime_sort.clone()]);
    let e2 = ExprGenerator::function("e'", vec![wprime_sort.clone()], vec![zprime_sort.clone()]);

    let t1 = {
        let id_tape = Tape::Id(u.clone());
        let c_tape = Tape::EmbedCircuit(Box::new(Circuit::Generator(c1.clone())));
        let sum1 = Tape::Sum(Box::new(id_tape), Box::new(c_tape));
        let merge = Tape::Merge(u_sort.clone().into());
        let d_tape = Tape::EmbedCircuit(Box::new(Circuit::Generator(d1.clone())));
        let split = Tape::Split(z_sort.clone().into());
        let e_tape = Tape::EmbedCircuit(Box::new(Circuit::Generator(e1.clone())));
        let id_tape_2 = Tape::Id(z_sort.clone().into());
        let sum2 = Tape::Sum(Box::new(e_tape), Box::new(id_tape_2));

        Tape::Seq(
            Box::new(sum1),
            Box::new(Tape::Seq(
                Box::new(merge),
                Box::new(Tape::Seq(
                    Box::new(d_tape),
                    Box::new(Tape::Seq(Box::new(split), Box::new(sum2))),
                )),
            )),
        )
    };

    let t2 = {
        let c_tape = Tape::EmbedCircuit(Box::new(Circuit::Generator(c2.clone())));
        let id_tape = Tape::Id(vprime.clone());
        let sum1 = Tape::Sum(Box::new(c_tape), Box::new(id_tape));
        let merge = Tape::Merge(vprime_sort.clone().into());
        let d_tape = Tape::EmbedCircuit(Box::new(Circuit::Generator(d2.clone())));
        let split = Tape::Split(wprime_sort.clone().into());
        let id_tape_2 = Tape::Id(wprime_sort.clone().into());
        let e_tape = Tape::EmbedCircuit(Box::new(Circuit::Generator(e2.clone())));
        let sum2 = Tape::Sum(Box::new(id_tape_2), Box::new(e_tape));

        Tape::Seq(
            Box::new(sum1),
            Box::new(Tape::Seq(
                Box::new(merge),
                Box::new(Tape::Seq(
                    Box::new(d_tape),
                    Box::new(Tape::Seq(Box::new(split), Box::new(sum2))),
                )),
            )),
        )
    };

    let actual = Tape::product_whisk(&t1, &t2);

    let left_whisk_u_t2 = {
        let id_u = Circuit::Id(u_sort.clone());
        let c_whisk = Tape::EmbedCircuit(Box::new(Circuit::product(
            id_u.clone(),
            Circuit::Generator(c2.clone()),
        )));
        let id_whisk = Tape::Id(Monomial::product(u.clone(), vprime.clone()));
        let sum1_whisk = Tape::Sum(Box::new(c_whisk), Box::new(id_whisk));
        let merge_whisk = Tape::Merge(Monomial::product(u.clone(), vprime.clone()));
        let d_whisk = Tape::EmbedCircuit(Box::new(Circuit::product(
            id_u.clone(),
            Circuit::Generator(d2.clone()),
        )));
        let split_whisk = Tape::Split(Monomial::product(u.clone(), wprime.clone()));
        let id_whisk_2 = Tape::Id(Monomial::product(u.clone(), wprime.clone()));
        let e_whisk = Tape::EmbedCircuit(Box::new(Circuit::product(
            id_u,
            Circuit::Generator(e2.clone()),
        )));
        let sum2_whisk = Tape::Sum(Box::new(id_whisk_2), Box::new(e_whisk));

        Tape::Seq(
            Box::new(sum1_whisk),
            Box::new(Tape::Seq(
                Box::new(merge_whisk),
                Box::new(Tape::Seq(
                    Box::new(d_whisk),
                    Box::new(Tape::Seq(Box::new(split_whisk), Box::new(sum2_whisk))),
                )),
            )),
        )
    };

    let left_whisk_v_t2 = {
        let id_v = Circuit::Id(v_sort.clone());
        let c_whisk = Tape::EmbedCircuit(Box::new(Circuit::product(
            id_v.clone(),
            Circuit::Generator(c2.clone()),
        )));
        let id_whisk = Tape::Id(Monomial::product(v.clone(), vprime.clone()));
        let sum1_whisk = Tape::Sum(Box::new(c_whisk), Box::new(id_whisk));
        let merge_whisk = Tape::Merge(Monomial::product(v.clone(), vprime.clone()));
        let d_whisk = Tape::EmbedCircuit(Box::new(Circuit::product(
            id_v.clone(),
            Circuit::Generator(d2.clone()),
        )));
        let split_whisk = Tape::Split(Monomial::product(v.clone(), wprime.clone()));
        let id_whisk_2 = Tape::Id(Monomial::product(v.clone(), wprime.clone()));
        let e_whisk = Tape::EmbedCircuit(Box::new(Circuit::product(
            id_v,
            Circuit::Generator(e2.clone()),
        )));
        let sum2_whisk = Tape::Sum(Box::new(id_whisk_2), Box::new(e_whisk));

        Tape::Seq(
            Box::new(sum1_whisk),
            Box::new(Tape::Seq(
                Box::new(merge_whisk),
                Box::new(Tape::Seq(
                    Box::new(d_whisk),
                    Box::new(Tape::Seq(Box::new(split_whisk), Box::new(sum2_whisk))),
                )),
            )),
        )
    };

    let left_whisk_p1_t2 = Tape::Sum(Box::new(left_whisk_u_t2), Box::new(left_whisk_v_t2));

    let right_whisk_wprime_t1 = {
        let id_wprime = Circuit::Id(wprime_sort.clone());
        let id_whisk = Tape::Id(Monomial::product(u.clone(), wprime.clone()));
        let c_whisk = Tape::EmbedCircuit(Box::new(Circuit::product(
            Circuit::Generator(c1.clone()),
            id_wprime.clone(),
        )));
        let sum1_whisk = Tape::Sum(Box::new(id_whisk), Box::new(c_whisk));
        let merge_whisk = Tape::Merge(Monomial::product(u.clone(), wprime.clone()));
        let d_whisk = Tape::EmbedCircuit(Box::new(Circuit::product(
            Circuit::Generator(d1.clone()),
            id_wprime.clone(),
        )));
        let split_whisk = Tape::Split(Monomial::product(z.clone(), wprime.clone()));
        let e_whisk = Tape::EmbedCircuit(Box::new(Circuit::product(
            Circuit::Generator(e1.clone()),
            id_wprime,
        )));
        let id_whisk_2 = Tape::Id(Monomial::product(z.clone(), wprime.clone()));
        let sum2_whisk = Tape::Sum(Box::new(e_whisk), Box::new(id_whisk_2));

        Tape::Seq(
            Box::new(sum1_whisk),
            Box::new(Tape::Seq(
                Box::new(merge_whisk),
                Box::new(Tape::Seq(
                    Box::new(d_whisk),
                    Box::new(Tape::Seq(Box::new(split_whisk), Box::new(sum2_whisk))),
                )),
            )),
        )
    };

    let right_whisk_zprime_t1 = {
        let id_zprime = Circuit::Id(zprime_sort.clone());
        let id_whisk = Tape::Id(Monomial::product(u.clone(), zprime.clone()));
        let c_whisk = Tape::EmbedCircuit(Box::new(Circuit::product(
            Circuit::Generator(c1.clone()),
            id_zprime.clone(),
        )));
        let sum1_whisk = Tape::Sum(Box::new(id_whisk), Box::new(c_whisk));
        let merge_whisk = Tape::Merge(Monomial::product(u.clone(), zprime.clone()));
        let d_whisk = Tape::EmbedCircuit(Box::new(Circuit::product(
            Circuit::Generator(d1.clone()),
            id_zprime.clone(),
        )));
        let split_whisk = Tape::Split(Monomial::product(z.clone(), zprime.clone()));
        let e_whisk = Tape::EmbedCircuit(Box::new(Circuit::product(
            Circuit::Generator(e1.clone()),
            id_zprime,
        )));
        let id_whisk_2 = Tape::Id(Monomial::product(z.clone(), zprime.clone()));
        let sum2_whisk = Tape::Sum(Box::new(e_whisk), Box::new(id_whisk_2));

        Tape::Seq(
            Box::new(sum1_whisk),
            Box::new(Tape::Seq(
                Box::new(merge_whisk),
                Box::new(Tape::Seq(
                    Box::new(d_whisk),
                    Box::new(Tape::Seq(Box::new(split_whisk), Box::new(sum2_whisk))),
                )),
            )),
        )
    };

    let p1 = Polynomial::sum(
        Polynomial::monomial(u.clone()),
        Polynomial::monomial(v.clone()),
    );
    let q1 = Polynomial::sum(
        Polynomial::monomial(w.clone()),
        Polynomial::monomial(z.clone()),
    );

    let right_whisk_q2_t1 = Tape::Seq(
        Box::new(left_distributor(
            &p1,
            &Polynomial::monomial(wprime.clone()),
            &Polynomial::monomial(zprime.clone()),
        )),
        Box::new(Tape::Seq(
            Box::new(Tape::Sum(
                Box::new(right_whisk_wprime_t1),
                Box::new(right_whisk_zprime_t1),
            )),
            Box::new(inverse_left_distributor(
                &q1,
                &Polynomial::monomial(wprime),
                &Polynomial::monomial(zprime),
            )),
        )),
    );

    let expected = Tape::Seq(Box::new(left_whisk_p1_t2), Box::new(right_whisk_q2_t1));

    assert_eq!(actual, expected);
}
