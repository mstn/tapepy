use std::error::Error;
use std::path::{Path, PathBuf};

use crate::compiler::{self, CompileOptions};
use crate::frontends::python::PythonCompiler;
use crate::infra::home_folder::HomeFolderLayout;

pub struct CompileRequest<'a> {
    pub input_file: &'a Path,
    pub language: &'a str,
    pub raw_tape: bool,
}

pub struct CompileRunResult {
    pub run_dir: PathBuf,
}

pub fn compile_run(
    home: &HomeFolderLayout,
    request: &CompileRequest<'_>,
) -> Result<CompileRunResult, Box<dyn Error>> {
    let source = std::fs::read_to_string(request.input_file)?;
    let run_id = home.create_run()?;

    std::fs::write(home.source_path(&run_id), &source)?;
    std::fs::write(
        home.options_path(&run_id),
        compile_options_yaml(request.input_file, request.language, request.raw_tape),
    )?;

    let compile_options = CompileOptions {
        raw_tape: request.raw_tape,
    };

    let artifacts = match request.language.to_lowercase().as_str() {
        "python" => compiler::compile(
            &PythonCompiler,
            request.input_file.to_string_lossy().as_ref(),
            &source,
            &compile_options,
        )?,
        language => return Err(format!("unsupported language `{}`", language).into()),
    };

    std::fs::write(home.ir_path(&run_id), artifacts.ir_dot)?;

    Ok(CompileRunResult {
        run_dir: home.run_dir(&run_id),
    })
}

fn compile_options_yaml(input_file: &Path, language: &str, raw_tape: bool) -> String {
    format!(
        "command: compile\ninput_file: {}\nlanguage: {}\nraw_tape: {}\noutput_format: {}\n",
        yaml_string(&input_file.display().to_string()),
        yaml_string(language),
        raw_tape,
        yaml_string("dot"),
    )
}

fn yaml_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}
