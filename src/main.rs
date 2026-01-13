mod context;
mod types;
mod typing;

use rustpython_parser::{ast, Parse};
use typing::infer_expression;

fn main() {
    let input = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    let source = &input;

    let expr = match ast::Expr::parse(source, "<input>") {
        Ok(expr) => expr,
        Err(err) => {
            eprintln!("Parse error: {}", err);
            return;
        }
    };

    let tree = infer_expression(&expr);
    println!("{}", tree);
}
