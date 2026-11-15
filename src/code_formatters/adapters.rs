use super::FormatterFailure;
use std::{path::Path, process::Command};

#[derive(Clone, Copy, Debug)]
pub(crate) enum Adapter {
    Rust,
    Go,
    Python,
    Json,
}
impl Adapter {
    pub(crate) fn for_language(language: &str) -> Option<Self> {
        match language {
            "rust" | "rs" => Some(Self::Rust),
            "go" | "golang" => Some(Self::Go),
            "python" | "py" => Some(Self::Python),
            "json" => Some(Self::Json),
            _ => None,
        }
    }
    pub(crate) fn command(
        self,
        width: usize,
        isolated: &Path,
    ) -> Result<Command, FormatterFailure> {
        if !(1..=4096).contains(&width) {
            return Err(FormatterFailure::Skipped(
                "available code width is outside 1..=4096".into(),
            ));
        }
        let mut command = match self {
            Self::Rust => {
                let config = isolated.join("rustfmt.toml");
                std::fs::write(&config, format!("max_width = {width}\n"))
                    .map_err(|error| FormatterFailure::Failed(error.to_string()))?;
                let mut command = Command::new("rustfmt");
                command
                    .args(["--edition", "2024", "--emit", "stdout", "--config-path"])
                    .arg(config);
                command
            }
            Self::Go => Command::new("gofmt"),
            Self::Python => {
                let mut command = Command::new("black");
                command.args([
                    "--isolated",
                    "--quiet",
                    "--line-length",
                    &width.to_string(),
                    "-",
                ]);
                command
            }
            Self::Json => {
                let mut command = Command::new("prettier");
                command.args([
                    "--no-config",
                    "--no-editorconfig",
                    "--no-plugin-search",
                    "--parser",
                    "json",
                    "--print-width",
                    &width.to_string(),
                ]);
                command
            }
        };
        command.current_dir(isolated);
        Ok(command)
    }
}
