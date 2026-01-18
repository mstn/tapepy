mod command_dot;
mod command_edge;
mod command_hypergraph;
mod command_tape;
mod command_typing;
mod context;
mod expression_circuit;
mod hypergraph;
mod predicate_tape;
mod python_builtin_signatures;
mod solver;
mod tape_language;
mod types;
mod typing;

use std::error::Error;

use command_dot::{generate_dot_with_embedded_clusters, to_svg_with_embedded_clusters};
use command_edge::CommandEdge;
use command_tape::tape_from_command;
use command_typing::infer_command_from_suite;
use graphviz_rust::printer::{DotPrinter, PrinterContext};
use open_hypergraphs::lax::OpenHypergraph;
use open_hypergraphs_dot::{generate_dot_with, Options};
use rustpython_parser::{ast, Parse};
use solver::{apply_substitution, solve_hypergraph_types};

fn main() -> Result<(), Box<dyn Error>> {
    let input = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    let source = if input.is_empty() {
        "\nif True:\n  x = 1\nelse:\n  x = 2"
    } else {
        &input
    };

    let suite = match ast::Suite::parse(source, "<input>") {
        Ok(suite) => suite,
        Err(err) => {
            eprintln!("Parse error: {}", err);
            return Ok(());
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

    let opts = Options {
        node_label: Box::new(|mono: &tape_language::Monomial<types::TypeExpr>| mono.to_string()),
        edge_label: Box::new(|e: &CommandEdge| e.to_string()),
        ..Options::default()
    };

    write_svg_with_fallback("./out", &term, &opts)?;
    let strict_term = term.map_edges(|edge| {
        let subst = solve_hypergraph_types(&edge)
            .unwrap_or_else(|err| panic!("type solving failed for embedded circuit: {}", err));
        let solved = apply_substitution(&edge, &subst);
        OpenHypergraph::from_strict(solved.to_strict())
    });
    // let strict = strict_term.to_strict();
    // let strict_lax = OpenHypergraph::from_strict(strict);
    // write_svg_with_fallback("./out_strict", &strict_lax, &opts)?;

    // Type solving is only available for graphs with TypeExpr node labels.
    Ok(())
}

fn write_svg_with_fallback<O: Clone, E: Clone + std::fmt::Display>(
    prefix: &str,
    graph: &OpenHypergraph<O, OpenHypergraph<types::TypeExpr, E>>,
    opts: &Options<O, CommandEdge>,
) -> Result<(), Box<dyn Error>> {
    let svg_path = format!("{}.svg", prefix);
    let dot_path = format!("{}.dot", prefix);
    let dot_graph = generate_dot_with_embedded_clusters(graph, opts);
    let mut ctx = PrinterContext::default();
    let dot_string = dot_graph.print(&mut ctx);
    std::fs::write(&dot_path, dot_string)?;
    match to_svg_with_embedded_clusters(graph, opts) {
        Ok(svg) => {
            std::fs::write(svg_path, svg)?;
        }
        Err(err) => {
            eprintln!(
                "SVG rendering failed ({}). Wrote DOT output to {}.dot.",
                err, prefix
            );
        }
    }
    Ok(())
}
