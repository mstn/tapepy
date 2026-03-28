use std::error::Error;
use std::path::Path;

use graphviz_rust::printer::{DotPrinter, PrinterContext};
use open_hypergraphs::lax::OpenHypergraph;
use open_hypergraphs_dot::Options;
use rustpython_parser::{ast, Parse};

use crate::cli::args::CompileArgs;
use crate::cli::home::HomeFolderLayout;
use crate::cli::output::{yaml_string, OutputFormat};
use crate::command_dot::{generate_dot_with_clusters, CommandEdge};
use crate::command_tape::tape_from_command;
use crate::command_typing::{collect_constraints, infer_command_from_suite};
use crate::program_tape::{apply_substitution_to_monomial, solve_program_tape_with_subst};
use crate::solver::TypeSubstitution;
use crate::tape_language::{self, MonomialTape};
use crate::types;

pub fn run(home: &HomeFolderLayout, args: &CompileArgs) -> Result<(), Box<dyn Error>> {
    let source = std::fs::read_to_string(&args.filepath)?;
    if args.language.to_lowercase() != "python" {
        return Err(format!("unsupported language `{}`", args.language).into());
    }

    let run_id = home.create_run()?;
    let run_options = CompileRunOptions {
        input_file: &args.filepath,
        language: &args.language,
        raw_tape: args.raw_tape,
        output_format: OutputFormat::Dot,
    };
    write_compile_run_metadata(home, &run_id, &run_options, &source)?;

    let suite = match ast::Suite::parse(&source, args.filepath.to_string_lossy().as_ref()) {
        Ok(suite) => suite,
        Err(err) => {
            return Err(format!("Parse error: {}", err).into());
        }
    };

    let tree = infer_command_from_suite(&suite);
    let tape = tape_from_command(&tree);
    let monomial_tape = MonomialTape::try_from_tape(tape.clone())
        .map_err(|err| format!("monomial tape conversion failed: {:?}", err))?;
    let mut next_id = 0usize;
    let term = tape.to_hypergraph(&mut || {
        let id = next_id;
        next_id += 1;
        types::TypeExpr::Var(types::TypeVar(id))
    });
    let mut flat_next_id = 0usize;
    let flat_term = tape.to_flat_hypergraph(&mut || {
        let id = flat_next_id;
        flat_next_id += 1;
        types::TypeExpr::Var(types::TypeVar(id))
    });

    let (subst, flat_graph) = if args.raw_tape {
        (None, tape_language::simplify_flat_plus_id(flat_term))
    } else {
        let constraints = collect_constraints(&tree);
        let (_solved, subst) = solve_program_tape_with_subst(&term, constraints.constraints());
        let flat_solved = flat_term
            .map_nodes(|mono| apply_substitution_to_monomial(&mono, &subst))
            .map_edges(|edge| match edge {
                tape_language::FlatTapeEdge::Atom(generator) => {
                    tape_language::FlatTapeEdge::Atom(generator.clone())
                }
                tape_language::FlatTapeEdge::Plus => tape_language::FlatTapeEdge::Plus,
            });
        let flat_solved = OpenHypergraph::from_strict(flat_solved.to_strict());
        (Some(subst), tape_language::simplify_flat_plus_id(flat_solved))
    };

    let mut mono_next_id = 0usize;
    let monomial_graph = monomial_tape.to_hypergraph(&mut || {
        let id = mono_next_id;
        mono_next_id += 1;
        types::TypeExpr::Var(types::TypeVar(id))
    });
    let _monomial_graph = if let Some(subst) = &subst {
        let monomial_graph = monomial_graph
            .map_nodes(|node| apply_substitution_to_monomial_node(&node, subst))
            .map_edges(|edge| apply_substitution_to_monomial_edge(&edge, subst));
        OpenHypergraph::from_strict(monomial_graph.to_strict())
    } else {
        monomial_graph
    };

    let opts = Options {
        node_label: Box::new(|mono: &tape_language::Monomial<types::TypeExpr>| mono.to_string()),
        edge_label: Box::new(|edge: &CommandEdge| edge.to_string()),
        ..Options::default()
    };

    write_flat_dot(&flat_graph, &opts, &home.ir_path(&run_id))?;
    println!("{}", home.run_dir(&run_id).display());
    Ok(())
}

struct CompileRunOptions<'a> {
    input_file: &'a Path,
    language: &'a str,
    raw_tape: bool,
    output_format: OutputFormat,
}

fn write_compile_run_metadata(
    home: &HomeFolderLayout,
    run_id: &str,
    options: &CompileRunOptions<'_>,
    source: &str,
) -> Result<(), Box<dyn Error>> {
    std::fs::write(home.source_path(run_id), source)?;
    std::fs::write(home.options_path(run_id), compile_options_yaml(options))?;
    Ok(())
}

fn compile_options_yaml(options: &CompileRunOptions<'_>) -> String {
    format!(
        "command: compile\ninput_file: {}\nlanguage: {}\nraw_tape: {}\noutput_format: {}\n",
        yaml_string(&options.input_file.display().to_string()),
        yaml_string(options.language),
        options.raw_tape,
        yaml_string(options.output_format.as_str()),
    )
}

fn write_flat_dot<E: Clone + std::fmt::Display>(
    graph: &OpenHypergraph<
        tape_language::Monomial<types::TypeExpr>,
        tape_language::FlatTapeEdge<E>,
    >,
    opts: &Options<tape_language::Monomial<types::TypeExpr>, CommandEdge>,
    output: &Path,
) -> Result<(), Box<dyn Error>> {
    let flat_graph = graph
        .clone()
        .map_edges(|edge| CommandEdge::Atom(edge.to_string()));
    let dot_graph = generate_dot_with_clusters(&flat_graph, opts);
    let mut ctx = PrinterContext::default();
    let dot_string = dot_graph.print(&mut ctx);
    std::fs::write(output, dot_string)?;
    Ok(())
}

fn apply_substitution_to_monomial_node(
    node: &tape_language::MonomialHyperNode<types::TypeExpr>,
    subst: &TypeSubstitution,
) -> tape_language::MonomialHyperNode<types::TypeExpr> {
    tape_language::MonomialHyperNode::new(
        node.tensor_kind,
        apply_substitution_to_monomial(&node.context, subst),
    )
}

fn apply_substitution_to_monomial_edge<E: Clone>(
    edge: &tape_language::MonomialTapeEdge<types::TypeExpr, E>,
    subst: &TypeSubstitution,
) -> tape_language::MonomialTapeEdge<types::TypeExpr, E> {
    match edge {
        tape_language::MonomialTapeEdge::Generator(generator) => {
            tape_language::MonomialTapeEdge::Generator(generator.clone())
        }
        tape_language::MonomialTapeEdge::CircuitCopy => tape_language::MonomialTapeEdge::CircuitCopy,
        tape_language::MonomialTapeEdge::CircuitDiscard => {
            tape_language::MonomialTapeEdge::CircuitDiscard
        }
        tape_language::MonomialTapeEdge::CircuitJoin => tape_language::MonomialTapeEdge::CircuitJoin,
        tape_language::MonomialTapeEdge::TapeDiscard => tape_language::MonomialTapeEdge::TapeDiscard,
        tape_language::MonomialTapeEdge::TapeSplit => tape_language::MonomialTapeEdge::TapeSplit,
        tape_language::MonomialTapeEdge::TapeCreate => tape_language::MonomialTapeEdge::TapeCreate,
        tape_language::MonomialTapeEdge::TapeMerge => tape_language::MonomialTapeEdge::TapeMerge,
        tape_language::MonomialTapeEdge::FromAddToMul(mono) => {
            tape_language::MonomialTapeEdge::FromAddToMul(apply_substitution_to_monomial(
                mono, subst,
            ))
        }
        tape_language::MonomialTapeEdge::FromMulToAdd(mono) => {
            tape_language::MonomialTapeEdge::FromMulToAdd(apply_substitution_to_monomial(
                mono, subst,
            ))
        }
    }
}
