use crate::{
    Diagnostic, Severity, Source,
    config::{DocumentConfig, js_space},
    edit::Edit,
    parse::{ParsedDocument, RegionKind, equivalent_with_thematic_styles, parse},
};
use std::ops::Range;

struct Finding {
    span: Range<usize>,
    expected: String,
    severity: Severity,
}

fn findings(source: &Source, parsed: &ParsedDocument, document: &DocumentConfig) -> Vec<Finding> {
    let mut expected = None;
    let mut result = vec![];
    for region in &parsed.regions {
        if region.kind != RegionKind::ThematicBreak {
            continue;
        }
        let line = source.position(region.span.start).unwrap().line;
        let end = source.line_range(line).unwrap().end;
        let raw = source.text()[region.span.start..end].trim_start_matches([' ', '\t']);
        let span = end - raw.len()..end;
        let Some(state) = document.state("MD035", line) else {
            continue;
        };
        let style = expected.get_or_insert_with(|| {
            let option = state
                .options
                .get("style")
                .and_then(|value| value.as_str())
                .filter(|value| !value.is_empty())
                .unwrap_or("consistent")
                .trim_matches(js_space);
            if option == "consistent" {
                raw.to_owned()
            } else {
                option.to_owned()
            }
        });
        if state.enabled && style != raw {
            result.push(Finding {
                span,
                expected: style.clone(),
                severity: state.severity,
            });
        }
    }
    result
}

fn valid_style(style: &str) -> bool {
    let mut markers = style.bytes().filter(|byte| !matches!(byte, b' ' | b'\t'));
    let Some(marker @ (b'*' | b'-' | b'_')) = markers.next() else {
        return false;
    };
    let mut count = 1;
    for byte in markers {
        if byte != marker {
            return false;
        }
        count += 1;
    }
    count >= 3
}

pub(crate) fn propose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Edit> {
    if crate::parse::has_unmodeled_syntax(source, parsed) {
        return vec![];
    }
    let edits: Vec<_> = findings(source, parsed, document)
        .into_iter()
        .filter(|finding| {
            valid_style(&finding.expected)
                && document.allows_source("MD035", finding.span.clone(), source)
        })
        .map(|finding| Edit {
            span: finding.span,
            replacement: finding.expected,
            origin: "MD035".into(),
        })
        .collect();
    if edits.is_empty() {
        return edits;
    }
    let safe = crate::edit::apply_authorized_edits(source, parsed, edits.clone(), &edits)
        .ok()
        .and_then(|output| {
            let candidate = Source::new(output);
            let reparsed = parse(&candidate).ok()?;
            Some(equivalent_with_thematic_styles(
                source, parsed, &candidate, &reparsed,
            ))
        })
        .unwrap_or(false);
    if safe { edits } else { vec![] }
}

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    findings(source, parsed, document)
        .into_iter()
        .map(|finding| Diagnostic {
            code: "MD035".into(),
            rule_id: Some("MD035".into()),
            severity: finding.severity,
            message: format!(
                "Horizontal rule style [Expected: {}; Actual: {}]",
                finding.expected,
                &source.text()[finding.span.clone()]
            ),
            span: Some(finding.span),
        })
        .collect()
}
