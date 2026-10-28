use crate::{
    Diagnostic, Severity, Source,
    config::DocumentConfig,
    edit::Edit,
    parse::{ParsedDocument, RegionKind, equivalent, parse},
};
use std::ops::Range;

struct List {
    depth: usize,
    ambiguous: bool,
    markers: Vec<Range<usize>>,
}

fn column(source: &Source, offset: usize) -> usize {
    let position = source.position(offset).unwrap();
    let start = source.line_range(position.line).unwrap().start;
    source.text()[start..offset]
        .chars()
        .fold(0, |column, ch| match ch {
            '\t' => column + 4 - column % 4,
            '\u{feff}' if position.line == 1 && column == 0 => column,
            _ => column + ch.len_utf16(),
        })
}

fn content_column(source: &Source, span: Range<usize>) -> usize {
    let raw = source.text()[span.clone()].trim_start_matches([' ', '\t']);
    let start = span.end - raw.len();
    let digits = raw.bytes().take_while(u8::is_ascii_digit).count();
    let width = if digits > 0 { digits + 1 } else { 1 };
    let after_marker = start + width;
    let separator = raw[width..]
        .bytes()
        .take_while(|byte| matches!(byte, b' ' | b'\t'))
        .count();
    let marker_column = column(source, after_marker);
    let padding = column(source, after_marker + separator) - marker_column;
    marker_column
        + if (1..=4).contains(&padding) {
            padding
        } else {
            1
        }
}

fn lists(source: &Source, parsed: &ParsedDocument) -> Vec<List> {
    let mut result: Vec<List> = vec![];
    let mut indices: Vec<Option<usize>> = vec![None; parsed.regions.len()];
    for (index, region) in parsed.regions.iter().enumerate() {
        if region.kind == RegionKind::List {
            // Tab-expanded marker padding can disagree with native ancestry.
            // Use source columns for depth and withhold repairs on disagreement.
            let mut depth = 0;
            let mut native_depth = 0;
            let mut found_parent = false;
            let raw = source.text()[region.span.clone()].trim_start_matches([' ', '\t']);
            let marker_column = column(source, region.span.end - raw.len());
            let mut parent = region.parent;
            while let Some(index) = parent {
                let ancestor = &parsed.regions[index];
                native_depth += usize::from(ancestor.kind == RegionKind::List);
                if !found_parent
                    && ancestor.kind == RegionKind::ListItem
                    && marker_column >= content_column(source, ancestor.span.clone())
                    && let Some(list) = ancestor.parent.and_then(|parent| indices[parent])
                {
                    depth = result[list].depth + 1;
                    found_parent = true;
                }
                parent = ancestor.parent;
            }
            indices[index] = Some(result.len());
            result.push(List {
                depth,
                ambiguous: depth != native_depth,
                markers: vec![],
            });
        } else if region.kind == RegionKind::ListItem {
            let Some(list) = region.parent.and_then(|parent| indices[parent]) else {
                continue;
            };
            let raw = source.text()[region.span.clone()].trim_start_matches([' ', '\t']);
            if raw
                .as_bytes()
                .first()
                .is_some_and(|marker| matches!(marker, b'-' | b'+' | b'*'))
            {
                let start = region.span.end - raw.len();
                result[list].markers.push(start..start + 1);
            }
        }
    }
    result
}

fn name(marker: u8) -> &'static str {
    match marker {
        b'-' => "dash",
        b'+' => "plus",
        _ => "asterisk",
    }
}

fn different(marker: u8) -> u8 {
    match marker {
        b'-' => b'+',
        b'+' => b'*',
        _ => b'-',
    }
}

struct Finding {
    span: Range<usize>,
    expected: u8,
    actual: u8,
    severity: Severity,
}

fn findings(source: &Source, parsed: &ParsedDocument, document: &DocumentConfig) -> Vec<Finding> {
    let Some(global) = document.state("MD004", 1) else {
        return vec![];
    };
    let style = global
        .options
        .get("style")
        .and_then(|value| value.as_str())
        .unwrap_or("consistent");
    let mut expected = match style {
        "dash" => Some(b'-'),
        "plus" => Some(b'+'),
        "asterisk" => Some(b'*'),
        _ => None,
    };
    let mut nesting_styles: Vec<Option<u8>> = vec![];
    let mut result = vec![];
    for list in lists(source, parsed) {
        for span in list.markers {
            let actual = source.text().as_bytes()[span.start];
            let target = if style == "sublist" {
                nesting_styles.resize(nesting_styles.len().max(list.depth + 1), None);
                if nesting_styles[list.depth].is_none() {
                    let parent = list
                        .depth
                        .checked_sub(1)
                        .and_then(|depth| nesting_styles[depth]);
                    nesting_styles[list.depth] = Some(if parent == Some(actual) {
                        different(actual)
                    } else {
                        actual
                    });
                }
                nesting_styles[list.depth].unwrap()
            } else {
                *expected.get_or_insert(actual)
            };
            let line = source.position(span.start).unwrap().line;
            if actual != target
                && let Some(state) = document.state("MD004", line).filter(|state| state.enabled)
            {
                result.push(Finding {
                    span,
                    expected: target,
                    actual,
                    severity: state.severity,
                });
            }
        }
    }
    result.sort_by_key(|finding| finding.span.start);
    result
}

pub(crate) fn propose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Edit> {
    if crate::parse::has_unmodeled_syntax(source, parsed)
        || lists(source, parsed).iter().any(|list| list.ambiguous)
    {
        return vec![];
    }
    let mut edits: Vec<_> = findings(source, parsed, document)
        .into_iter()
        .filter(|finding| document.allows_source("MD004", finding.span.clone(), source))
        .map(|finding| Edit {
            span: finding.span,
            replacement: char::from(finding.expected).to_string(),
            origin: "MD004".into(),
        })
        .collect();
    edits.sort_by_key(|edit| (edit.span.start, edit.span.end));
    if edits.is_empty() {
        return edits;
    }
    let safe = crate::edit::apply_authorized_edits(source, parsed, edits.clone(), &edits)
        .ok()
        .and_then(|output| {
            let candidate = Source::new(output);
            let reparsed = parse(&candidate).ok()?;
            Some(equivalent(source, parsed, &candidate, &reparsed))
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
            code: "MD004".into(),
            rule_id: Some("MD004".into()),
            severity: finding.severity,
            message: format!(
                "Unordered list style [Expected: {}; Actual: {}]",
                name(finding.expected),
                name(finding.actual)
            ),
            span: Some(finding.span),
        })
        .collect()
}
