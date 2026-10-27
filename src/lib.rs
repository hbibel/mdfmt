pub mod config;
mod diagnostic;
mod edit;
mod md013;
mod md026;
mod parse;
mod reflow;
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

/// Process Markdown without filesystem I/O. Rule coverage remains incomplete;
/// empty diagnostics do not establish lint compliance.
pub fn format(source: &str, options: &FormatOptions) -> Result<FormatResult, ProcessingError> {
    format_with_config(source, options, &config::ResolvedConfig::default())
}

/// Process using resolved configuration; this function performs no filesystem I/O.
pub fn format_with_config(
    source: &str,
    options: &FormatOptions,
    config: &config::ResolvedConfig,
) -> Result<FormatResult, ProcessingError> {
    let mut result = process(
        source,
        options,
        config,
        |source, parsed, document| {
            let mut edits = reflow::propose(source, parsed, document);
            edits.extend(md026::propose(source, parsed, document));
            edits
        },
        |_| vec![],
    )?;
    let output = Source::new(result.output.clone());
    let parsed = parse::parse(&output).map_err(ProcessingError::Parse)?;
    let document = config
        .document(output.text())
        .map_err(ProcessingError::Configuration)?;
    result.diagnostics = md013::diagnose(&output, &parsed, &document);
    result
        .diagnostics
        .extend(md026::diagnose(&output, &parsed, &document));
    result.coverage.implemented_rules = vec!["MD013".into(), "MD026".into()];
    Ok(result)
}

fn process(
    text: &str,
    options: &FormatOptions,
    config: &config::ResolvedConfig,
    produce: impl FnOnce(&Source, &parse::ParsedDocument, &config::DocumentConfig) -> Vec<edit::Edit>,
    diagnose: impl FnOnce(&str) -> Vec<Diagnostic>,
) -> Result<FormatResult, ProcessingError> {
    let source = Source::new(text.into());
    let document = config
        .document(text)
        .map_err(ProcessingError::Configuration)?;
    let parsed = parse::parse(&source).map_err(ProcessingError::Parse)?;
    let mut edits = if options.fix && config.fix {
        let proposed = produce(&source, &parsed, &document);
        edit::validate_ranges(&source, &proposed)?;
        proposed
            .into_iter()
            .filter(|edit| document.allows_source(&edit.origin, edit.span.clone(), &source))
            .collect()
    } else {
        vec![]
    };
    edits.sort_by_key(|edit| (edit.span.start, edit.span.end));
    let authorized = md026::propose(&source, &parsed, &document);
    let output = edit::apply_authorized_edits(&source, &parsed, edits.clone(), &authorized)?;
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

    fn replace_world(
        source: &Source,
        _: &parse::ParsedDocument,
        _: &config::DocumentConfig,
    ) -> Vec<edit::Edit> {
        source
            .text()
            .find("world")
            .map(|start| edit::Edit {
                span: start..start + 5,
                replacement: "Rust".into(),
                origin: "MD013".into(),
            })
            .into_iter()
            .collect()
    }

    #[test]
    fn real_edits_are_idempotent_and_permission_gated() {
        let first = process(
            "Hello world!\r\n",
            &FormatOptions::default(),
            &config::ResolvedConfig::default(),
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
            &config::ResolvedConfig::default(),
            replace_world,
            |_| vec![],
        )
        .unwrap();
        assert_eq!(first.output, second.output);
        assert!(second.edits.is_empty());
        let disabled = process(
            "Hello world!",
            &FormatOptions { fix: false },
            &config::ResolvedConfig::default(),
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

    #[test]
    fn resolved_fix_and_inline_permissions_gate_real_edits() {
        for (text, config) in [
            (
                "Hello world!",
                config::ResolvedConfig {
                    fix: false,
                    ..Default::default()
                },
            ),
            (
                "Hello world!",
                config::ResolvedConfig {
                    rules: config::RuleConfig::from_value(serde_json::json!({"MD013":false}))
                        .unwrap(),
                    ..Default::default()
                },
            ),
            (
                "<!-- markdownlint-disable-file MD013 -->\nHello world!",
                config::ResolvedConfig::default(),
            ),
            (
                "Hello world! <!-- markdownlint-disable-line MD013 -->",
                config::ResolvedConfig::default(),
            ),
        ] {
            let result = process(
                text,
                &FormatOptions::default(),
                &config,
                replace_world,
                |_| vec![],
            )
            .unwrap();
            assert_eq!(result.output, text);
            assert!(result.edits.is_empty());
        }
    }

    #[test]
    fn invalid_ranges_are_errors_not_silently_filtered_permissions() {
        let error = process(
            "Hello world",
            &FormatOptions::default(),
            &config::ResolvedConfig::default(),
            |_, _, _| {
                vec![edit::Edit {
                    span: 0..99,
                    replacement: String::new(),
                    origin: "MD013".into(),
                }]
            },
            |_| vec![],
        )
        .unwrap_err();
        assert!(matches!(error, ProcessingError::InvalidRange(_)));
    }
}
