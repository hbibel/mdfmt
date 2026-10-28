use crate::{
    Diagnostic, Severity, Source,
    config::DocumentConfig,
    edit::Edit,
    parse::{ParsedDocument, RegionKind, equivalent, parse},
};
use std::ops::Range;

fn spaces(text: &str, cursor: &mut usize) -> Range<usize> {
    let start = *cursor;
    *cursor += text[start..]
        .bytes()
        .take_while(|byte| *byte == b' ')
        .count();
    start..*cursor
}

fn marker_width(text: &str) -> Option<usize> {
    let digits = text.bytes().take_while(u8::is_ascii_digit).count();
    let width = if digits > 0 && matches!(text.as_bytes().get(digits), Some(b'.' | b')')) {
        digits + 1
    } else if matches!(text.as_bytes().first(), Some(b'-' | b'+' | b'*')) {
        1
    } else {
        return None;
    };
    let padding = text[width..]
        .bytes()
        .take_while(|byte| *byte == b' ')
        .count();
    let empty =
        text[width + padding..].is_empty() || text[width + padding..].starts_with(['\r', '\n']);
    Some(
        width
            + if !empty && (1..=4).contains(&padding) {
                padding
            } else {
                1
            },
    )
}

fn prefix(source: &Source, parsed: &ParsedDocument, index: usize) -> Option<Range<usize>> {
    let heading = &parsed.regions[index];
    let line = source.position(heading.span.start).ok()?.line;
    let range = source.line_range(line)?;
    let bom = usize::from(line == 1 && source.text().starts_with('\u{feff}')) * 3;
    let base = range.start + bom;
    let text = &source.text()[base..range.end];
    let mut ancestors = vec![];
    let mut parent = heading.parent;
    while let Some(index) = parent {
        ancestors.push(index);
        parent = parsed.regions[index].parent;
    }
    let mut cursor = 0;
    let mut candidate = None;
    for index in ancestors.into_iter().rev() {
        let ancestor = &parsed.regions[index];
        if ancestor.kind == RegionKind::Blockquote {
            let gap = spaces(text, &mut cursor);
            if !gap.is_empty() {
                candidate = Some(gap);
            }
            if text.as_bytes().get(cursor) == Some(&b'>') {
                cursor += 1;
                cursor += usize::from(text.as_bytes().get(cursor) == Some(&b' '));
            }
        } else if ancestor.kind == RegionKind::ListItem {
            let start_line = source.position(ancestor.span.start).ok()?.line;
            let raw = &source.text()[ancestor.span.clone()];
            let trimmed = raw.trim_start_matches(' ');
            let indent = raw.len() - trimmed.len();
            let width = marker_width(trimmed)?;
            if line == start_line {
                let gap = spaces(text, &mut cursor);
                if !gap.is_empty() {
                    candidate = Some(gap);
                }
                cursor = (cursor + width).min(text.len());
            } else {
                let count = text[cursor..]
                    .bytes()
                    .take_while(|byte| *byte == b' ')
                    .count();
                cursor += count.min(indent + width);
            }
        }
    }
    let gap = spaces(text, &mut cursor);
    if !gap.is_empty() {
        candidate = Some(gap);
    }
    candidate.map(|span| base + span.start..base + span.end)
}

fn spans_in(source: &Source, parsed: &ParsedDocument) -> Vec<Range<usize>> {
    parsed
        .regions
        .iter()
        .enumerate()
        .filter(|(_, region)| region.kind == RegionKind::Heading)
        .filter_map(|(index, _)| prefix(source, parsed, index))
        .collect()
}

fn spans(source: &Source, parsed: &ParsedDocument) -> Vec<Range<usize>> {
    if source.text().contains('\t') {
        let (expanded, offsets) = crate::parse::expanded_tabs(source);
        let Ok(parsed) = parse(&expanded) else {
            return vec![];
        };
        return spans_in(&expanded, &parsed)
            .into_iter()
            .map(|span| {
                let start = offsets[span.start]
                    + usize::from(span.start > 0 && offsets[span.start] == offsets[span.start - 1]);
                start..offsets[span.end]
            })
            .filter(|span| !span.is_empty())
            .collect();
    }
    spans_in(source, parsed)
}

fn findings(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<(Range<usize>, Severity)> {
    spans(source, parsed)
        .into_iter()
        .filter_map(|span| {
            let line = source.position(span.start).ok()?.line;
            let state = document
                .state("MD023", line)
                .filter(|state| state.enabled)?;
            Some((span, state.severity))
        })
        .collect()
}

pub(crate) fn propose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Edit> {
    if crate::parse::has_unmodeled_syntax(source, parsed)
        || crate::parse::has_uncertain_container_prefixes(source, parsed)
    {
        return vec![];
    }
    let edits: Vec<_> = findings(source, parsed, document)
        .into_iter()
        .filter(|(span, _)| {
            document.allows_source("MD023", span.clone(), source)
                && !parsed.regions.iter().any(|region| {
                    matches!(
                        region.kind,
                        RegionKind::Code | RegionKind::Table | RegionKind::FrontMatter
                    ) && region.span.start < span.end
                        && span.start < region.span.end
                })
        })
        .map(|(span, _)| Edit {
            span,
            replacement: String::new(),
            origin: "MD023".into(),
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
            // Heading spans include indentation. Compare all other protected bytes
            // exactly while allowing only the already-selected heading prefix edits.
            let canonical = |source: &Source, parsed: &ParsedDocument| ParsedDocument {
                tree: parsed.tree.clone(),
                regions: parsed
                    .regions
                    .iter()
                    .map(|region| {
                        let mut span = region.span.clone();
                        if region.kind == RegionKind::Heading {
                            span.start += source.text()[span.clone()]
                                .bytes()
                                .take_while(|byte| *byte == b' ')
                                .count();
                        }
                        crate::parse::Region {
                            span,
                            kind: region.kind,
                            parent: region.parent,
                        }
                    })
                    .collect(),
            };
            Some(equivalent(
                source,
                &canonical(source, parsed),
                &candidate,
                &canonical(&candidate, &reparsed),
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
        .map(|(span, severity)| {
            let line = source.position(span.start).unwrap().line;
            let mut text = &source.text()[source.line_range(line).unwrap()];
            if line == 1 {
                text = text.strip_prefix('\u{feff}').unwrap_or(text);
            }
            let context = if crate::utf16_len(text) > 30 {
                let units: Vec<_> = text.encode_utf16().take(30).collect();
                format!("{}...", String::from_utf16_lossy(&units))
            } else {
                text.into()
            };
            Diagnostic {
                code: "MD023".into(),
                rule_id: Some("MD023".into()),
                severity,
                message: format!(
                    "Headings must start at the beginning of the line [Context: \"{context}\"]"
                ),
                span: Some(span),
            }
        })
        .collect()
}
