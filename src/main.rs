mod command_dot;
mod command_edge;
mod command_hypergraph;
mod command_typing;
mod context;
mod hypergraph;
mod python_builtin_signatures;
mod solver;
mod tape_language;
mod expression_circuit;
mod types;
mod typing;

use std::error::Error;

use command_dot::{generate_dot_with_clusters, to_svg_with_clusters};
use command_edge::CommandEdge;
use command_typing::infer_command_from_suite;
use graphviz_rust::printer::{DotPrinter, PrinterContext};
use open_hypergraphs::lax::OpenHypergraph;
use open_hypergraphs_dot::Options;
use rustpython_parser::{ast, Parse};
use solver::{apply_substitution, solve_hypergraph_types};

fn main() -> Result<(), Box<dyn Error>> {
    let input = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    let source = if input.is_empty() {
        "x = 1\ny = x"
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
    let term = command_hypergraph::from_command_tree(&tree);
    println!("{}", tree);
    println!("{}", hypergraph::format_hypergraph(&term));

    let opts = Options {
        node_label: Box::new(|t: &types::TypeExpr| t.to_string()),
        edge_label: Box::new(|e: &CommandEdge| e.to_string()),
        ..Options::default()
    };

    write_svg_with_fallback("./out", &term, &opts)?;

    match solve_hypergraph_types(&term) {
        Ok(subst) => {
            let substituted = apply_substitution(&term, &subst);
            let strict = substituted.to_strict();
            let strict_lax = OpenHypergraph::from_strict(strict);
            write_svg_with_fallback("./out_strict", &strict_lax, &opts)?;
        }
        Err(err) => {
            eprintln!("Type solving failed: {}", err);
        }
    }
    Ok(())
}

fn write_svg_with_fallback(
    prefix: &str,
    graph: &OpenHypergraph<types::TypeExpr, CommandEdge>,
    opts: &Options<types::TypeExpr, CommandEdge>,
) -> Result<(), Box<dyn Error>> {
    let svg_path = format!("{}.svg", prefix);
    let dot_path = format!("{}.dot", prefix);
    match to_svg_with_clusters(graph, opts) {
        Ok(svg) => {
            std::fs::write(svg_path, svg)?;
        }
        Err(err) => {
            let dot_graph = generate_dot_with_clusters(graph, opts);
            let mut ctx = PrinterContext::default();
            let dot_string = dot_graph.print(&mut ctx);
            std::fs::write(dot_path, dot_string)?;
            eprintln!(
                "SVG rendering failed ({}). Wrote DOT output to {}.dot.",
                err, prefix
            );
        }
    }
    Ok(())
}
