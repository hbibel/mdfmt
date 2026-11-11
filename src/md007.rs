use crate::{
    Diagnostic, Severity, Source,
    config::DocumentConfig,
    edit::Edit,
    parse::{
        ParsedDocument,
        list_indentation::{
            ListItemIndentation, indentation_repair, list_indentation, validated_indentation_edits,
        },
    },
};
use std::ops::Range;

struct Finding {
    span: Range<usize>,
    expected: u128,
    actual: usize,
    severity: Severity,
    item: ListItemIndentation,
    repair_expected: Option<usize>,
}
fn findings(source: &Source, document: &DocumentConfig) -> Vec<Finding> {
    if !(1..=source.line_count()).any(|line| {
        document
            .state("MD007", line)
            .is_some_and(|state| state.enabled)
    }) {
        return vec![];
    }
    let mut output = vec![];
    for list in list_indentation(source) {
        let Some(depth) = list.ul_depth else {
            continue;
        };
        let first = source.position(list.span.start).unwrap().line;
        let Some(settings) = document.state("MD007", first) else {
            continue;
        };
        let indent = settings
            .options
            .get("indent")
            .and_then(|value| value.as_u64())
            .unwrap_or(2);
        let start = if settings
            .options
            .get("start_indented")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
        {
            settings
                .options
                .get("start_indent")
                .and_then(|value| value.as_u64())
                .unwrap_or(indent)
        } else {
            0
        };
        let expected =
            list.footnote_base as u128 + u128::from(start) + depth as u128 * u128::from(indent);
        for item in list.items {
            let actual = item.indent - item.quote_adjustment;
            let line = source.position(item.span.start).unwrap().line;
            if expected != actual as u128
                && let Some(state) = document.state("MD007", line).filter(|state| state.enabled)
            {
                let start = source.line_range(line).unwrap().start
                    + usize::from(line == 1 && source.text().starts_with('\u{feff}')) * 3;
                let repair_expected = usize::try_from(expected)
                    .ok()
                    .and_then(|expected| expected.checked_add(item.quote_adjustment));
                output.push(Finding {
                    span: start..item.padding_end,
                    expected,
                    actual,
                    severity: state.severity,
                    item,
                    repair_expected,
                });
            }
        }
    }
    output.sort_by_key(|finding| finding.span.start);
    output
}
pub(crate) fn propose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Edit> {
    if crate::parse::has_unmodeled_syntax(source, parsed) {
        return vec![];
    }
    validated_indentation_edits(
        source,
        parsed,
        document,
        findings(source, document)
            .into_iter()
            .filter_map(|finding| {
                finding.repair_expected.and_then(|expected| {
                    indentation_repair(source, &finding.item, expected, "MD007", document)
                })
            })
            .collect(),
    )
}
pub(crate) fn diagnose(
    source: &Source,
    _parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    findings(source, document)
        .into_iter()
        .map(|finding| Diagnostic {
            code: "MD007".into(),
            rule_id: Some("MD007".into()),
            severity: finding.severity,
            message: format!(
                "Unordered list indentation [Expected: {}; Actual: {}]",
                finding.expected, finding.actual
            ),
            span: Some(finding.span),
        })
        .collect()
}
