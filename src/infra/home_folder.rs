use std::error::Error;
use std::path::PathBuf;

use chrono::Local;

pub struct HomeFolderLayout {
    root: PathBuf,
}

impl HomeFolderLayout {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn ensure_root(&self) -> Result<(), Box<dyn Error>> {
        std::fs::create_dir_all(self.runs_dir())?;
        Ok(())
    }

    pub fn runs_dir(&self) -> PathBuf {
        self.root.join("runs")
    }

    pub fn run_dir(&self, run_id: &str) -> PathBuf {
        self.runs_dir().join(run_id)
    }

    pub fn options_path(&self, run_id: &str) -> PathBuf {
        self.run_dir(run_id).join("options.yaml")
    }

    pub fn source_path(&self, run_id: &str) -> PathBuf {
        self.run_dir(run_id).join("source.py")
    }

    pub fn ir_path(&self, run_id: &str) -> PathBuf {
        self.run_dir(run_id).join("ir.dot")
    }

    pub fn ir_before_rewrite_path(&self, run_id: &str) -> PathBuf {
        self.run_dir(run_id).join("ir.before_rewrite.dot")
    }

    pub fn rewrite_steps_dir(&self, run_id: &str) -> PathBuf {
        self.run_dir(run_id).join("rewrite_steps")
    }

    pub fn rewrite_debug_path(&self, run_id: &str) -> PathBuf {
        self.run_dir(run_id).join("rewrite_debug.txt")
    }

    pub fn create_run(&self) -> Result<String, Box<dyn Error>> {
        self.ensure_root()?;
        let timestamp = Local::now().format("%Y-%m-%dT%H-%M-%S").to_string();
        let mut suffix = 0usize;
        loop {
            let run_id = if suffix == 0 {
                timestamp.clone()
            } else {
                format!("{}-{}", timestamp, suffix)
            };
            let run_dir = self.run_dir(&run_id);
            match std::fs::create_dir(&run_dir) {
                Ok(()) => return Ok(run_id),
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                    suffix += 1;
                }
                Err(err) => return Err(Box::new(err)),
            }
        }
    }
}
