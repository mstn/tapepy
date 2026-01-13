use std::fmt;

use rustpython_parser::ast::{Expr, ExprName, Stmt, StmtAssign, StmtIf, StmtWhile};

use crate::context::Context;
use crate::types::TypeExpr;
use crate::typing::{infer_expression_in_context, infer_predicate_in_context, ContextSnapshot, DeductionTree};

#[derive(Debug, Clone)]
pub struct CommandJudgment {
    context: ContextSnapshot,
    command: String,
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
    infer_command(stmt, &context)
}

pub fn infer_command(stmt: &Stmt, context: &Context) -> CommandDerivationTree {
    match stmt {
        Stmt::Pass(_) => make_leaf("Skip", context, "skip".to_string()),
        Stmt::Raise(_) => make_leaf("Abort", context, "abort".to_string()),
        Stmt::Assign(assign) => infer_assign(assign, context),
        Stmt::If(if_stmt) => infer_if(if_stmt, context),
        Stmt::While(while_stmt) => infer_while(while_stmt, context),
        Stmt::Expr(expr_stmt) => {
            panic!("unsupported command expression: {:?}", expr_stmt)
        }
        _ => panic!("unsupported statement for commands: {:?}", stmt),
    }
}

fn infer_assign(assign: &StmtAssign, context: &Context) -> CommandDerivationTree {
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
    if expr_tree.judgment().ty() != &target_ty {
        panic!(
            "type error: cannot assign {} to {}:{}",
            expr_tree.judgment().ty(),
            target_name,
            target_ty
        );
    }

    let cmd = format!("{} := {}", target_name, expr_tree.judgment().expr());
    make_node(
        "Assign",
        context,
        cmd,
        vec![CommandChild::Expression(expr_tree)],
    )
}

fn infer_if(if_stmt: &StmtIf, context: &Context) -> CommandDerivationTree {
    let pred_tree = infer_predicate_in_context(&if_stmt.test, context);
    if pred_tree.judgment().ty() != &TypeExpr::Unit {
        panic!("type error: if predicate must have type 1");
    }

    let then_tree = infer_block(&if_stmt.body, context);
    let else_tree = infer_block(&if_stmt.orelse, context);
    let cmd = format!("if {} then ... else ...", pred_tree.judgment().expr());
    make_node(
        "If",
        context,
        cmd,
        vec![
            CommandChild::Predicate(pred_tree),
            CommandChild::Command(then_tree),
            CommandChild::Command(else_tree),
        ],
    )
}

fn infer_while(while_stmt: &StmtWhile, context: &Context) -> CommandDerivationTree {
    let pred_tree = infer_predicate_in_context(&while_stmt.test, context);
    if pred_tree.judgment().ty() != &TypeExpr::Unit {
        panic!("type error: while predicate must have type 1");
    }

    let body_tree = infer_block(&while_stmt.body, context);
    let cmd = format!("while {} do ...", pred_tree.judgment().expr());
    make_node(
        "While",
        context,
        cmd,
        vec![CommandChild::Predicate(pred_tree), CommandChild::Command(body_tree)],
    )
}

fn infer_block(stmts: &[Stmt], context: &Context) -> CommandDerivationTree {
    if stmts.is_empty() {
        return make_leaf("Skip", context, "skip".to_string());
    }

    let mut iter = stmts.iter();
    let first = infer_command(iter.next().unwrap(), context);
    iter.fold(first, |acc, stmt| {
        let next = infer_command(stmt, context);
        make_node(
            "Seq",
            context,
            format!("{}; ...", acc.judgment.command),
            vec![CommandChild::Command(acc), CommandChild::Command(next)],
        )
    })
}

fn make_leaf(rule: &'static str, context: &Context, command: String) -> CommandDerivationTree {
    CommandDerivationTree {
        rule,
        judgment: CommandJudgment {
            context: ContextSnapshot(context.entries()),
            command,
        },
        children: Vec::new(),
    }
}

fn make_node(
    rule: &'static str,
    context: &Context,
    command: String,
    children: Vec<CommandChild>,
) -> CommandDerivationTree {
    CommandDerivationTree {
        rule,
        judgment: CommandJudgment {
            context: ContextSnapshot(context.entries()),
            command,
        },
        children,
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
