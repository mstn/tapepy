#[derive(Clone, Copy)]
pub enum OutputFormat {
    Dot,
}

impl OutputFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            OutputFormat::Dot => "dot",
        }
    }
}

pub fn yaml_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}
