mod blank_lines;
mod code_formatters;
pub mod config;
mod conflicts;
mod diagnostic;
mod edit;
mod emphasis_styles;
mod heading_spaces;
mod heading_structure;
mod inline_spaces;
mod md003;
mod md004;
mod md005;
mod md007;
mod md009;
mod md010;
mod md011;
mod md012;
mod md013;
mod md014;
mod md018;
mod md020;
mod md023;
mod md024;
mod md026;
mod md027;
mod md028;
mod md029;
mod md030;
mod md033;
mod md034;
mod md035;
mod md036;
mod md040;
mod md042;
mod md044;
mod md045;
mod md046;
mod md047;
mod md048;
mod md051;
mod md055;
mod md056;
mod md058;
mod md059;
mod md060;
mod parse;
mod pipeline;
mod reference_rules;
mod reflow;
mod source;
mod stabilize;

pub use code_formatters::{CodeFormatOptions, format_with_code_formatters};
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

/// Process Markdown without filesystem I/O. Some rules are diagnostic-only;
/// empty diagnostics do not establish complete parser or option parity.
pub fn format(source: &str, options: &FormatOptions) -> Result<FormatResult, ProcessingError> {
    format_with_config(source, options, &config::ResolvedConfig::default())
}

/// Process using resolved configuration; this function performs no filesystem I/O.
pub fn format_with_config(
    source: &str,
    options: &FormatOptions,
    config: &config::ResolvedConfig,
) -> Result<FormatResult, ProcessingError> {
    transaction(source, options, config, formatting_pass)
}

fn transaction(
    source: &str,
    options: &FormatOptions,
    config: &config::ResolvedConfig,
    mut pass: impl FnMut(
        &str,
        &FormatOptions,
        &config::ResolvedConfig,
        &mut std::collections::BTreeSet<String>,
    ) -> Result<FormatResult, stabilize::AttemptFailure>,
) -> Result<FormatResult, ProcessingError> {
    use stabilize::{AttemptFailure, Phase, StateTracker};
    let mut tracker = StateTracker::new(source.len());
    let metadata = config.source(source)?;
    let mut current = source.to_owned();
    let mut edits = vec![];
    let mut origins = std::collections::BTreeSet::new();
    let failure = loop {
        if let Err(failure) = tracker.observe(
            Phase::Complete,
            &current,
            &origins.iter().cloned().collect::<Vec<_>>(),
        ) {
            break failure;
        }
        let mut result = match pass(&current, options, config, &mut origins) {
            Ok(result) => result,
            Err(failure) => break failure,
        };
        if !metadata.metadata_stable(&result.output) {
            return Err(ProcessingError::ProtectedRegion(
                0..metadata.front_matter_end().unwrap_or(0),
            ));
        }
        edits = stabilize::compose(source, &edits, &result.edits, &mut origins)?;
        if result.output == current {
            result.edits = edits;
            return Ok(result);
        }
        current = result.output;
    };
    match failure {
        AttemptFailure::Processing(error) => Err(error),
        AttemptFailure::Cycle { rules } => {
            let mut result = pass(
                source,
                &FormatOptions { fix: false },
                config,
                &mut std::collections::BTreeSet::new(),
            )
            .map_err(|failure| match failure {
                AttemptFailure::Processing(error) => error,
                AttemptFailure::Cycle { .. } => {
                    ProcessingError::WorkLimit("no-fix diagnostic pass cycled".into())
                }
            })?;
            let original = config.source(source)?;
            let document = config
                .document(source)
                .map_err(ProcessingError::Configuration)?;
            let at = if source.starts_with('\u{feff}') { 3 } else { 0 };
            result.diagnostics.extend(conflicts::diagnose_conflicts(
                &original,
                &document,
                &[conflicts::RuleConflict {
                    span: at..at,
                    rules,
                    retained: None,
                }],
            ));
            Ok(result)
        }
    }
}

fn formatting_pass(
    source: &str,
    options: &FormatOptions,
    config: &config::ResolvedConfig,
    origins: &mut std::collections::BTreeSet<String>,
) -> Result<FormatResult, stabilize::AttemptFailure> {
    use stabilize::{Phase, StateTracker};
    let mut normalize = StateTracker::new(source.len());
    let mut context = pipeline::Context::new(config);
    let mut current = source.to_owned();
    let mut deletion_edits = vec![];
    // Tab expansion and heading removal can expose suffix whitespace.
    loop {
        normalize.observe(
            Phase::Normalize,
            &current,
            &origins.iter().cloned().collect::<Vec<_>>(),
        )?;
        let mut changed = false;
        let pass = context.process(&current, options, &["MD010"], md010::propose, |_| vec![])?;
        if !pass.edits.is_empty() {
            deletion_edits = stabilize::compose(source, &deletion_edits, &pass.edits, origins)?;
            current = pass.output;
            changed = true;
        }
        let pass = context.process(&current, options, &["MD030"], md030::propose, |_| vec![])?;
        if !pass.edits.is_empty() {
            deletion_edits = stabilize::compose(source, &deletion_edits, &pass.edits, origins)?;
            current = pass.output;
            changed = true;
        }
        let pass = context.process(&current, options, &["MD029"], md029::propose, |_| vec![])?;
        if !pass.edits.is_empty() {
            deletion_edits = stabilize::compose(source, &deletion_edits, &pass.edits, origins)?;
            current = pass.output;
            changed = true;
        }
        let pass = context.process(&current, options, &["MD005"], md005::propose, |_| vec![])?;
        if !pass.edits.is_empty() {
            deletion_edits = stabilize::compose(source, &deletion_edits, &pass.edits, origins)?;
            current = pass.output;
            changed = true;
        }
        let pass = context.process(&current, options, &["MD007"], md007::propose, |_| vec![])?;
        if !pass.edits.is_empty() {
            deletion_edits = stabilize::compose(source, &deletion_edits, &pass.edits, origins)?;
            current = pass.output;
            changed = true;
        }
        let pass = context.process(&current, options, &["MD027"], md027::propose, |_| vec![])?;
        if !pass.edits.is_empty() {
            deletion_edits = stabilize::compose(source, &deletion_edits, &pass.edits, origins)?;
            current = pass.output;
            changed = true;
        }
        let pass = context.process(&current, options, &["MD023"], md023::propose, |_| vec![])?;
        if !pass.edits.is_empty() {
            deletion_edits = stabilize::compose(source, &deletion_edits, &pass.edits, origins)?;
            current = pass.output;
            changed = true;
        }
        let mut trailing = StateTracker::new(source.len());
        loop {
            trailing.observe(
                Phase::Trailing,
                &current,
                &origins.iter().cloned().collect::<Vec<_>>(),
            )?;
            let pass =
                context.process(&current, options, &["MD009"], md009::propose, |_| vec![])?;
            if pass.edits.is_empty() {
                break;
            }
            deletion_edits = stabilize::compose(source, &deletion_edits, &pass.edits, origins)?;
            current = pass.output;
            changed = true;
        }
        let mut headings = StateTracker::new(source.len());
        loop {
            headings.observe(
                Phase::Headings,
                &current,
                &origins.iter().cloned().collect::<Vec<_>>(),
            )?;
            let pass = context.process(
                &current,
                options,
                &["MD026", "MD019", "MD021"],
                |source, parsed, document| {
                    let mut edits = md026::propose(source, parsed, document);
                    edits.extend(heading_spaces::propose(source, parsed, document));
                    edits
                },
                |_| vec![],
            )?;
            if pass.edits.is_empty() {
                break;
            }
            deletion_edits = stabilize::compose(source, &deletion_edits, &pass.edits, origins)?;
            current = pass.output;
            changed = true;
        }
        if !changed {
            break;
        }
    }
    let mut blanks = StateTracker::new(source.len());
    loop {
        blanks.observe(
            Phase::Blanks,
            &current,
            &origins.iter().cloned().collect::<Vec<_>>(),
        )?;
        let pass = context.process(&current, options, &["MD012"], md012::propose, |_| vec![])?;
        if pass.edits.is_empty() {
            break;
        }
        deletion_edits = stabilize::compose(source, &deletion_edits, &pass.edits, origins)?;
        current = pass.output;
    }
    let snapshot = context.snapshot(&current)?;
    let before_inline = &snapshot.source;
    let before_inline_parsed = &snapshot.parsed;
    let before_inline_document = &snapshot.document;
    let inline_edits = if options.fix && config.fix && snapshot.enabled(&["MD049", "MD050"]) {
        emphasis_styles::propose(before_inline, before_inline_parsed, before_inline_document)
    } else {
        vec![]
    };
    let current = edit::apply_authorized_edits(
        before_inline,
        before_inline_parsed,
        inline_edits.clone(),
        &inline_edits,
    )?;
    deletion_edits = stabilize::compose(source, &deletion_edits, &inline_edits, origins)?;
    // Heading suffix deletions can expose closing markers or turn setext text
    // into prose. Reparse that stable result before selecting prose gaps.
    let mut result = context.process(&current, options, &["MD013"], reflow::propose, |_| vec![])?;
    result.edits = stabilize::compose(source, &deletion_edits, &result.edits, origins)?;
    // Either contextual marker repair can make the other rule's rejected batch
    // safe without changing the parsed block tree.
    let mut markers = StateTracker::new(source.len());
    loop {
        markers.observe(
            Phase::Markers,
            &result.output,
            &origins.iter().cloned().collect::<Vec<_>>(),
        )?;
        let snapshot = context.snapshot(&result.output)?;
        let before_markers = &snapshot.source;
        let before_markers_parsed = &snapshot.parsed;
        let before_markers_document = &snapshot.document;
        let marker_edits = if options.fix && config.fix && snapshot.enabled(&["MD004"]) {
            md004::propose(
                before_markers,
                before_markers_parsed,
                before_markers_document,
            )
        } else {
            vec![]
        };
        result.output = edit::apply_authorized_edits(
            before_markers,
            before_markers_parsed,
            marker_edits.clone(),
            &marker_edits,
        )?;
        result.edits = stabilize::compose(source, &result.edits, &marker_edits, origins)?;
        let snapshot = context.snapshot(&result.output)?;
        let before_style = &snapshot.source;
        let before_style_parsed = &snapshot.parsed;
        let before_style_document = &snapshot.document;
        let style_edits = if options.fix && config.fix && snapshot.enabled(&["MD035"]) {
            md035::propose(before_style, before_style_parsed, before_style_document)
        } else {
            vec![]
        };
        result.output = edit::apply_authorized_edits(
            before_style,
            before_style_parsed,
            style_edits.clone(),
            &style_edits,
        )?;
        result.edits = stabilize::compose(source, &result.edits, &style_edits, origins)?;
        if marker_edits.is_empty() && style_edits.is_empty() {
            break;
        }
    }
    let snapshot = context.snapshot(&result.output)?;
    let before_fences = &snapshot.source;
    let before_fences_parsed = &snapshot.parsed;
    let before_fences_document = &snapshot.document;
    let fence_edits = if options.fix && config.fix && snapshot.enabled(&["MD048"]) {
        md048::propose(before_fences, before_fences_parsed, before_fences_document)
    } else {
        vec![]
    };
    result.output = edit::apply_authorized_edits(
        before_fences,
        before_fences_parsed,
        fence_edits.clone(),
        &fence_edits,
    )?;
    result.edits = stabilize::compose(source, &result.edits, &fence_edits, origins)?;
    let snapshot = context.snapshot(&result.output)?;
    let before_separators = &snapshot.source;
    let before_separators_parsed = &snapshot.parsed;
    let before_separators_document = &snapshot.document;
    let separator_edits =
        if options.fix && config.fix && snapshot.enabled(&["MD022", "MD031", "MD032"]) {
            blank_lines::propose(
                before_separators,
                before_separators_parsed,
                before_separators_document,
            )
        } else {
            vec![]
        };
    result.output = edit::apply_authorized_edits(
        before_separators,
        before_separators_parsed,
        separator_edits.clone(),
        &separator_edits,
    )?;
    result.edits = stabilize::compose(source, &result.edits, &separator_edits, origins)?;
    let snapshot = context.snapshot(&result.output)?;
    let before_tables = &snapshot.source;
    let before_tables_parsed = &snapshot.parsed;
    let before_tables_document = &snapshot.document;
    let table_edits = if options.fix && config.fix && snapshot.enabled(&["MD058"]) {
        md058::propose(before_tables, before_tables_parsed, before_tables_document)
    } else {
        vec![]
    };
    result.output = edit::apply_authorized_edits(
        before_tables,
        before_tables_parsed,
        table_edits.clone(),
        &table_edits,
    )?;
    result.edits = stabilize::compose(source, &result.edits, &table_edits, origins)?;
    // Table separators can make an ATX-to-setext conversion safe in the same call.
    let snapshot = context.snapshot(&result.output)?;
    let before_headings = &snapshot.source;
    let before_headings_parsed = &snapshot.parsed;
    let before_headings_document = &snapshot.document;
    let heading_edits = if options.fix && config.fix && snapshot.enabled(&["MD003"]) {
        md003::propose(
            before_headings,
            before_headings_parsed,
            before_headings_document,
        )
    } else {
        vec![]
    };
    result.output = edit::apply_authorized_edits(
        before_headings,
        before_headings_parsed,
        heading_edits.clone(),
        &heading_edits,
    )?;
    result.edits = stabilize::compose(source, &result.edits, &heading_edits, origins)?;
    let snapshot = context.snapshot(&result.output)?;
    let before_fragments = &snapshot.source;
    let before_fragments_parsed = &snapshot.parsed;
    let before_fragments_document = &snapshot.document;
    let fragment_edits = if options.fix && config.fix && snapshot.enabled(&["MD051"]) {
        md051::propose(
            before_fragments,
            before_fragments_parsed,
            before_fragments_document,
        )?
    } else {
        vec![]
    };
    result.output = edit::apply_authorized_edits(
        before_fragments,
        before_fragments_parsed,
        fragment_edits.clone(),
        &fragment_edits,
    )?;
    result.edits = stabilize::compose(source, &result.edits, &fragment_edits, origins)?;
    let snapshot = context.snapshot(&result.output)?;
    let before_eof = &snapshot.source;
    let before_eof_parsed = &snapshot.parsed;
    let before_eof_document = &snapshot.document;
    let eof_edits = if options.fix && config.fix && snapshot.enabled(&["MD047"]) {
        md047::propose(
            before_eof,
            before_eof_parsed,
            before_eof_document,
            md047::preferred_newline(source),
        )
    } else {
        vec![]
    };
    result.output =
        edit::apply_authorized_edits(before_eof, before_eof_parsed, eof_edits.clone(), &eof_edits)?;
    result.edits = stabilize::compose(source, &result.edits, &eof_edits, origins)?;
    let snapshot = context.snapshot(&result.output)?;
    let output = &snapshot.source;
    let parsed = &snapshot.parsed;
    let document = &snapshot.document;
    result.diagnostics = snapshot.diagnose(&["MD013"], md013::diagnose);
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD052", "MD053", "MD054"], reference_rules::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD037", "MD038", "MD039"], inline_spaces::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD049", "MD050"], emphasis_styles::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD022", "MD031", "MD032"], blank_lines::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD060"], md060::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD033"], md033::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD034"], md034::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD044"], md044::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD003"], md003::diagnose));
    result
        .diagnostics
        .extend(heading_structure::diagnose(output, parsed, document)?);
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD036"], md036::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD059"], md059::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD058"], md058::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD055"], md055::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD056"], md056::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD014"], md014::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD046"], md046::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD011"], md011::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD018"], md018::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD020"], md020::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD030"], md030::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD029"], md029::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD027"], md027::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD048"], md048::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD024"], md024::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD028"], md028::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD045"], md045::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD042"], md042::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD040"], md040::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD023"], md023::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD010"], md010::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD009"], md009::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD004"], md004::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD012"], md012::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD026"], md026::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD019", "MD021"], heading_spaces::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD047"], md047::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD035"], md035::diagnose));
    result.coverage.implemented_rules = vec![
        "MD001".into(),
        "MD003".into(),
        "MD004".into(),
        "MD005".into(),
        "MD007".into(),
        "MD009".into(),
        "MD010".into(),
        "MD011".into(),
        "MD012".into(),
        "MD013".into(),
        "MD014".into(),
        "MD018".into(),
        "MD019".into(),
        "MD020".into(),
        "MD021".into(),
        "MD022".into(),
        "MD023".into(),
        "MD024".into(),
        "MD025".into(),
        "MD026".into(),
        "MD027".into(),
        "MD028".into(),
        "MD029".into(),
        "MD030".into(),
        "MD031".into(),
        "MD032".into(),
        "MD033".into(),
        "MD034".into(),
        "MD035".into(),
        "MD036".into(),
        "MD037".into(),
        "MD038".into(),
        "MD039".into(),
        "MD040".into(),
        "MD041".into(),
        "MD042".into(),
        "MD043".into(),
        "MD044".into(),
        "MD045".into(),
        "MD046".into(),
        "MD047".into(),
        "MD048".into(),
        "MD049".into(),
        "MD050".into(),
        "MD051".into(),
        "MD052".into(),
        "MD053".into(),
        "MD054".into(),
        "MD055".into(),
        "MD056".into(),
        "MD058".into(),
        "MD059".into(),
        "MD060".into(),
    ];
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD005"], md005::diagnose));
    result
        .diagnostics
        .extend(snapshot.diagnose(&["MD007"], md007::diagnose));
    result
        .diagnostics
        .extend(md051::diagnose(output, parsed, document)?);
    let mut rejected = vec![];
    if snapshot.enabled(&["MD005"]) {
        rejected.extend(md005::evaluate(output, parsed, document).conflicts);
    }
    if snapshot.enabled(&["MD007"]) {
        rejected.extend(md007::evaluate(output, parsed, document).conflicts);
    }
    if snapshot.enabled(&["MD022", "MD031", "MD032"]) {
        rejected.extend(blank_lines::evaluate(output, parsed, document).conflicts);
    }
    result
        .diagnostics
        .extend(conflicts::diagnose_conflicts(output, document, &rejected));
    Ok(result)
}

#[cfg(test)]
fn process(
    text: &str,
    options: &FormatOptions,
    config: &config::ResolvedConfig,
    produce: impl FnOnce(&Source, &parse::ParsedDocument, &config::DocumentConfig) -> Vec<edit::Edit>,
    diagnose: impl FnOnce(&str) -> Vec<Diagnostic>,
) -> Result<FormatResult, ProcessingError> {
    pipeline::Context::new(config).process(text, options, &["MD013"], produce, diagnose)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_transaction_discards_all_edits_and_diagnoses_original_source() {
        let config=config::ResolvedConfig{rules:config::RuleConfig::from_value(serde_json::json!({"default":false,"MD007":{"severity":"warning"},"MD023":{"severity":"warning"},"MD051":{"severity":"warning"}})).unwrap(),..Default::default()};
        let input = "- # H\n\n[x](#missing)\n";
        let result = transaction(
            input,
            &FormatOptions::default(),
            &config,
            |text, options, config, origins| {
                if !options.fix {
                    return formatting_pass(text, options, config, origins);
                }
                let (output, edits) = if let Some(output) = text.strip_prefix("  ") {
                    (
                        output.to_owned(),
                        vec![edit::Edit {
                            span: 0..2,
                            replacement: String::new(),
                            origin: "MD023".into(),
                        }],
                    )
                } else {
                    (
                        format!("  {text}"),
                        vec![edit::Edit {
                            span: 0..0,
                            replacement: "  ".into(),
                            origin: "MD007".into(),
                        }],
                    )
                };
                origins.extend(edits.iter().map(|edit| edit.origin.clone()));
                Ok(FormatResult {
                    output,
                    edits,
                    diagnostics: vec![],
                    coverage: Coverage {
                        implemented_rules: vec![],
                        complete: false,
                    },
                })
            },
        )
        .unwrap();
        assert_eq!(result.output, input);
        assert!(result.edits.is_empty());
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "MD051")
        );
        let conflict = result
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "rule-conflict")
            .unwrap();
        assert_eq!(conflict.severity, Severity::Warning);
        assert!(conflict.message.contains("MD007, MD023"));
        assert!(conflict.message.contains("original source preserved"));
    }

    #[test]
    fn growing_transaction_returns_work_limit_instead_of_partial_output() {
        let error = transaction(
            "x",
            &FormatOptions::default(),
            &config::ResolvedConfig::default(),
            |text, _, _, _| {
                let output = text.repeat(2);
                Ok(FormatResult {
                    output,
                    edits: vec![edit::Edit {
                        span: text.len()..text.len(),
                        replacement: text.into(),
                        origin: "MD013".into(),
                    }],
                    diagnostics: vec![],
                    coverage: Coverage {
                        implemented_rules: vec![],
                        complete: false,
                    },
                })
            },
        )
        .unwrap_err();
        assert!(matches!(error, ProcessingError::WorkLimit(_)));
        assert!(error.to_string().contains("work limit"));
    }

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
