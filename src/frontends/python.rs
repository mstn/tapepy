use rustpython_parser::{ast, Parse, ParseError};

use crate::command_typing::{infer_command_from_suite, CommandDerivationTree};
use crate::compiler::CompilerFrontend;

pub struct PythonCompiler;

impl CompilerFrontend for PythonCompiler {
    type Error = ParseError;

    fn parse(&self, source_name: &str, source: &str) -> Result<CommandDerivationTree, Self::Error> {
        let suite = ast::Suite::parse(source, source_name)?;
        Ok(infer_command_from_suite(&suite))
    }
}
