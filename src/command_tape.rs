use crate::command_typing::{CommandChild, CommandDerivationTree, CommandForm};
use crate::expression_circuit::{self, ExprGenerator};
use crate::predicate_tape::{tape_from_predicate, tape_from_predicate_with_negation};
use crate::tape_language::{monomial_from_entries, Circuit, Monomial, Tape};
use crate::tape_language::tape::monomial_atoms;
use crate::types::TypeExpr;

pub fn tape_from_command(tree: &CommandDerivationTree) -> Tape<TypeExpr, ExprGenerator> {
    match tree.form() {
        CommandForm::Abort => Tape::Discard(context_monomial(tree)),
        CommandForm::Skip => Tape::Id(context_monomial(tree)),
        CommandForm::Assign(name) => assignment_tape(tree, name),
        CommandForm::Seq => {
            let mut iter = tree.children().iter().filter_map(|child| match child {
                CommandChild::Command(cmd) => Some(cmd),
                _ => None,
            });
            let left = iter.next().expect("sequence expects left command");
            let right = iter.next().expect("sequence expects right command");
            let left_tape = tape_from_command(left);
            let right_tape = tape_from_command(right);
            match (left_tape.clone(), right_tape.clone()) {
                (Tape::EmbedCircuit(circuit_left), Tape::EmbedCircuit(circuit_right)) => {
                    Tape::EmbedCircuit(Box::new(Circuit::Seq(circuit_left, circuit_right)))
                }
                _ => Tape::Seq(Box::new(left_tape), Box::new(right_tape)),
            }
        }
        CommandForm::If => if_tape(tree),
        CommandForm::While => {
            panic!("while tapes not implemented yet");
        }
    }
}

fn assignment_tape(tree: &CommandDerivationTree, name: &str) -> Tape<TypeExpr, ExprGenerator> {
    let context_entries = tree.judgment().context().entries();
    let index = context_entries
        .iter()
        .position(|(var, _)| var == name)
        .unwrap_or_else(|| panic!("assignment target `{}` not in context", name));

    let left_types: Vec<TypeExpr> = context_entries[0..index]
        .iter()
        .map(|(_, ty)| ty.clone())
        .collect();
    let right_types: Vec<TypeExpr> = context_entries[index + 1..]
        .iter()
        .map(|(_, ty)| ty.clone())
        .collect();
    let lhs_ty = context_entries[index].1.clone();
    let expr_tree = match tree.children().get(0) {
        Some(CommandChild::Expression(expr)) => expr,
        _ => panic!("assignment expects an expression child"),
    };
    let expr_circuit =
        expression_circuit::circuit_from_expr_with_context(expr_tree, context_entries);
    let left_id = Circuit::id(left_types.clone());
    let right_id = Circuit::id(right_types.clone());

    let left_copy = Circuit::copy_wires(left_types);
    let right_copy = Circuit::copy_wires(right_types);
    let split = Circuit::Product(
        Box::new(left_copy),
        Box::new(Circuit::Product(
            Box::new(Circuit::Id(lhs_ty)),
            Box::new(right_copy),
        )),
    );

    let updated = Circuit::Product(
        Box::new(left_id),
        Box::new(Circuit::Product(Box::new(expr_circuit), Box::new(right_id))),
    );

    let assign = Circuit::Seq(Box::new(split), Box::new(updated));

    Tape::EmbedCircuit(Box::new(assign))
}

fn if_tape(tree: &CommandDerivationTree) -> Tape<TypeExpr, ExprGenerator> {
    let mut pred = None;
    let mut then_branch = None;
    let mut else_branch = None;
    for child in tree.children() {
        match child {
            CommandChild::Predicate(pred_tree) => pred = Some(pred_tree),
            CommandChild::Command(cmd) => {
                if then_branch.is_none() {
                    then_branch = Some(cmd);
                } else {
                    else_branch = Some(cmd);
                }
            }
            _ => {}
        }
    }
    let pred = pred.expect("if expects predicate child");
    let then_branch = then_branch.expect("if expects then branch");
    let else_branch = else_branch.expect("if expects else branch");

    let context_entries = tree.judgment().context().entries();
    let context = monomial_from_entries(context_entries);
    let then_tape = tape_from_command(then_branch);
    let else_tape = tape_from_command(else_branch);
    let pred_tape = tape_from_predicate(pred);
    let neg_pred_tape = tape_from_predicate_with_negation(pred, true);

    let left = gate_tape(&context, pred_tape, then_tape);
    let right = gate_tape(&context, neg_pred_tape, else_tape);

    let context_types = monomial_atoms(&context)
        .into_iter()
        .map(|mono| match mono {
            Monomial::Atom(ty) => ty,
            Monomial::One | Monomial::Product(_, _) => {
                panic!("context monomial atoms must be flat")
            }
        })
        .collect::<Vec<_>>();
    let copy = Tape::EmbedCircuit(Box::new(Circuit::copy_wires(context_types.clone())));
    let join = Tape::EmbedCircuit(Box::new(Circuit::join_wires(context_types)));
    let branches = Tape::Sum(Box::new(left), Box::new(right));
    Tape::Seq(
        Box::new(copy),
        Box::new(Tape::Seq(Box::new(branches), Box::new(join))),
    )
}

fn gate_tape(
    context: &Monomial<TypeExpr>,
    pred_tape: Tape<TypeExpr, ExprGenerator>,
    exec_tape: Tape<TypeExpr, ExprGenerator>,
) -> Tape<TypeExpr, ExprGenerator> {
    let context_types = monomial_atoms(&context)
        .into_iter()
        .map(|mono| match mono {
            Monomial::Atom(ty) => ty,
            Monomial::One | Monomial::Product(_, _) => {
                panic!("context monomial atoms must be flat")
            }
        })
        .collect::<Vec<_>>();
    let copy = Tape::EmbedCircuit(Box::new(Circuit::copy_wires(context_types)));
    Tape::Seq(
        Box::new(copy),
        Box::new(Tape::Product(Box::new(pred_tape), Box::new(exec_tape))),
    )
}

fn context_monomial(tree: &CommandDerivationTree) -> Monomial<TypeExpr> {
    let entries = tree.judgment().context().entries();
    monomial_from_entries(entries)
}
