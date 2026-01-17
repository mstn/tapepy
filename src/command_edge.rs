use std::fmt;

use open_hypergraphs::lax::OpenHypergraph;

use crate::hypergraph::format_hypergraph;
use crate::types::TypeExpr;

#[derive(Debug, Clone, PartialEq)]
pub enum CommandEdge {
    Atom(String),
    Convolution(Vec<OpenHypergraph<TypeExpr, CommandEdge>>),
    Kleene(Box<OpenHypergraph<TypeExpr, CommandEdge>>),
    Embedded(Box<OpenHypergraph<TypeExpr, CommandEdge>>),
}

impl fmt::Display for CommandEdge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandEdge::Atom(label) => write!(f, "{}", label),
            CommandEdge::Convolution(children) => write!(
                f,
                "Convolution({})",
                children
                    .iter()
                    .map(|child| format!("{}x{}", child.sources.len(), child.targets.len()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            CommandEdge::Kleene(child) => {
                write!(f, "Kleene({}x{})", child.sources.len(), child.targets.len())
            }
            CommandEdge::Embedded(child) => {
                write!(f, "Embedded({}x{})", child.sources.len(), child.targets.len())
            }
        }
    }
}
