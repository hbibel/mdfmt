use crate::{
    Diagnostic, Severity, Source,
    config::DocumentConfig,
    edit::Edit,
    parse::{ParsedDocument, RegionKind, equivalent, parse},
};
use std::ops::Range;

struct List {
    depth: usize,
    markers: Vec<Range<usize>>,
}

fn expanded_tabs(source: &Source) -> (Source, Vec<usize>) {
    let mut expanded = String::new();
    let mut offsets = vec![0];
    let mut width = 0;
    for (index, ch) in source.text().char_indices() {
        match ch {
            '\t' => {
                let spaces = 4 - width % 4;
                expanded.extend(std::iter::repeat_n(' ', spaces));
                offsets.extend(std::iter::repeat_n(index, spaces - 1));
                offsets.push(index + 1);
                width += spaces;
            }
            _ => {
                expanded.push(ch);
                offsets.extend(index + 1..=index + ch.len_utf8());
                if matches!(ch, '\r' | '\n') {
                    width = 0;
                } else if index != 0 || ch != '\u{feff}' {
                    width += ch.len_utf16();
                }
            }
        }
    }
    (Source::new(expanded), offsets)
}

fn lists(source: &Source, parsed: &ParsedDocument) -> Vec<List> {
    let mut result: Vec<List> = vec![];
    let mut indices: Vec<Option<usize>> = vec![None; parsed.regions.len()];
    for (index, region) in parsed.regions.iter().enumerate() {
        if region.kind == RegionKind::List {
            let mut depth = 0;
            let mut parent = region.parent;
            while let Some(index) = parent {
                let ancestor = &parsed.regions[index];
                depth += usize::from(ancestor.kind == RegionKind::List);
                parent = ancestor.parent;
            }
            indices[index] = Some(result.len());
            result.push(List {
                depth,
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
    if source.text().contains('\t') {
        // A diagnostic-only expanded parse avoids native tab/container boundary
        // disagreements. Marker spans are mapped back; original bytes stay intact.
        let (expanded, offsets) = expanded_tabs(source);
        let Ok(parsed) = parse(&expanded) else {
            return vec![];
        };
        return findings_in(&expanded, &parsed, document)
            .into_iter()
            .map(|mut finding| {
                finding.span = offsets[finding.span.start]..offsets[finding.span.end];
                finding
            })
            .collect();
    }
    findings_in(source, parsed, document)
}

fn findings_in(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Finding> {
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
    if crate::parse::has_unmodeled_syntax(source, parsed) {
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
