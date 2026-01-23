use std::fmt;

use open_hypergraphs::lax::OpenHypergraph;

use crate::types::TypeExpr;

#[derive(Debug, Clone, PartialEq)]
pub enum CommandEdge {
    Atom(String),
    Embedded(Box<OpenHypergraph<TypeExpr, CommandEdge>>),
}

impl fmt::Display for CommandEdge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandEdge::Atom(label) => write!(f, "{}", label),
            CommandEdge::Embedded(child) => {
                write!(
                    f,
                    "Embedded({}x{})",
                    child.sources.len(),
                    child.targets.len()
                )
            }
        }
    }
}
