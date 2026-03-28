mod cli;
mod command_dot;
mod command_tape;
mod command_typing;
mod context;
mod expression_circuit;
mod predicate_tape;
mod program_tape;
mod python_builtin_signatures;
mod solver;
mod tape_language;
mod types;
mod typing;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    cli::run()
}
