mod context;
mod hypergraph;
mod types;
mod typing;

use std::error::Error;

use graphviz_rust::printer::{DotPrinter, PrinterContext};
use open_hypergraphs_dot::{generate_dot_with, svg::to_svg_with, Options};
use rustpython_parser::{ast, Parse};
use typing::infer_expression;

fn main() -> Result<(), Box<dyn Error>> {
    let input = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    let source = if input.is_empty() { "x + 1.0" } else { &input };

    let expr = match ast::Expr::parse(source, "<input>") {
        Ok(expr) => expr,
        Err(err) => {
            eprintln!("Parse error: {}", err);
            return Ok(());
        }
    };

    let tree = infer_expression(&expr);
    let term = hypergraph::from_deduction_tree(&tree);
    println!("{}", tree);
    println!("{}", hypergraph::format_hypergraph(&term));
    match to_svg_with(&term, &Options::default()) {
        Ok(svg) => {
            std::fs::write("./out.svg", svg)?;
        }
        Err(err) => {
            let dot_graph = generate_dot_with(&term, &Options::default());
            let mut ctx = PrinterContext::default();
            let dot_string = dot_graph.print(&mut ctx);
            std::fs::write("./out.dot", dot_string)?;
            eprintln!(
                "SVG rendering failed ({}). Wrote DOT output to out.dot.",
                err
            );
        }
    }
    Ok(())
}
