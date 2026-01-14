use std::fmt;

use open_hypergraphs::lax::OpenHypergraph;

use crate::hypergraph::format_hypergraph;
use crate::types::TypeExpr;

#[derive(Debug, Clone, PartialEq)]
pub enum CommandEdge {
    Atom(String),
    Convolution(Vec<OpenHypergraph<TypeExpr, CommandEdge>>),
    Kleene(Box<OpenHypergraph<TypeExpr, CommandEdge>>),
}

impl fmt::Display for CommandEdge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandEdge::Atom(label) => write!(f, "{}", label),
            CommandEdge::Convolution(children) => {
                writeln!(f, "Convolution({})", children.len())?;
                for (idx, child) in children.iter().enumerate() {
                    writeln!(f, "  [alt {}]", idx)?;
                    for line in format_hypergraph(child).lines() {
                        writeln!(f, "    {}", line)?;
                    }
                }
                Ok(())
            }
            CommandEdge::Kleene(child) => {
                writeln!(f, "Kleene")?;
                for line in format_hypergraph(child).lines() {
                    writeln!(f, "  {}", line)?;
                }
                Ok(())
            }
        }
    }
}
