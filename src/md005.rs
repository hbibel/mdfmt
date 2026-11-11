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
    expected: usize,
    actual: usize,
    end_matching: bool,
    severity: Severity,
    item: ListItemIndentation,
    repair_expected: Option<usize>,
}
fn findings(source: &Source, document: &DocumentConfig) -> Vec<Finding> {
    if !(1..=source.line_count()).any(|line| {
        document
            .state("MD005", line)
            .is_some_and(|state| state.enabled)
    }) {
        return vec![];
    }
    let mut output = vec![];
    for list in list_indentation(source) {
        let first_line = source.position(list.span.start).unwrap().line;
        if document.state("MD005", first_line).is_none() {
            continue;
        }
        let expected_indent = list.items[0].indent;
        let expected_end = list.items[0].marker_end_column;
        let mut end_matching = false;
        let delegate = !list.ordered
            && list.items.iter().any(|item| {
                document
                    .state("MD007", source.position(item.span.start).unwrap().line)
                    .is_some_and(|state| state.enabled)
            });
        for item in list.items {
            let line = source.position(item.span.start).unwrap().line;
            if list.ordered
                && (item.indent != expected_indent || end_matching)
                && item.marker_end_column == expected_end
            {
                end_matching = true;
                continue;
            }
            let (expected, actual, repair) = if list.ordered && end_matching {
                (
                    expected_end,
                    item.marker_end_column,
                    expected_end.checked_sub(item.marker.len()),
                )
            } else {
                (expected_indent, item.indent, Some(expected_indent))
            };
            if expected != actual
                && let Some(state) = document.state("MD005", line).filter(|state| state.enabled)
            {
                let start = source.line_range(line).unwrap().start
                    + usize::from(line == 1 && source.text().starts_with('\u{feff}')) * 3;
                let repair_expected = if delegate { None } else { repair };
                output.push(Finding {
                    span: start..item.padding_end,
                    expected,
                    actual,
                    end_matching: list.ordered && end_matching,
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
                    indentation_repair(source, &finding.item, expected, "MD005", document)
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
        .map(|finding| {
            let detail = if finding.end_matching {
                format!(
                    "Expected: ({}); Actual: ({})",
                    finding.expected, finding.actual
                )
            } else {
                format!("Expected: {}; Actual: {}", finding.expected, finding.actual)
            };
            Diagnostic {
                code: "MD005".into(),
                rule_id: Some("MD005".into()),
                severity: finding.severity,
                message: format!(
                    "Inconsistent indentation for list items at the same level [{detail}]"
                ),
                span: Some(finding.span),
            }
        })
        .collect()
}
