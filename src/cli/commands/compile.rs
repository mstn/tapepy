use std::error::Error;

use crate::app::compile_run::{compile_run, CompileRequest};
use crate::cli::args::CompileArgs;
use crate::infra::home_folder::HomeFolderLayout;

pub fn run(home: &HomeFolderLayout, args: &CompileArgs) -> Result<(), Box<dyn Error>> {
    let result = compile_run(
        home,
        &CompileRequest {
            input_file: &args.filepath,
            language: &args.language,
            raw_tape: args.raw_tape,
        },
    )?;
    println!("{}", result.run_dir.display());
    Ok(())
}
