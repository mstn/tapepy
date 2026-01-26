mod command_dot;
mod command_tape;
mod command_typing;
mod context;
mod expression_circuit;
mod hypergraph_utils;
mod predicate_tape;
mod program_tape;
mod python_builtin_signatures;
mod solver;
mod tape_language;
mod types;
mod typing;

use std::error::Error;
use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use command_dot::{generate_dot_with_tape_clusters, to_svg_with_tape_clusters, CommandEdge};
use command_tape::tape_from_command;
use command_typing::{collect_constraints, infer_command_from_suite};
use graphviz_rust::printer::{DotPrinter, PrinterContext};
use open_hypergraphs::lax::OpenHypergraph;
use open_hypergraphs_dot::Options;
use program_tape::solve_and_strictify_program_tape;
use rustpython_parser::{ast, Parse};

#[derive(Parser)]
#[command(author, version, about = "Tapepy CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Parser)]
enum Command {
    /// Compile a source file into a tape hypergraph.
    Compile {
        /// Path to the source file.
        filepath: PathBuf,
        /// Source language (default: python).
        #[arg(long, default_value = "python")]
        language: String,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Svg)]
        format: OutputFormat,
        /// Output file path.
        #[arg(long)]
        output: PathBuf,
        /// Skip type solving and emit the raw tape hypergraph.
        #[arg(long)]
        raw_tape: bool,
    },
}

#[derive(ValueEnum, Clone, Copy)]
enum OutputFormat {
    Dot,
    Svg,
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    match cli.command {
        Command::Compile {
            filepath,
            language,
            format,
            output,
            raw_tape,
        } => compile_file(&filepath, &language, format, &output, raw_tape),
    }
}

fn compile_file(
    filepath: &PathBuf,
    language: &str,
    format: OutputFormat,
    output: &PathBuf,
    raw_tape: bool,
) -> Result<(), Box<dyn Error>> {
    let source = std::fs::read_to_string(filepath)?;
    if language.to_lowercase() != "python" {
        return Err(format!("unsupported language `{}`", language).into());
    }

    let suite = match ast::Suite::parse(&source, filepath.to_string_lossy().as_ref()) {
        Ok(suite) => suite,
        Err(err) => {
            return Err(format!("Parse error: {}", err).into());
        }
    };

    let tree = infer_command_from_suite(&suite);
    let tape = tape_from_command(&tree);
    let mut next_id = 0usize;
    let term = tape.to_hypergraph(&mut || {
        let id = next_id;
        next_id += 1;
        types::TypeExpr::Var(types::TypeVar(id))
    });

    let graph = if raw_tape {
        term
    } else {
        let constraints = collect_constraints(&tree);
        solve_and_strictify_program_tape(&term, constraints.constraints())
    };

    let opts = Options {
        node_label: Box::new(|mono: &tape_language::Monomial<types::TypeExpr>| mono.to_string()),
        edge_label: Box::new(|e: &CommandEdge| e.to_string()),
        ..Options::default()
    };

    match format {
        OutputFormat::Dot => write_dot(&graph, &opts, output),
        OutputFormat::Svg => write_svg(&graph, &opts, output),
    }
}

fn write_dot<E: Clone + std::fmt::Display>(
    graph: &OpenHypergraph<
        tape_language::Monomial<types::TypeExpr>,
        tape_language::TapeEdge<types::TypeExpr, E>,
    >,
    opts: &Options<tape_language::Monomial<types::TypeExpr>, CommandEdge>,
    output: &PathBuf,
) -> Result<(), Box<dyn Error>> {
    let dot_graph = generate_dot_with_tape_clusters(graph, opts);
    let mut ctx = PrinterContext::default();
    let dot_string = dot_graph.print(&mut ctx);
    std::fs::write(output, dot_string)?;
    Ok(())
}

fn write_svg<E: Clone + std::fmt::Display>(
    graph: &OpenHypergraph<
        tape_language::Monomial<types::TypeExpr>,
        tape_language::TapeEdge<types::TypeExpr, E>,
    >,
    opts: &Options<tape_language::Monomial<types::TypeExpr>, CommandEdge>,
    output: &PathBuf,
) -> Result<(), Box<dyn Error>> {
    let svg = to_svg_with_tape_clusters(graph, opts)?;
    std::fs::write(output, svg)?;
    Ok(())
}
