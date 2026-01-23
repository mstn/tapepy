use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeVar(pub usize);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeExpr {
    Bool,
    Unit,
    Int,
    Float,
    Named(String),
    Var(TypeVar),
    Union(Box<TypeExpr>, Box<TypeExpr>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeConstraint {
    Equal(TypeExpr, TypeExpr),
    Numeric(TypeExpr),
    Iterable(TypeExpr),
    Mapping(TypeExpr, TypeExpr),
    Sequence(TypeExpr),
}

impl fmt::Display for TypeExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeExpr::Bool => write!(f, "Bool"),
            TypeExpr::Unit => write!(f, "1"),
            TypeExpr::Int => write!(f, "Int"),
        TypeExpr::Float => write!(f, "Float"),
        TypeExpr::Named(name) => write!(f, "{}", name),
        TypeExpr::Var(TypeVar(id)) => write!(f, "a{}", id),
        TypeExpr::Union(left, right) => write!(f, "union({}, {})", left, right),
    }
}
}
