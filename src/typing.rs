use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};

use rustpython_parser::ast::{
    BoolOp, CmpOp, Constant, Expr, ExprCall, ExprCompare, ExprName, Operator, UnaryOp,
};

use crate::context::Context;
use crate::python_builtin_signatures::{
    builtin_type_signatures, Constraint, PyType, TypeScheme, TypeVar,
};
use crate::types::{TypeExpr, TypeVar as ExprTypeVar};

static NEXT_TYPE_VAR: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone)]
pub struct Judgment {
    context: ContextSnapshot,
    expr: String,
    ty: TypeExpr,
}

#[derive(Debug, Clone)]
pub struct ContextSnapshot(Vec<(String, TypeExpr)>);

impl ContextSnapshot {
    pub fn new(entries: Vec<(String, TypeExpr)>) -> Self {
        Self(entries)
    }
}

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

pub fn infer_expression_in_context(expr: &Expr, context: &Context) -> DeductionTree {
    infer_expr(expr, context)
}

pub fn infer_expression(expr: &Expr) -> DeductionTree {
    let mut context = Context::default();
    collect_free_vars(expr, &mut context);
    infer_expr(expr, &context)
}

pub fn infer_predicate_in_context(expr: &Expr, context: &Context) -> DeductionTree {
    infer_predicate_expr(expr, context)
}

pub fn infer_predicate(expr: &Expr) -> DeductionTree {
    let mut context = Context::default();
    collect_free_vars(expr, &mut context);
    infer_predicate_expr(expr, &context)
}

fn infer_expr(expr: &Expr, context: &Context) -> DeductionTree {
    match expr {
        Expr::Name(ExprName { id, .. }) => {
            let ty = context
                .get(id.as_str())
                .cloned()
                .unwrap_or_else(|| panic!("missing type variable for `{}`", id));
            make_leaf("Var", expr, context, ty, ExprForm::Var(id.to_string()))
        }
        Expr::Constant(c) => {
            let ty = match &c.value {
                Constant::Bool(_) => TypeExpr::Bool,
                Constant::Int(_) => TypeExpr::Int,
                Constant::Float(_) => TypeExpr::Float,
                _ => panic!("unsupported literal in expression: {:?}", c.value),
            };
            make_leaf("Const", expr, context, ty, ExprForm::Const(const_label(c)))
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
                if !is_potential_bool(&child.judgment.ty) {
                    panic!("type error: not expects Bool, got {}", child.judgment.ty);
                }
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
            let left = infer_expr(&bin.left, context);
            let right = infer_expr(&bin.right, context);
            let op = binop_label(&bin.op);
            let ty =
                resolve_builtin_output(&op, &[left.judgment.ty.clone(), right.judgment.ty.clone()]);
            make_node(
                "BinOp",
                expr,
                context,
                ty,
                vec![left, right],
                ExprForm::BinOp(op),
            )
        }
        Expr::Call(call) => infer_call(expr, call, context),
        Expr::BoolOp(bool_op) => {
            let mut children = Vec::with_capacity(bool_op.values.len());
            for value in &bool_op.values {
                let child = infer_expr(value, context);
                if !is_potential_bool(&child.judgment.ty) {
                    panic!(
                        "type error: boolean operator expects Bool, got {}",
                        child.judgment.ty
                    );
                }
                children.push(child);
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
        Expr::Compare(compare) => {
            if compare.ops.len() != 1 || compare.comparators.len() != 1 {
                panic!("chained comparisons are not supported in expressions");
            }
            let left = infer_expr(&compare.left, context);
            let right = infer_expr(&compare.comparators[0], context);
            let op_label = compare_op_label(&compare.ops[0]);
            let output =
                resolve_builtin_output(&op_label, &[left.judgment.ty.clone(), right.judgment.ty.clone()]);
            if !is_potential_bool(&output) {
                panic!(
                    "type error: comparison expects Bool result, got {}",
                    output
                );
            }
            make_node(
                "Compare",
                expr,
                context,
                TypeExpr::Bool,
                vec![left, right],
                ExprForm::Compare(op_label),
            )
        }
        _ => panic!("unsupported Python expression: {:?}", expr),
    }
}

fn infer_predicate_expr(expr: &Expr, context: &Context) -> DeductionTree {
    match expr {
        Expr::UnaryOp(unary) => match unary.op {
            UnaryOp::Not => {
                let child = infer_predicate_expr(&unary.operand, context);
                if child.judgment.ty != TypeExpr::Unit {
                    panic!(
                        "type error: predicate not expects 1, got {}",
                        child.judgment.ty
                    );
                }
                make_node(
                    "PredNot",
                    expr,
                    context,
                    TypeExpr::Unit,
                    vec![child],
                    ExprForm::UnaryOp(unary_op_label(unary.op)),
                )
            }
            _ => panic!("unsupported unary operator in predicate: {:?}", unary.op),
        },
        Expr::BoolOp(bool_op) => {
            let mut children = Vec::with_capacity(bool_op.values.len());
            for value in &bool_op.values {
                let child = infer_predicate_expr(value, context);
                if child.judgment.ty != TypeExpr::Unit {
                    panic!(
                        "type error: predicate operator expects 1, got {}",
                        child.judgment.ty
                    );
                }
                children.push(child);
            }
            let label = match bool_op.op {
                BoolOp::And => "and".to_string(),
                BoolOp::Or => "or".to_string(),
            };
            make_node(
                "PredBoolOp",
                expr,
                context,
                TypeExpr::Unit,
                children,
                ExprForm::BoolOp(label),
            )
        }
        Expr::Call(call) => infer_predicate_call(expr, call, context),
        Expr::Compare(compare) => infer_predicate_compare(expr, compare, context),
        Expr::Constant(c) => match &c.value {
            Constant::Bool(value) => make_leaf(
                "PredConst",
                expr,
                context,
                TypeExpr::Unit,
                ExprForm::Const(predicate_const_label(*value)),
            ),
            _ => panic!("unsupported predicate literal: {:?}", c.value),
        },
        _ => panic!("unsupported predicate expression: {:?}", expr),
    }
}

fn infer_predicate_compare(expr: &Expr, compare: &ExprCompare, context: &Context) -> DeductionTree {
    if compare.ops.len() != 1 || compare.comparators.len() != 1 {
        panic!("chained comparisons are not supported in predicates");
    }

    let left = infer_expression_in_context(&compare.left, context);
    let right = infer_expression_in_context(&compare.comparators[0], context);

    let op_label = compare_op_label(&compare.ops[0]);
    let output = resolve_builtin_output(
        &op_label,
        &[left.judgment.ty.clone(), right.judgment.ty.clone()],
    );
    if !is_potential_bool(&output) {
        panic!("type error: comparison expects Bool result, got {}", output);
    }
    make_node(
        "PredCompare",
        expr,
        context,
        TypeExpr::Unit,
        vec![left, right],
        ExprForm::Compare(op_label),
    )
}

fn infer_predicate_call(expr: &Expr, call: &ExprCall, context: &Context) -> DeductionTree {
    if !call.keywords.is_empty() {
        panic!("keyword arguments are not supported");
    }

    let name = match call.func.as_ref() {
        Expr::Name(ExprName { id, .. }) => id.as_str().to_string(),
        _ => panic!("unsupported call target in predicate: {:?}", call.func),
    };

    let mut children = Vec::with_capacity(call.args.len());
    let mut arg_types = Vec::with_capacity(call.args.len());
    for arg in &call.args {
        let child = infer_expr(arg, context);
        arg_types.push(child.judgment.ty.clone());
        children.push(child);
    }
    let output = resolve_builtin_output(&name, &arg_types);
    if !is_potential_bool(&output) {
        panic!(
            "predicate call expects Bool, got {} from `{}`",
            output, name
        );
    }
    make_node(
        "PredCall",
        expr,
        context,
        TypeExpr::Unit,
        children,
        ExprForm::Call(name),
    )
}

fn infer_call(expr: &Expr, call: &ExprCall, context: &Context) -> DeductionTree {
    if !call.keywords.is_empty() {
        panic!("keyword arguments are not supported");
    }

    let name = match call.func.as_ref() {
        Expr::Name(ExprName { id, .. }) => id.as_str().to_string(),
        _ => panic!("unsupported call target in expression: {:?}", call.func),
    };

    let mut children = Vec::with_capacity(call.args.len());
    let mut arg_types = Vec::with_capacity(call.args.len());
    for arg in &call.args {
        let child = infer_expr(arg, context);
        arg_types.push(child.judgment.ty.clone());
        children.push(child);
    }
    let ty = resolve_builtin_output(&name, &arg_types);
    make_node("Call", expr, context, ty, children, ExprForm::Call(name))
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
        Expr::Compare(compare) => {
            collect_free_vars(&compare.left, context);
            for value in &compare.comparators {
                collect_free_vars(value, context);
            }
        }
        Expr::Call(call) => {
            if !call.keywords.is_empty() {
                panic!("keyword arguments are not supported");
            }
            match call.func.as_ref() {
                Expr::Name(ExprName { .. }) => {}
                _ => panic!("unsupported call target in expression: {:?}", call.func),
            }
            for arg in &call.args {
                collect_free_vars(arg, context);
            }
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

fn resolve_builtin_output(name: &str, args: &[TypeExpr]) -> TypeExpr {
    let schemes = builtin_schemes(name, args.len());
    if schemes.is_empty() {
        panic!("unsupported builtin or operator `{}`", name);
    }

    let mut numeric_fallback = false;
    for scheme in schemes {
        if let Some(mapping) = match_scheme_inputs(args, &scheme) {
            if constraints_ok(&scheme, &mapping) {
                let mut mapping = mapping;
                return pytype_to_typeexpr(&scheme.output, &mut mapping);
            }
        }
        if is_numeric_scheme(&scheme) {
            numeric_fallback = true;
        }
    }

    if numeric_fallback && args.len() == 2 {
        let left = &args[0];
        let right = &args[1];
        if is_potential_numeric(left) && is_potential_numeric(right) {
            return TypeExpr::lub(left.clone(), right.clone());
        }
    }

    panic!(
        "no matching builtin signature for `{}` with {} args",
        name,
        args.len()
    );
}

fn builtin_schemes(name: &str, arity: usize) -> Vec<TypeScheme> {
    for builtin in builtin_type_signatures() {
        if builtin.name == name {
            return builtin
                .schemes
                .into_iter()
                .filter(|scheme| scheme.inputs.len() == arity)
                .collect();
        }
    }
    Vec::new()
}

fn match_scheme_inputs(
    args: &[TypeExpr],
    scheme: &TypeScheme,
) -> Option<HashMap<TypeVar, TypeExpr>> {
    if args.len() != scheme.inputs.len() {
        return None;
    }

    let mut mapping = HashMap::new();
    for (arg, expected) in args.iter().zip(scheme.inputs.iter()) {
        if !match_pytype(arg, expected, &mut mapping) {
            return None;
        }
    }
    Some(mapping)
}

fn match_pytype(
    arg: &TypeExpr,
    expected: &PyType,
    mapping: &mut HashMap<TypeVar, TypeExpr>,
) -> bool {
    match expected {
        PyType::Any => true,
        PyType::Var(var) => match mapping.get(var) {
            Some(existing) => match (existing, arg) {
                (TypeExpr::Var(_), _) => {
                    mapping.insert(var.clone(), arg.clone());
                    true
                }
                (_, TypeExpr::Var(_)) => true,
                _ => existing == arg,
            },
            None => {
                mapping.insert(var.clone(), arg.clone());
                true
            }
        },
        _ => {
            let expected_expr = pytype_to_typeexpr_label(expected, mapping);
            matches!(arg, TypeExpr::Var(_)) || *arg == expected_expr
        }
    }
}

fn constraints_ok(scheme: &TypeScheme, mapping: &HashMap<TypeVar, TypeExpr>) -> bool {
    for constraint in &scheme.constraints {
        match constraint {
            Constraint::Numeric(var) => {
                if let Some(ty) = mapping.get(var) {
                    if !is_potential_numeric(ty) {
                        return false;
                    }
                }
            }
            Constraint::Iterable(_) | Constraint::Mapping(_, _) | Constraint::Sequence(_) => {}
        }
    }
    true
}

fn is_numeric_scheme(scheme: &TypeScheme) -> bool {
    scheme
        .constraints
        .iter()
        .any(|constraint| matches!(constraint, Constraint::Numeric(_)))
}

fn pytype_to_typeexpr(pytype: &PyType, mapping: &mut HashMap<TypeVar, TypeExpr>) -> TypeExpr {
    match pytype {
        PyType::Bool => TypeExpr::Bool,
        PyType::Int => TypeExpr::Int,
        PyType::Float => TypeExpr::Float,
        PyType::NoneType => TypeExpr::Unit,
        PyType::Var(var) => mapping
            .entry(var.clone())
            .or_insert_with(fresh_type_var)
            .clone(),
        PyType::Any => TypeExpr::Named("Any".to_string()),
        _ => TypeExpr::Named(format_pytype(pytype, mapping)),
    }
}

fn pytype_to_typeexpr_label(pytype: &PyType, mapping: &HashMap<TypeVar, TypeExpr>) -> TypeExpr {
    match pytype {
        PyType::Bool => TypeExpr::Bool,
        PyType::Int => TypeExpr::Int,
        PyType::Float => TypeExpr::Float,
        PyType::NoneType => TypeExpr::Unit,
        PyType::Any => TypeExpr::Named("Any".to_string()),
        PyType::Var(var) => mapping
            .get(var)
            .cloned()
            .unwrap_or_else(|| TypeExpr::Named(var.0.to_string())),
        _ => TypeExpr::Named(format_pytype(pytype, mapping)),
    }
}

fn format_pytype(pytype: &PyType, mapping: &HashMap<TypeVar, TypeExpr>) -> String {
    match pytype {
        PyType::Any => "Any".to_string(),
        PyType::NoneType => "None".to_string(),
        PyType::Bool => "Bool".to_string(),
        PyType::Int => "Int".to_string(),
        PyType::Float => "Float".to_string(),
        PyType::Complex => "Complex".to_string(),
        PyType::Str => "Str".to_string(),
        PyType::Bytes => "Bytes".to_string(),
        PyType::ByteArray => "ByteArray".to_string(),
        PyType::Range => "Range".to_string(),
        PyType::Slice => "Slice".to_string(),
        PyType::MemoryView => "MemoryView".to_string(),
        PyType::Object => "Object".to_string(),
        PyType::Type => "Type".to_string(),
        PyType::Var(var) => mapping
            .get(var)
            .map(|ty| ty.to_string())
            .unwrap_or_else(|| var.0.to_string()),
        PyType::Iterable(inner) => format!("Iterable[{}]", format_pytype(inner, mapping)),
        PyType::Sequence(inner) => format!("Sequence[{}]", format_pytype(inner, mapping)),
        PyType::List(inner) => format!("List[{}]", format_pytype(inner, mapping)),
        PyType::Tuple(items) => {
            let parts = items
                .iter()
                .map(|item| format_pytype(item, mapping))
                .collect::<Vec<_>>();
            format!("Tuple[{}]", parts.join(", "))
        }
        PyType::TupleOf(inner) => format!("Tuple[{}]", format_pytype(inner, mapping)),
        PyType::Dict(key, value) => format!(
            "Dict[{}, {}]",
            format_pytype(key, mapping),
            format_pytype(value, mapping)
        ),
        PyType::Set(inner) => format!("Set[{}]", format_pytype(inner, mapping)),
        PyType::FrozenSet(inner) => format!("FrozenSet[{}]", format_pytype(inner, mapping)),
        PyType::Mapping(key, value) => format!(
            "Mapping[{}, {}]",
            format_pytype(key, mapping),
            format_pytype(value, mapping)
        ),
    }
}

fn fresh_type_var() -> TypeExpr {
    let id = NEXT_TYPE_VAR.fetch_add(1, Ordering::Relaxed);
    TypeExpr::Var(ExprTypeVar(id))
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
        Expr::Compare(compare) => {
            if compare.ops.len() != 1 || compare.comparators.len() != 1 {
                return format!("{:?}", expr);
            }
            let op = compare_op_display(&compare.ops[0]);
            format!(
                "({} {} {})",
                expr_to_string(&compare.left),
                op,
                expr_to_string(&compare.comparators[0])
            )
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

fn predicate_const_label(value: bool) -> String {
    if value {
        "Top".to_string()
    } else {
        "Bot".to_string()
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

fn compare_op_label(op: &CmpOp) -> String {
    match op {
        CmpOp::Lt => "<".to_string(),
        CmpOp::LtE => "<=".to_string(),
        CmpOp::Gt => ">".to_string(),
        CmpOp::GtE => ">=".to_string(),
        CmpOp::Eq => "==".to_string(),
        CmpOp::NotEq => "!=".to_string(),
        _ => "cmp".to_string(),
    }
}

fn compare_op_display(op: &CmpOp) -> &'static str {
    match op {
        CmpOp::Lt => "<",
        CmpOp::LtE => "<=",
        CmpOp::Gt => ">",
        CmpOp::GtE => ">=",
        CmpOp::Eq => "==",
        CmpOp::NotEq => "!=",
        _ => "?",
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
    Compare(String),
}

fn is_potential_bool(expr: &TypeExpr) -> bool {
    match expr {
        TypeExpr::Bool => true,
        TypeExpr::Unit => false,
        TypeExpr::Var(_) => true,
        TypeExpr::Lub(left, right) => is_potential_bool(left) && is_potential_bool(right),
        TypeExpr::Union(left, right) => is_potential_bool(left) && is_potential_bool(right),
        TypeExpr::Int | TypeExpr::Float | TypeExpr::Named(_) => false,
    }
}

fn is_potential_numeric(expr: &TypeExpr) -> bool {
    match expr {
        TypeExpr::Int | TypeExpr::Float => true,
        TypeExpr::Var(_) => true,
        TypeExpr::Lub(left, right) => is_potential_numeric(left) && is_potential_numeric(right),
        TypeExpr::Union(left, right) => is_potential_numeric(left) && is_potential_numeric(right),
        TypeExpr::Bool | TypeExpr::Unit | TypeExpr::Named(_) => false,
    }
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
