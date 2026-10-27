use crate::{
    Diagnostic, Severity, Source,
    config::DocumentConfig,
    edit::Edit,
    parse::{ParsedDocument, RegionKind},
};
use std::ops::Range;

fn findings(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<(Range<usize>, &'static str, Severity)> {
    let mut result = vec![];
    for region in &parsed.regions {
        if region.kind != RegionKind::Heading {
            continue;
        }
        let raw = source.text()[region.span.clone()].trim_start_matches([' ', '\t']);
        if !raw.starts_with('#') || raw.contains(['\r', '\n']) {
            continue;
        }
        let base = region.span.end - raw.len();
        let opening_end = raw.bytes().take_while(|&byte| byte == b'#').count();
        let opening_gap_end = raw.len() - raw[opening_end..].trim_start_matches([' ', '\t']).len();
        let trimmed = raw.trim_end_matches([' ', '\t']);
        let closing_start = trimmed.trim_end_matches('#').len();
        let closed = closing_start < trimmed.len()
            && closing_start >= opening_end
            && raw[..closing_start].ends_with([' ', '\t']);
        let rule = if closed { "MD021" } else { "MD019" };
        let line = source.position(region.span.start).unwrap().line;
        let Some(state) = document.state(rule, line).filter(|state| state.enabled) else {
            continue;
        };
        let mut gaps = Vec::with_capacity(2);
        gaps.push(opening_end..opening_gap_end);
        if closed {
            let start = raw[..closing_start].trim_end_matches([' ', '\t']).len();
            gaps.push(start..closing_start);
        }
        for gap in gaps {
            if gap.len() > 1 {
                result.push((base + gap.start + 1..base + gap.end, rule, state.severity));
            }
        }
    }
    result
}

pub(crate) fn propose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Edit> {
    let mut edits: Vec<_> = findings(source, parsed, document)
        .into_iter()
        .map(|(span, rule, _)| Edit {
            span,
            replacement: String::new(),
            origin: rule.into(),
        })
        .collect();
    edits.sort_by_key(|edit| (edit.span.start, edit.span.end));
    edits.dedup();
    edits
}

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    findings(source, parsed, document)
        .into_iter()
        .map(|(span, rule, severity)| Diagnostic {
            code: rule.into(),
            rule_id: Some(rule.into()),
            severity,
            message: if rule == "MD019" {
                "Multiple spaces after hash on atx style heading"
            } else {
                "Multiple spaces inside hashes on closed atx style heading"
            }
            .into(),
            span: Some(span),
        })
        .collect()
}
