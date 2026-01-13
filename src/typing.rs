use std::fmt;

use rustpython_parser::ast::{BoolOp, Constant, Expr, ExprCall, ExprName, Operator, UnaryOp};

use crate::context::Context;
use crate::types::TypeExpr;

#[derive(Debug, Clone)]
pub struct Judgment {
    context: ContextSnapshot,
    expr: String,
    ty: TypeExpr,
}

#[derive(Debug, Clone)]
pub struct ContextSnapshot(Vec<(String, TypeExpr)>);

#[derive(Debug, Clone)]
pub struct DeductionTree {
    rule: &'static str,
    judgment: Judgment,
    children: Vec<DeductionTree>,
    form: ExprForm,
}

impl fmt::Display for ContextSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return write!(f, "Context");
        }

        write!(f, "Context[")?;
        for (idx, (name, ty)) in self.0.iter().enumerate() {
            if idx > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}: {}", name, ty)?;
        }
        write!(f, "]")
    }
}

impl fmt::Display for Judgment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} |- {} : {}", self.context, self.expr, self.ty)
    }
}

impl fmt::Display for DeductionTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_with_indent(f, 0)
    }
}

impl DeductionTree {
    pub fn judgment(&self) -> &Judgment {
        &self.judgment
    }

    pub fn children(&self) -> &[DeductionTree] {
        &self.children
    }

    pub fn rule(&self) -> &'static str {
        self.rule
    }

    pub fn form(&self) -> &ExprForm {
        &self.form
    }

    fn fmt_with_indent(&self, f: &mut fmt::Formatter<'_>, indent: usize) -> fmt::Result {
        for _ in 0..indent {
            write!(f, "  ")?;
        }
        writeln!(f, "[{}] {}", self.rule, self.judgment)?;
        for child in &self.children {
            child.fmt_with_indent(f, indent + 1)?;
        }
        Ok(())
    }
}

pub fn infer_expression(expr: &Expr) -> DeductionTree {
    let mut context = Context::default();
    collect_free_vars(expr, &mut context);
    infer_expr(expr, &context)
}

fn infer_expr(expr: &Expr, context: &Context) -> DeductionTree {
    match expr {
        Expr::Name(ExprName { id, .. }) => {
            let ty = context
                .get(id.as_str())
                .cloned()
                .unwrap_or_else(|| panic!("missing type variable for `{}`", id));
            make_leaf(
                "Var",
                expr,
                context,
                ty,
                ExprForm::Var(id.to_string()),
            )
        }
        Expr::Constant(c) => {
            let ty = match &c.value {
                Constant::Bool(_) => TypeExpr::Bool,
                Constant::Int(_) => TypeExpr::Int,
                Constant::Float(_) => TypeExpr::Float,
                _ => panic!("unsupported literal in expression: {:?}", c.value),
            };
            make_leaf(
                "Const",
                expr,
                context,
                ty,
                ExprForm::Const(const_label(c)),
            )
        }
        Expr::UnaryOp(unary) => match unary.op {
            UnaryOp::UAdd | UnaryOp::USub => {
                let child = infer_expr(&unary.operand, context);
                let ty = child.judgment.ty.clone();
                make_node(
                    "UnaryOp",
                    expr,
                    context,
                    ty,
                    vec![child],
                    ExprForm::UnaryOp(unary_op_label(unary.op)),
                )
            }
            UnaryOp::Not => {
                let child = infer_expr(&unary.operand, context);
                let ty = TypeExpr::Bool;
                make_node(
                    "UnaryOp",
                    expr,
                    context,
                    ty,
                    vec![child],
                    ExprForm::UnaryOp(unary_op_label(unary.op)),
                )
            }
            _ => panic!("unsupported unary operator in expression: {:?}", unary.op),
        },
        Expr::BinOp(bin) => {
            if !is_numeric_binop(&bin.op) {
                panic!("unsupported binary operator in expression: {:?}", bin.op);
            }
            let left = infer_expr(&bin.left, context);
            let right = infer_expr(&bin.right, context);
            let ty = TypeExpr::lub(left.judgment.ty.clone(), right.judgment.ty.clone());
            make_node(
                "BinOp",
                expr,
                context,
                ty,
                vec![left, right],
                ExprForm::BinOp(binop_label(&bin.op)),
            )
        }
        Expr::Call(call) => infer_call(expr, call, context),
        Expr::BoolOp(bool_op) => {
            let mut children = Vec::with_capacity(bool_op.values.len());
            for value in &bool_op.values {
                children.push(infer_expr(value, context));
            }
            let label = match bool_op.op {
                BoolOp::And => "and".to_string(),
                BoolOp::Or => "or".to_string(),
            };
            make_node(
                "BoolOp",
                expr,
                context,
                TypeExpr::Bool,
                children,
                ExprForm::BoolOp(label),
            )
        }
        _ => panic!("unsupported Python expression: {:?}", expr),
    }
}

fn infer_call(expr: &Expr, call: &ExprCall, context: &Context) -> DeductionTree {
    if !call.keywords.is_empty() {
        panic!("keyword arguments are not supported");
    }

    let (name, arg) = match call.func.as_ref() {
        Expr::Name(ExprName { id, .. }) => {
            if call.args.len() != 1 {
                panic!("function `{}` is unary; got {} args", id, call.args.len());
            }
            (id.as_str().to_string(), &call.args[0])
        }
        _ => panic!("unsupported call target in expression: {:?}", call.func),
    };

    let builtin = builtin_fn(&name).unwrap_or_else(|| panic!("unsupported function `{}`", name));
    let child = infer_expr(arg, context);
    let ty = match builtin {
        BuiltinFn::Fixed(fixed) => fixed,
        BuiltinFn::SameAsArg => child.judgment.ty.clone(),
    };

    make_node(
        "Call",
        expr,
        context,
        ty,
        vec![child],
        ExprForm::Call(name),
    )
}

fn collect_free_vars(expr: &Expr, context: &mut Context) {
    match expr {
        Expr::Name(ExprName { id, .. }) => {
            context.get_or_insert_var(id.as_str());
        }
        Expr::UnaryOp(unary) => collect_free_vars(&unary.operand, context),
        Expr::BinOp(bin) => {
            collect_free_vars(&bin.left, context);
            collect_free_vars(&bin.right, context);
        }
        Expr::BoolOp(bool_op) => {
            for value in &bool_op.values {
                collect_free_vars(value, context);
            }
        }
        Expr::Call(call) => {
            if !call.keywords.is_empty() {
                panic!("keyword arguments are not supported");
            }
            match call.func.as_ref() {
                Expr::Name(ExprName { id, .. }) => {
                    if call.args.len() != 1 {
                        panic!("function `{}` is unary; got {} args", id, call.args.len());
                    }
                }
                _ => panic!("unsupported call target in expression: {:?}", call.func),
            }
            collect_free_vars(&call.args[0], context);
        }
        Expr::Constant(_) => {}
        _ => panic!("unsupported Python expression: {:?}", expr),
    }
}

fn make_leaf(
    rule: &'static str,
    expr: &Expr,
    context: &Context,
    ty: TypeExpr,
    form: ExprForm,
) -> DeductionTree {
    DeductionTree {
        rule,
        judgment: Judgment {
            context: ContextSnapshot(context.entries()),
            expr: expr_to_string(expr),
            ty,
        },
        children: Vec::new(),
        form,
    }
}

fn make_node(
    rule: &'static str,
    expr: &Expr,
    context: &Context,
    ty: TypeExpr,
    children: Vec<DeductionTree>,
    form: ExprForm,
) -> DeductionTree {
    DeductionTree {
        rule,
        judgment: Judgment {
            context: ContextSnapshot(context.entries()),
            expr: expr_to_string(expr),
            ty,
        },
        children,
        form,
    }
}

fn is_numeric_binop(op: &Operator) -> bool {
    matches!(
        op,
        Operator::Add
            | Operator::Sub
            | Operator::Mult
            | Operator::Div
            | Operator::Mod
            | Operator::Pow
            | Operator::FloorDiv
    )
}

enum BuiltinFn {
    Fixed(TypeExpr),
    SameAsArg,
}

fn builtin_fn(name: &str) -> Option<BuiltinFn> {
    match name {
        "bool" => Some(BuiltinFn::Fixed(TypeExpr::Bool)),
        "int" => Some(BuiltinFn::Fixed(TypeExpr::Int)),
        "float" => Some(BuiltinFn::Fixed(TypeExpr::Float)),
        "abs" => Some(BuiltinFn::SameAsArg),
        "bit_length" => Some(BuiltinFn::Fixed(TypeExpr::Int)),
        _ => None,
    }
}

fn expr_to_string(expr: &Expr) -> String {
    match expr {
        Expr::Name(ExprName { id, .. }) => id.to_string(),
        Expr::Constant(c) => match &c.value {
            Constant::Bool(value) => value.to_string(),
            Constant::Int(value) => value.to_string(),
            Constant::Float(value) => value.to_string(),
            _ => format!("{:?}", expr),
        },
        Expr::UnaryOp(unary) => {
            let op = match unary.op {
                UnaryOp::UAdd => "+",
                UnaryOp::USub => "-",
                UnaryOp::Not => "not ",
                _ => "?",
            };
            format!("{}{}", op, expr_to_string(&unary.operand))
        }
        Expr::BinOp(bin) => {
            let op = match bin.op {
                Operator::Add => "+",
                Operator::Sub => "-",
                Operator::Mult => "*",
                Operator::Div => "/",
                Operator::Mod => "%",
                Operator::Pow => "**",
                Operator::FloorDiv => "//",
                _ => "?",
            };
            format!(
                "({} {} {})",
                expr_to_string(&bin.left),
                op,
                expr_to_string(&bin.right)
            )
        }
        Expr::Call(call) => {
            let name = match call.func.as_ref() {
                Expr::Name(ExprName { id, .. }) => id.to_string(),
                _ => "<call>".to_string(),
            };
            if call.args.len() == 1 {
                format!("{}({})", name, expr_to_string(&call.args[0]))
            } else {
                format!("{}(...)", name)
            }
        }
        Expr::BoolOp(bool_op) => {
            let op = match bool_op.op {
                BoolOp::And => "and",
                BoolOp::Or => "or",
            };
            let mut parts = bool_op
                .values
                .iter()
                .map(expr_to_string)
                .collect::<Vec<_>>();
            if parts.is_empty() {
                return "(<empty boolop>)".to_string();
            }
            let mut expr = parts.remove(0);
            for part in parts {
                expr = format!("({} {} {})", expr, op, part);
            }
            expr
        }
        _ => format!("{:?}", expr),
    }
}

fn const_label(c: &rustpython_parser::ast::ExprConstant) -> String {
    match &c.value {
        Constant::Bool(value) => value.to_string(),
        Constant::Int(value) => value.to_string(),
        Constant::Float(value) => value.to_string(),
        _ => format!("{:?}", c.value),
    }
}

fn unary_op_label(op: UnaryOp) -> String {
    match op {
        UnaryOp::UAdd => "pos".to_string(),
        UnaryOp::USub => "neg".to_string(),
        UnaryOp::Not => "not".to_string(),
        _ => "unary".to_string(),
    }
}

fn binop_label(op: &Operator) -> String {
    match op {
        Operator::Add => "+".to_string(),
        Operator::Sub => "-".to_string(),
        Operator::Mult => "*".to_string(),
        Operator::Div => "/".to_string(),
        Operator::Mod => "%".to_string(),
        Operator::Pow => "**".to_string(),
        Operator::FloorDiv => "//".to_string(),
        _ => "?".to_string(),
    }
}

#[derive(Debug, Clone)]
pub enum ExprForm {
    Var(String),
    Const(String),
    UnaryOp(String),
    BinOp(String),
    Call(String),
    BoolOp(String),
}

impl Judgment {
    pub fn context(&self) -> &ContextSnapshot {
        &self.context
    }

    pub fn ty(&self) -> &TypeExpr {
        &self.ty
    }

    pub fn expr(&self) -> &str {
        &self.expr
    }
}

impl ContextSnapshot {
    pub fn entries(&self) -> &[(String, TypeExpr)] {
        &self.0
    }
}
