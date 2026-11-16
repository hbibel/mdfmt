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
        let pass = process(&current, options, config, md010::propose, |_| vec![])?;
        if !pass.edits.is_empty() {
            deletion_edits = stabilize::compose(source, &deletion_edits, &pass.edits, origins)?;
            current = pass.output;
            changed = true;
        }
        let pass = process(&current, options, config, md030::propose, |_| vec![])?;
        if !pass.edits.is_empty() {
            deletion_edits = stabilize::compose(source, &deletion_edits, &pass.edits, origins)?;
            current = pass.output;
            changed = true;
        }
        let pass = process(&current, options, config, md029::propose, |_| vec![])?;
        if !pass.edits.is_empty() {
            deletion_edits = stabilize::compose(source, &deletion_edits, &pass.edits, origins)?;
            current = pass.output;
            changed = true;
        }
        let pass = process(&current, options, config, md005::propose, |_| vec![])?;
        if !pass.edits.is_empty() {
            deletion_edits = stabilize::compose(source, &deletion_edits, &pass.edits, origins)?;
            current = pass.output;
            changed = true;
        }
        let pass = process(&current, options, config, md007::propose, |_| vec![])?;
        if !pass.edits.is_empty() {
            deletion_edits = stabilize::compose(source, &deletion_edits, &pass.edits, origins)?;
            current = pass.output;
            changed = true;
        }
        let pass = process(&current, options, config, md027::propose, |_| vec![])?;
        if !pass.edits.is_empty() {
            deletion_edits = stabilize::compose(source, &deletion_edits, &pass.edits, origins)?;
            current = pass.output;
            changed = true;
        }
        let pass = process(&current, options, config, md023::propose, |_| vec![])?;
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
            let pass = process(&current, options, config, md009::propose, |_| vec![])?;
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
            let pass = process(
                &current,
                options,
                config,
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
        let pass = process(&current, options, config, md012::propose, |_| vec![])?;
        if pass.edits.is_empty() {
            break;
        }
        deletion_edits = stabilize::compose(source, &deletion_edits, &pass.edits, origins)?;
        current = pass.output;
    }
    let before_inline = config.source(&current)?;
    let before_inline_parsed = parse::parse(&before_inline).map_err(ProcessingError::Parse)?;
    let before_inline_document = config
        .document(before_inline.text())
        .map_err(ProcessingError::Configuration)?;
    let inline_edits = if options.fix && config.fix {
        emphasis_styles::propose(
            &before_inline,
            &before_inline_parsed,
            &before_inline_document,
        )
    } else {
        vec![]
    };
    let current = edit::apply_authorized_edits(
        &before_inline,
        &before_inline_parsed,
        inline_edits.clone(),
        &inline_edits,
    )?;
    deletion_edits = stabilize::compose(source, &deletion_edits, &inline_edits, origins)?;
    // Heading suffix deletions can expose closing markers or turn setext text
    // into prose. Reparse that stable result before selecting prose gaps.
    let mut result = process(&current, options, config, reflow::propose, |_| vec![])?;
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
        let before_markers = config.source(&result.output)?;
        let before_markers_parsed =
            parse::parse(&before_markers).map_err(ProcessingError::Parse)?;
        let before_markers_document = config
            .document(before_markers.text())
            .map_err(ProcessingError::Configuration)?;
        let marker_edits = if options.fix && config.fix {
            md004::propose(
                &before_markers,
                &before_markers_parsed,
                &before_markers_document,
            )
        } else {
            vec![]
        };
        result.output = edit::apply_authorized_edits(
            &before_markers,
            &before_markers_parsed,
            marker_edits.clone(),
            &marker_edits,
        )?;
        result.edits = stabilize::compose(source, &result.edits, &marker_edits, origins)?;
        let before_style = config.source(&result.output)?;
        let before_style_parsed = parse::parse(&before_style).map_err(ProcessingError::Parse)?;
        let before_style_document = config
            .document(before_style.text())
            .map_err(ProcessingError::Configuration)?;
        let style_edits = if options.fix && config.fix {
            md035::propose(&before_style, &before_style_parsed, &before_style_document)
        } else {
            vec![]
        };
        result.output = edit::apply_authorized_edits(
            &before_style,
            &before_style_parsed,
            style_edits.clone(),
            &style_edits,
        )?;
        result.edits = stabilize::compose(source, &result.edits, &style_edits, origins)?;
        if marker_edits.is_empty() && style_edits.is_empty() {
            break;
        }
    }
    let before_fences = config.source(&result.output)?;
    let before_fences_parsed = parse::parse(&before_fences).map_err(ProcessingError::Parse)?;
    let before_fences_document = config
        .document(before_fences.text())
        .map_err(ProcessingError::Configuration)?;
    let fence_edits = if options.fix && config.fix {
        md048::propose(
            &before_fences,
            &before_fences_parsed,
            &before_fences_document,
        )
    } else {
        vec![]
    };
    result.output = edit::apply_authorized_edits(
        &before_fences,
        &before_fences_parsed,
        fence_edits.clone(),
        &fence_edits,
    )?;
    result.edits = stabilize::compose(source, &result.edits, &fence_edits, origins)?;
    let before_separators = config.source(&result.output)?;
    let before_separators_parsed =
        parse::parse(&before_separators).map_err(ProcessingError::Parse)?;
    let before_separators_document = config
        .document(before_separators.text())
        .map_err(ProcessingError::Configuration)?;
    let separator_edits = if options.fix && config.fix {
        blank_lines::propose(
            &before_separators,
            &before_separators_parsed,
            &before_separators_document,
        )
    } else {
        vec![]
    };
    result.output = edit::apply_authorized_edits(
        &before_separators,
        &before_separators_parsed,
        separator_edits.clone(),
        &separator_edits,
    )?;
    result.edits = stabilize::compose(source, &result.edits, &separator_edits, origins)?;
    let before_tables = config.source(&result.output)?;
    let before_tables_parsed = parse::parse(&before_tables).map_err(ProcessingError::Parse)?;
    let before_tables_document = config
        .document(before_tables.text())
        .map_err(ProcessingError::Configuration)?;
    let table_edits = if options.fix && config.fix {
        md058::propose(
            &before_tables,
            &before_tables_parsed,
            &before_tables_document,
        )
    } else {
        vec![]
    };
    result.output = edit::apply_authorized_edits(
        &before_tables,
        &before_tables_parsed,
        table_edits.clone(),
        &table_edits,
    )?;
    result.edits = stabilize::compose(source, &result.edits, &table_edits, origins)?;
    // Table separators can make an ATX-to-setext conversion safe in the same call.
    let before_headings = config.source(&result.output)?;
    let before_headings_parsed = parse::parse(&before_headings).map_err(ProcessingError::Parse)?;
    let before_headings_document = config
        .document(before_headings.text())
        .map_err(ProcessingError::Configuration)?;
    let heading_edits = if options.fix && config.fix {
        md003::propose(
            &before_headings,
            &before_headings_parsed,
            &before_headings_document,
        )
    } else {
        vec![]
    };
    result.output = edit::apply_authorized_edits(
        &before_headings,
        &before_headings_parsed,
        heading_edits.clone(),
        &heading_edits,
    )?;
    result.edits = stabilize::compose(source, &result.edits, &heading_edits, origins)?;
    let before_fragments = config.source(&result.output)?;
    let before_fragments_parsed =
        parse::parse(&before_fragments).map_err(ProcessingError::Parse)?;
    let before_fragments_document = config
        .document(before_fragments.text())
        .map_err(ProcessingError::Configuration)?;
    let fragment_edits = if options.fix && config.fix {
        md051::propose(
            &before_fragments,
            &before_fragments_parsed,
            &before_fragments_document,
        )?
    } else {
        vec![]
    };
    result.output = edit::apply_authorized_edits(
        &before_fragments,
        &before_fragments_parsed,
        fragment_edits.clone(),
        &fragment_edits,
    )?;
    result.edits = stabilize::compose(source, &result.edits, &fragment_edits, origins)?;
    let before_eof = config.source(&result.output)?;
    let before_eof_parsed = parse::parse(&before_eof).map_err(ProcessingError::Parse)?;
    let before_eof_document = config
        .document(before_eof.text())
        .map_err(ProcessingError::Configuration)?;
    let eof_edits = if options.fix && config.fix {
        md047::propose(
            &before_eof,
            &before_eof_parsed,
            &before_eof_document,
            md047::preferred_newline(source),
        )
    } else {
        vec![]
    };
    result.output = edit::apply_authorized_edits(
        &before_eof,
        &before_eof_parsed,
        eof_edits.clone(),
        &eof_edits,
    )?;
    result.edits = stabilize::compose(source, &result.edits, &eof_edits, origins)?;
    let output = config.source(&result.output)?;
    let parsed = parse::parse(&output).map_err(ProcessingError::Parse)?;
    let document = config
        .document(output.text())
        .map_err(ProcessingError::Configuration)?;
    result.diagnostics = md013::diagnose(&output, &parsed, &document);
    result
        .diagnostics
        .extend(reference_rules::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(inline_spaces::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(emphasis_styles::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(blank_lines::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(md060::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(md033::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(md034::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(md044::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(md003::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(heading_structure::diagnose(&output, &parsed, &document)?);
    result
        .diagnostics
        .extend(md036::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(md059::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(md058::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(md055::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(md056::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(md014::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(md046::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(md011::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(md018::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(md020::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(md030::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(md029::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(md027::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(md048::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(md024::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(md028::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(md045::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(md042::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(md040::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(md023::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(md010::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(md009::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(md004::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(md012::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(md026::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(heading_spaces::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(md047::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(md035::diagnose(&output, &parsed, &document));
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
        .extend(md005::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(md007::diagnose(&output, &parsed, &document));
    result
        .diagnostics
        .extend(md051::diagnose(&output, &parsed, &document)?);
    let mut rejected = md005::evaluate(&output, &parsed, &document).conflicts;
    rejected.extend(md007::evaluate(&output, &parsed, &document).conflicts);
    rejected.extend(blank_lines::evaluate(&output, &parsed, &document).conflicts);
    result
        .diagnostics
        .extend(conflicts::diagnose_conflicts(&output, &document, &rejected));
    Ok(result)
}

fn process(
    text: &str,
    options: &FormatOptions,
    config: &config::ResolvedConfig,
    produce: impl FnOnce(&Source, &parse::ParsedDocument, &config::DocumentConfig) -> Vec<edit::Edit>,
    diagnose: impl FnOnce(&str) -> Vec<Diagnostic>,
) -> Result<FormatResult, ProcessingError> {
    let source = config.source(text)?;
    let document = config
        .document(text)
        .map_err(ProcessingError::Configuration)?;
    let parsed = parse::parse(&source).map_err(ProcessingError::Parse)?;
    let mut authorized = vec![];
    if options.fix && config.fix {
        authorized = md026::propose(&source, &parsed, &document);
        authorized.extend(heading_spaces::propose(&source, &parsed, &document));
        authorized.extend(md012::propose(&source, &parsed, &document));
        authorized.extend(md009::propose(&source, &parsed, &document));
        authorized.extend(md010::propose(&source, &parsed, &document));
        authorized.extend(md023::propose(&source, &parsed, &document));
        authorized.extend(md027::propose(&source, &parsed, &document));
        authorized.extend(md029::propose(&source, &parsed, &document));
        authorized.extend(md030::propose(&source, &parsed, &document));
        authorized.extend(md005::propose(&source, &parsed, &document));
        authorized.extend(md007::propose(&source, &parsed, &document));
    }
    let mut edits = if options.fix
        && config.fix
        && !parse::has_uncertain_container_prefixes(&source, &parsed)
    {
        let proposed = produce(&source, &parsed, &document);
        edit::validate_ranges(&source, &proposed)?;
        proposed
            .into_iter()
            .filter(|edit| {
                authorized.contains(edit)
                    || document.allows_source(&edit.origin, edit.span.clone(), &source)
            })
            .collect()
    } else {
        vec![]
    };
    edits.sort_by_key(|edit| (edit.span.start, edit.span.end));
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
