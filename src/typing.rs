use std::collections::BTreeMap;
use std::fmt;

use rustpython_parser::ast::{Constant, Expr, ExprCall, ExprName, Operator, UnaryOp};

use crate::context::Gamma;
use crate::types::TypeExpr;

#[derive(Debug, Clone)]
pub struct Judgment {
    gamma: GammaSnapshot,
    expr: String,
    ty: TypeExpr,
}

#[derive(Debug, Clone)]
struct GammaSnapshot(BTreeMap<String, TypeExpr>);

#[derive(Debug, Clone)]
pub struct DeductionTree {
    rule: &'static str,
    judgment: Judgment,
    children: Vec<DeductionTree>,
}

impl fmt::Display for GammaSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return write!(f, "Γ");
        }

        write!(f, "Γ[")?;
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
        write!(f, "{} |- {} : {}", self.gamma, self.expr, self.ty)
    }
}

impl fmt::Display for DeductionTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_with_indent(f, 0)
    }
}

impl DeductionTree {
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
    let mut gamma = Gamma::default();
    collect_free_vars(expr, &mut gamma);
    infer_expr(expr, &gamma)
}

fn infer_expr(expr: &Expr, gamma: &Gamma) -> DeductionTree {
    match expr {
        Expr::Name(ExprName { id, .. }) => {
            let ty = gamma
                .get(id.as_str())
                .cloned()
                .unwrap_or_else(|| panic!("missing type variable for `{}`", id));
            make_leaf("Var", expr, gamma, ty)
        }
        Expr::Constant(c) => {
            let ty = match &c.value {
                Constant::Int(_) => TypeExpr::Int,
                Constant::Float(_) => TypeExpr::Float,
                _ => panic!("unsupported literal in expression: {:?}", c.value),
            };
            make_leaf("Const", expr, gamma, ty)
        }
        Expr::UnaryOp(unary) => match unary.op {
            UnaryOp::UAdd | UnaryOp::USub => {
                let child = infer_expr(&unary.operand, gamma);
                let ty = child.judgment.ty.clone();
                make_node("UnaryOp", expr, gamma, ty, vec![child])
            }
            _ => panic!("unsupported unary operator in expression: {:?}", unary.op),
        },
        Expr::BinOp(bin) => {
            if !is_numeric_binop(&bin.op) {
                panic!("unsupported binary operator in expression: {:?}", bin.op);
            }
            let left = infer_expr(&bin.left, gamma);
            let right = infer_expr(&bin.right, gamma);
            let ty = TypeExpr::lub(left.judgment.ty.clone(), right.judgment.ty.clone());
            make_node("BinOp", expr, gamma, ty, vec![left, right])
        }
        Expr::Call(call) => infer_call(expr, call, gamma),
        _ => panic!("unsupported Python expression: {:?}", expr),
    }
}

fn infer_call(expr: &Expr, call: &ExprCall, gamma: &Gamma) -> DeductionTree {
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
    let child = infer_expr(arg, gamma);
    let ty = match builtin {
        BuiltinFn::Fixed(fixed) => fixed,
        BuiltinFn::SameAsArg => child.judgment.ty.clone(),
    };

    make_node("Call", expr, gamma, ty, vec![child])
}

fn collect_free_vars(expr: &Expr, gamma: &mut Gamma) {
    match expr {
        Expr::Name(ExprName { id, .. }) => {
            gamma.get_or_insert_var(id.as_str());
        }
        Expr::UnaryOp(unary) => collect_free_vars(&unary.operand, gamma),
        Expr::BinOp(bin) => {
            collect_free_vars(&bin.left, gamma);
            collect_free_vars(&bin.right, gamma);
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
            collect_free_vars(&call.args[0], gamma);
        }
        Expr::Constant(_) => {}
        _ => panic!("unsupported Python expression: {:?}", expr),
    }
}

fn make_leaf(rule: &'static str, expr: &Expr, gamma: &Gamma, ty: TypeExpr) -> DeductionTree {
    DeductionTree {
        rule,
        judgment: Judgment {
            gamma: GammaSnapshot(gamma.snapshot()),
            expr: expr_to_string(expr),
            ty,
        },
        children: Vec::new(),
    }
}

fn make_node(
    rule: &'static str,
    expr: &Expr,
    gamma: &Gamma,
    ty: TypeExpr,
    children: Vec<DeductionTree>,
) -> DeductionTree {
    DeductionTree {
        rule,
        judgment: Judgment {
            gamma: GammaSnapshot(gamma.snapshot()),
            expr: expr_to_string(expr),
            ty,
        },
        children,
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
        "int" => Some(BuiltinFn::Fixed(TypeExpr::Int)),
        "float" => Some(BuiltinFn::Fixed(TypeExpr::Float)),
        "abs" => Some(BuiltinFn::SameAsArg),
        _ => None,
    }
}

fn expr_to_string(expr: &Expr) -> String {
    match expr {
        Expr::Name(ExprName { id, .. }) => id.to_string(),
        Expr::Constant(c) => match &c.value {
            Constant::Int(value) => value.to_string(),
            Constant::Float(value) => value.to_string(),
            _ => format!("{:?}", expr),
        },
        Expr::UnaryOp(unary) => {
            let op = match unary.op {
                UnaryOp::UAdd => "+",
                UnaryOp::USub => "-",
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
        _ => format!("{:?}", expr),
    }
}
