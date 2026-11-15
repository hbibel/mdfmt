mod adapters;
mod fences;
mod runner;

use crate::{
    Diagnostic, FormatOptions, FormatResult, ProcessingError, Severity, Source,
    config::ResolvedConfig,
    edit::{Edit, compose_edits},
    parse,
};
use adapters::Adapter;
use runner::{RunLimits, run};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, Default)]
pub struct CodeFormatOptions {
    pub enabled: bool,
}

#[derive(Debug)]
pub(crate) enum FormatterFailure {
    #[cfg(unix)]
    Unavailable(String),
    Failed(String),
    Timeout,
    OutputLimit,
    Unstable,
    Skipped(String),
    Unsafe,
    #[cfg_attr(not(unix), allow(dead_code))]
    Cleanup(String),
}
impl FormatterFailure {
    pub(crate) fn code(&self) -> &'static str {
        match self {
            #[cfg(unix)]
            Self::Unavailable(_) => "formatter-unavailable",
            Self::Failed(_) => "formatter-failed",
            Self::Timeout => "formatter-timeout",
            Self::OutputLimit => "formatter-output-limit",
            Self::Unstable => "formatter-unstable",
            Self::Skipped(_) => "formatter-skipped",
            Self::Unsafe => "formatter-unsafe",
            Self::Cleanup(_) => "formatter-cleanup-failed",
        }
    }
    pub(crate) fn message(&self) -> String {
        match self {
            #[cfg(unix)]
            Self::Unavailable(message) => message.clone(),
            Self::Failed(message) | Self::Skipped(message) | Self::Cleanup(message) => {
                message.clone()
            }
            Self::Timeout => "formatter deadline exceeded".into(),
            Self::OutputLimit => "formatter output exceeded its byte budget".into(),
            Self::Unstable => "formatter output did not settle on a second invocation".into(),
            Self::Unsafe => "formatter output changes Markdown outside its owned code value".into(),
        }
    }
}

/// Format Markdown and optionally execute installed code formatters with process I/O.
pub fn format_with_code_formatters(
    source: &str,
    options: &FormatOptions,
    config: &ResolvedConfig,
    code: &CodeFormatOptions,
) -> Result<FormatResult, ProcessingError> {
    let mut first = crate::format_with_config(source, options, config)?;
    if !code.enabled || !options.fix || !config.fix {
        return Ok(first);
    }
    let before = Source::new(first.output.clone());
    let parsed = parse::parse(&before).map_err(ProcessingError::Parse)?;
    let document = config
        .document(before.text())
        .map_err(ProcessingError::Configuration)?;
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut edits = vec![];
    let mut selected = std::collections::HashSet::new();
    let mut failures = vec![];
    let mut containment_failed = false;
    for (index, body) in fences::bodies(&before, &parsed, &document)
        .into_iter()
        .enumerate()
    {
        let formatted = if containment_failed {
            Err(FormatterFailure::Skipped(
                "formatter execution disabled after cleanup failure".into(),
            ))
        } else if let Some(failure) = body.skip {
            Err(failure)
        } else if index >= 64 {
            Err(FormatterFailure::Skipped(
                "document exceeds 64 eligible code bodies".into(),
            ))
        } else {
            format_stable(body.adapter, &body.logical, body.width, deadline)
        };
        let failure = match formatted {
            Err(failure) => Some(failure),
            Ok(output) => {
                let mut replacement = String::new();
                for line in output.split_terminator('\n') {
                    replacement.push_str(&body.prefix);
                    replacement.push_str(line);
                    replacement.push_str(body.newline);
                }
                if replacement == before.text()[body.span.clone()] {
                    continue;
                }
                let edit = Edit {
                    span: body.span,
                    replacement,
                    origin: "code-formatter".into(),
                };
                edits.push(edit);
                selected.insert(body.ordinal);
                if fences::validated_output(&before, &parsed, &edits, &selected).is_some() {
                    None
                } else {
                    edits.pop();
                    selected.remove(&body.ordinal);
                    Some(FormatterFailure::Unsafe)
                }
            }
        };
        if let Some(failure) = failure {
            containment_failed |= matches!(failure, FormatterFailure::Cleanup(_));
            failures.push((body.ordinal, body.language, failure));
        }
    }
    if !edits.is_empty() {
        let output = fences::validated_output(&before, &parsed, &edits, &selected)
            .ok_or(ProcessingError::EditConflict)?;
        let combined = compose_edits(source, &first.edits, &edits)?;
        let mut result = crate::format_with_config(&output, options, config)?;
        result.edits = compose_edits(source, &combined, &result.edits)?;
        first = result;
    }
    let output = Source::new(first.output.clone());
    let parsed = parse::parse(&output).map_err(ProcessingError::Parse)?;
    let anchors = fences::anchors(&parsed);
    for (ordinal, language, failure) in failures {
        first.diagnostics.push(Diagnostic {
            code: failure.code().into(),
            severity: Severity::Warning,
            message: format!("{language}: {}", failure.message()),
            rule_id: None,
            span: anchors.get(ordinal).copied().map(|start| start..start),
        });
    }
    Ok(first)
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
