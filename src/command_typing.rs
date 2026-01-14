use std::fmt;

use rustpython_parser::ast::{Expr, ExprName, Stmt, StmtAssign, StmtIf, StmtWhile};

use crate::context::Context;
use crate::types::TypeExpr;
use crate::typing::{
    infer_expression_in_context, infer_predicate_in_context, ContextSnapshot, DeductionTree,
};

#[derive(Debug, Clone)]
pub struct CommandJudgment {
    context: ContextSnapshot,
    command: String,
}

impl CommandJudgment {
    pub fn context(&self) -> &ContextSnapshot {
        &self.context
    }

    pub fn command(&self) -> &str {
        &self.command
    }
}

#[derive(Debug, Clone)]
pub enum CommandChild {
    Command(CommandDerivationTree),
    Predicate(DeductionTree),
    Expression(DeductionTree),
}

#[derive(Debug, Clone)]
pub struct CommandDerivationTree {
    rule: &'static str,
    judgment: CommandJudgment,
    children: Vec<CommandChild>,
    form: CommandForm,
}

#[derive(Debug, Clone)]
pub enum CommandForm {
    Abort,
    Skip,
    Assign(String),
    Seq,
    If,
    While,
}

impl fmt::Display for CommandJudgment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} |- {}", self.context, self.command)
    }
}

impl fmt::Display for CommandDerivationTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_with_indent(f, 0)
    }
}

impl CommandDerivationTree {
    pub fn form(&self) -> &CommandForm {
        &self.form
    }

    pub fn judgment(&self) -> &CommandJudgment {
        &self.judgment
    }

    pub fn children(&self) -> &[CommandChild] {
        &self.children
    }

    fn fmt_with_indent(&self, f: &mut fmt::Formatter<'_>, indent: usize) -> fmt::Result {
        for _ in 0..indent {
            write!(f, "  ")?;
        }
        writeln!(f, "[{}] {}", self.rule, self.judgment)?;
        for child in &self.children {
            match child {
                CommandChild::Command(cmd) => cmd.fmt_with_indent(f, indent + 1)?,
                CommandChild::Predicate(pred) | CommandChild::Expression(pred) => {
                    let pred_str = format!("{}", pred);
                    for line in pred_str.lines() {
                        for _ in 0..(indent + 1) {
                            write!(f, "  ")?;
                        }
                        writeln!(f, "{}", line)?;
                    }
                }
            }
        }
        Ok(())
    }
}

pub fn infer_command_from_stmt(stmt: &Stmt) -> CommandDerivationTree {
    let mut context = Context::default();
    collect_free_vars_stmt(stmt, &mut context);
    infer_command(stmt, &context).0
}

pub fn infer_command_from_suite(stmts: &[Stmt]) -> CommandDerivationTree {
    let mut context = Context::default();
    for stmt in stmts {
        collect_free_vars_stmt(stmt, &mut context);
    }
    infer_block(stmts, &context).0
}

pub fn infer_command(stmt: &Stmt, context: &Context) -> (CommandDerivationTree, Context) {
    match stmt {
        Stmt::Pass(_) => (
            make_leaf("Skip", context, "skip".to_string(), CommandForm::Skip),
            context.clone(),
        ),
        Stmt::Raise(_) => (
            make_leaf("Abort", context, "abort".to_string(), CommandForm::Abort),
            context.clone(),
        ),
        Stmt::Assign(assign) => infer_assign(assign, context),
        Stmt::If(if_stmt) => infer_if(if_stmt, context),
        Stmt::While(while_stmt) => infer_while(while_stmt, context),
        Stmt::Expr(expr_stmt) => {
            panic!("unsupported command expression: {:?}", expr_stmt)
        }
        _ => panic!("unsupported statement for commands: {:?}", stmt),
    }
}

fn infer_assign(assign: &StmtAssign, context: &Context) -> (CommandDerivationTree, Context) {
    if assign.targets.len() != 1 {
        panic!("assignment expects a single target");
    }
    let target_name = match &assign.targets[0] {
        Expr::Name(ExprName { id, .. }) => id.as_str().to_string(),
        _ => panic!("assignment target must be a variable name"),
    };

    let target_ty = context
        .get(&target_name)
        .cloned()
        .unwrap_or_else(|| panic!("assignment target `{}` not in context", target_name));

    let expr_tree = infer_expression_in_context(&assign.value, context);
    let new_target_ty = if expr_tree.judgment().ty() == &target_ty {
        target_ty.clone()
    } else {
        TypeExpr::lub(target_ty.clone(), expr_tree.judgment().ty().clone())
    };
    let mut updated_context = context.clone();
    updated_context.set_var(&target_name, new_target_ty);

    let cmd = format!("{} := {}", target_name, expr_tree.judgment().expr());
    (
        make_node(
            "Assign",
            &updated_context,
            cmd,
            vec![CommandChild::Expression(expr_tree)],
            CommandForm::Assign(target_name),
        ),
        updated_context,
    )
}

fn infer_if(if_stmt: &StmtIf, context: &Context) -> (CommandDerivationTree, Context) {
    let pred_tree = infer_predicate_in_context(&if_stmt.test, context);
    if pred_tree.judgment().ty() != &TypeExpr::Unit {
        panic!("type error: if predicate must have type 1");
    }

    let then_tree = infer_block(&if_stmt.body, context).0;
    let else_tree = infer_block(&if_stmt.orelse, context).0;
    let cmd = format!("if {} then ... else ...", pred_tree.judgment().expr());
    (
        make_node(
            "If",
            context,
            cmd,
            vec![
                CommandChild::Predicate(pred_tree),
                CommandChild::Command(then_tree),
                CommandChild::Command(else_tree),
            ],
            CommandForm::If,
        ),
        context.clone(),
    )
}

fn infer_while(while_stmt: &StmtWhile, context: &Context) -> (CommandDerivationTree, Context) {
    let pred_tree = infer_predicate_in_context(&while_stmt.test, context);
    if pred_tree.judgment().ty() != &TypeExpr::Unit {
        panic!("type error: while predicate must have type 1");
    }

    let body_tree = infer_block(&while_stmt.body, context).0;
    let cmd = format!("while {} do ...", pred_tree.judgment().expr());
    (
        make_node(
            "While",
            context,
            cmd,
            vec![
                CommandChild::Predicate(pred_tree),
                CommandChild::Command(body_tree),
            ],
            CommandForm::While,
        ),
        context.clone(),
    )
}

fn infer_block(stmts: &[Stmt], context: &Context) -> (CommandDerivationTree, Context) {
    if stmts.is_empty() {
        return (
            make_leaf("Skip", context, "skip".to_string(), CommandForm::Skip),
            context.clone(),
        );
    }

    let mut iter = stmts.iter();
    let (mut acc_tree, mut acc_context) = infer_command(iter.next().unwrap(), context);
    for stmt in iter {
        let (next_tree, next_context) = infer_command(stmt, &acc_context);
        acc_tree = make_node(
            "Seq",
            context,
            format!("{}; ...", acc_tree.judgment.command),
            vec![CommandChild::Command(acc_tree), CommandChild::Command(next_tree)],
            CommandForm::Seq,
        );
        acc_context = next_context;
    }
    (acc_tree, acc_context)
}

fn make_leaf(
    rule: &'static str,
    context: &Context,
    command: String,
    form: CommandForm,
) -> CommandDerivationTree {
    CommandDerivationTree {
        rule,
        judgment: CommandJudgment {
            context: ContextSnapshot::new(context.entries()),
            command,
        },
        children: Vec::new(),
        form,
    }
}

fn make_node(
    rule: &'static str,
    context: &Context,
    command: String,
    children: Vec<CommandChild>,
    form: CommandForm,
) -> CommandDerivationTree {
    CommandDerivationTree {
        rule,
        judgment: CommandJudgment {
            context: ContextSnapshot::new(context.entries()),
            command,
        },
        children,
        form,
    }
}

fn collect_free_vars_stmt(stmt: &Stmt, context: &mut Context) {
    match stmt {
        Stmt::Pass(_) | Stmt::Raise(_) => {}
        Stmt::Assign(assign) => {
            for target in &assign.targets {
                if let Expr::Name(ExprName { id, .. }) = target {
                    context.get_or_insert_var(id.as_str());
                }
            }
            collect_free_vars_expr(&assign.value, context);
        }
        Stmt::If(if_stmt) => {
            collect_free_vars_expr(&if_stmt.test, context);
            for stmt in &if_stmt.body {
                collect_free_vars_stmt(stmt, context);
            }
            for stmt in &if_stmt.orelse {
                collect_free_vars_stmt(stmt, context);
            }
        }
        Stmt::While(while_stmt) => {
            collect_free_vars_expr(&while_stmt.test, context);
            for stmt in &while_stmt.body {
                collect_free_vars_stmt(stmt, context);
            }
        }
        _ => {}
    }
}

fn collect_free_vars_expr(expr: &Expr, context: &mut Context) {
    match expr {
        Expr::Name(ExprName { id, .. }) => {
            context.get_or_insert_var(id.as_str());
        }
        Expr::UnaryOp(unary) => collect_free_vars_expr(&unary.operand, context),
        Expr::BinOp(bin) => {
            collect_free_vars_expr(&bin.left, context);
            collect_free_vars_expr(&bin.right, context);
        }
        Expr::BoolOp(bool_op) => {
            for value in &bool_op.values {
                collect_free_vars_expr(value, context);
            }
        }
        Expr::Call(call) => {
            for arg in &call.args {
                collect_free_vars_expr(arg, context);
            }
        }
        Expr::Constant(_) => {}
        _ => {}
    }
}
