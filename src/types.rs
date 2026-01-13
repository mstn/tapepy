use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeVar(pub usize);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeExpr {
    Bool,
    Unit,
    Int,
    Float,
    Var(TypeVar),
    Lub(Box<TypeExpr>, Box<TypeExpr>),
}

impl TypeExpr {
    pub fn lub(left: TypeExpr, right: TypeExpr) -> TypeExpr {
        if left == right {
            return left;
        }

        match (&left, &right) {
            (TypeExpr::Bool, TypeExpr::Bool) => TypeExpr::Bool,
            (TypeExpr::Unit, TypeExpr::Unit) => TypeExpr::Unit,
            (TypeExpr::Int, TypeExpr::Float) | (TypeExpr::Float, TypeExpr::Int) => TypeExpr::Float,
            (TypeExpr::Int, TypeExpr::Int) => TypeExpr::Int,
            (TypeExpr::Float, TypeExpr::Float) => TypeExpr::Float,
            _ => TypeExpr::Lub(Box::new(left), Box::new(right)),
        }
    }
}

impl fmt::Display for TypeExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeExpr::Bool => write!(f, "Bool"),
            TypeExpr::Unit => write!(f, "1"),
            TypeExpr::Int => write!(f, "Int"),
            TypeExpr::Float => write!(f, "Float"),
            TypeExpr::Var(TypeVar(id)) => write!(f, "a{}", id),
            TypeExpr::Lub(left, right) => write!(f, "lub({}, {})", left, right),
        }
    }
}
