mod diagnostic;
mod edit;
mod parse;
mod source;

pub use diagnostic::{Diagnostic, ProcessingError, Severity};
pub use edit::AppliedEdit;
pub use source::{Position, PositionError, Source, utf16_len};

#[derive(Debug, Clone, Copy)]
pub struct FormatOptions {
    pub fix: bool,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self { fix: true }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Coverage {
    pub implemented_rules: Vec<String>,
    pub complete: bool,
}

#[derive(Debug)]
pub struct FormatResult {
    pub output: String,
    pub edits: Vec<AppliedEdit>,
    pub diagnostics: Vec<Diagnostic>,
    pub coverage: Coverage,
}

impl FormatResult {
    pub fn changed(&self, original: &str) -> bool {
        self.output != original
    }
}

/// Process Markdown without filesystem I/O. The foundation currently implements
/// no rule fixes or findings; empty diagnostics do not establish lint compliance.
pub fn format(source: &str, options: &FormatOptions) -> Result<FormatResult, ProcessingError> {
    process(source, options, |_| vec![], |_| vec![])
}

fn process(
    text: &str,
    options: &FormatOptions,
    produce: impl FnOnce(&Source) -> Vec<edit::Edit>,
    diagnose: impl FnOnce(&str) -> Vec<Diagnostic>,
) -> Result<FormatResult, ProcessingError> {
    let source = Source::new(text.into());
    let parsed = parse::parse(&source).map_err(ProcessingError::Parse)?;
    let mut edits = if options.fix {
        produce(&source)
    } else {
        vec![]
    };
    edits.sort_by_key(|edit| (edit.span.start, edit.span.end));
    let output = edit::apply_edits(&source, &parsed, edits.clone())?;
    let diagnostics = diagnose(&output);
    Ok(FormatResult {
        output,
        edits,
        diagnostics,
        coverage: Coverage {
            implemented_rules: vec![],
            complete: false,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn replace_world(source: &Source) -> Vec<edit::Edit> {
        source
            .text()
            .find("world")
            .map(|start| edit::Edit {
                span: start..start + 5,
                replacement: "Rust".into(),
                origin: "test".into(),
            })
            .into_iter()
            .collect()
    }

    #[test]
    fn real_edits_are_idempotent_and_permission_gated() {
        let first = process(
            "Hello world!\r\n",
            &FormatOptions::default(),
            replace_world,
            |_| vec![],
        )
        .unwrap();
        assert_eq!(first.output, "Hello Rust!\r\n");
        assert!(first.changed("Hello world!\r\n"));
        assert_eq!(first.edits.len(), 1);
        let second = process(
            &first.output,
            &FormatOptions::default(),
            replace_world,
            |_| vec![],
        )
        .unwrap();
        assert_eq!(first.output, second.output);
        assert!(second.edits.is_empty());
        let disabled = process(
            "Hello world!",
            &FormatOptions { fix: false },
            replace_world,
            |output| {
                vec![Diagnostic {
                    code: "test".into(),
                    severity: Severity::Warning,
                    message: output.into(),
                    rule_id: None,
                    span: Some(6..11),
                }]
            },
        )
        .unwrap();
        assert_eq!(disabled.output, "Hello world!");
        assert!(disabled.edits.is_empty());
        assert_eq!(disabled.diagnostics[0].message, "Hello world!");
        assert_eq!(disabled.diagnostics[0].severity, Severity::Warning);
    }
}
