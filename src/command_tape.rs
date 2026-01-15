use crate::command_typing::{CommandChild, CommandDerivationTree, CommandForm};
use crate::expression_circuit::{circuit_from_expr, ExprGenerator};
use crate::predicate_tape::tape_from_predicate;
use crate::tape_language::{Circuit, Monomial, Tape};
use crate::types::TypeExpr;

pub fn tape_from_command(tree: &CommandDerivationTree) -> Tape<TypeExpr, ExprGenerator> {
    match tree.form() {
        CommandForm::Abort => Tape::Discard(context_monomial(tree)),
        CommandForm::Skip => Tape::Id(context_monomial(tree)),
        CommandForm::Assign(_) => assignment_tape(tree),
        CommandForm::Seq => {
            let (left, right) = command_children(tree);
            let left_tape = tape_from_command(left);
            let right_tape = tape_from_command(right);
            Tape::Seq(Box::new(left_tape), Box::new(right_tape))
        }
        CommandForm::If => if_tape(tree),
        CommandForm::While => {
            panic!("while tapes not implemented yet");
        }
    }
}

fn assignment_tape(tree: &CommandDerivationTree) -> Tape<TypeExpr, ExprGenerator> {
    let expr = tree
        .children()
        .iter()
        .find_map(|child| match child {
            CommandChild::Expression(expr) => Some(expr),
            _ => None,
        })
        .unwrap_or_else(|| panic!("assignment expects expression child"));
    let circuit = circuit_from_expr(expr);
    // Context wiring for assignment is deferred; we only embed the RHS circuit here.
    Tape::EmbedCircuit(Box::new(circuit))
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

    let context = context_monomial(tree);
    let pred_tape = tape_from_predicate(pred);
    let then_tape = tape_from_command(then_branch);
    let else_tape = tape_from_command(else_branch);

    let not_gate = Tape::EmbedCircuit(Box::new(Circuit::Generator(ExprGenerator::new(
        "not", 1, 1,
    ))));
    let not_pred = Tape::Seq(Box::new(pred_tape.clone()), Box::new(not_gate));

    let left_guarded = Tape::Seq(Box::new(pred_tape), Box::new(Tape::Create(context.clone())));
    let right_guarded = Tape::Seq(Box::new(not_pred), Box::new(Tape::Create(context)));

    let left = Tape::Seq(Box::new(left_guarded), Box::new(then_tape));
    let right = Tape::Seq(Box::new(right_guarded), Box::new(else_tape));
    Tape::Sum(Box::new(left), Box::new(right))
}

fn command_children(tree: &CommandDerivationTree) -> (&CommandDerivationTree, &CommandDerivationTree) {
    let mut iter = tree.children().iter().filter_map(|child| match child {
        CommandChild::Command(cmd) => Some(cmd),
        _ => None,
    });
    let left = iter.next().expect("sequence expects left command");
    let right = iter.next().expect("sequence expects right command");
    (left, right)
}

fn context_monomial(tree: &CommandDerivationTree) -> Monomial<TypeExpr> {
    let entries = tree.judgment().context().entries();
    entries.iter().fold(Monomial::one(), |acc, (_, ty)| {
        Monomial::product(acc, Monomial::atom(ty.clone()))
    })
}
