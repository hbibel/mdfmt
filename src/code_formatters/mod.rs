mod adapters;
mod runner;

use adapters::Adapter;
use runner::{RunLimits, run};
use std::time::{Duration, Instant};

#[derive(Debug)]
pub(crate) enum FormatterFailure {
    Unavailable(String),
    Failed(String),
    Timeout,
    OutputLimit,
    Unstable,
    Skipped(String),
}
impl FormatterFailure {
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::Unavailable(_) => "formatter-unavailable",
            Self::Failed(_) => "formatter-failed",
            Self::Timeout => "formatter-timeout",
            Self::OutputLimit => "formatter-output-limit",
            Self::Unstable => "formatter-unstable",
            Self::Skipped(_) => "formatter-skipped",
        }
    }
    pub(crate) fn message(&self) -> String {
        match self {
            Self::Unavailable(message) | Self::Failed(message) | Self::Skipped(message) => {
                message.clone()
            }
            Self::Timeout => "formatter deadline exceeded".into(),
            Self::OutputLimit => "formatter output exceeded its byte budget".into(),
            Self::Unstable => "formatter output did not settle on a second invocation".into(),
        }
    }
}

fn stable_output(
    input: &str,
    mut format: impl FnMut(&[u8]) -> Result<Vec<u8>, FormatterFailure>,
) -> Result<String, FormatterFailure> {
    fn normalize(bytes: Vec<u8>, nonempty: bool) -> Result<String, FormatterFailure> {
        let output = String::from_utf8(bytes)
            .map_err(|_| FormatterFailure::Failed("formatter output is not UTF-8".into()))?;
        if output.contains('\0') || (nonempty && output.trim().is_empty()) {
            return Err(FormatterFailure::Failed(
                "formatter returned empty or NUL-containing output".into(),
            ));
        }
        Ok(output.replace("\r\n", "\n").replace('\r', "\n"))
    }
    let first = normalize(format(input.as_bytes())?, !input.trim().is_empty())?;
    let second = normalize(format(first.as_bytes())?, !input.trim().is_empty())?;
    if first != second {
        return Err(FormatterFailure::Unstable);
    }
    Ok(first)
}

pub(crate) fn format_stable(
    adapter: Adapter,
    input: &str,
    width: usize,
    deadline: Instant,
) -> Result<String, FormatterFailure> {
    if input.len() > 1024 * 1024 {
        return Err(FormatterFailure::Skipped("code body exceeds 1 MiB".into()));
    }
    let isolated =
        tempfile::tempdir().map_err(|error| FormatterFailure::Failed(error.to_string()))?;
    stable_output(input, |input| {
        let timeout = deadline
            .saturating_duration_since(Instant::now())
            .min(Duration::from_secs(5));
        let command = adapter.command(width, isolated.path())?;
        run(
            command,
            input,
            &RunLimits {
                timeout,
                stdout: 1024 * 1024,
                stderr: 64 * 1024,
            },
        )
    })
}

#[cfg(test)]
mod tests;
