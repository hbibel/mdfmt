use crate::{
    Diagnostic, Severity, Source,
    config::DocumentConfig,
    edit::Edit,
    parse::{ParsedDocument, RegionKind, equivalent, parse},
};
use std::ops::Range;

fn spaces(text: &str, start: usize) -> usize {
    start
        + text[start..]
            .bytes()
            .take_while(|byte| *byte == b' ')
            .count()
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
    let end = spaces(text, width);
    let empty = text[end..].is_empty() || text[end..].starts_with(['\r', '\n']);
    Some(
        width
            + if !empty && (1..=4).contains(&(end - width)) {
                end - width
            } else {
                1
            },
    )
}

fn spans_in(source: &Source, parsed: &ParsedDocument) -> Vec<(Range<usize>, bool)> {
    let mut depths = vec![0; parsed.regions.len()];
    let mut chosen = vec![None; source.line_count()];
    for (index, region) in parsed.regions.iter().enumerate() {
        depths[index] = region.parent.map_or(0, |parent| depths[parent] + 1);
        if !matches!(
            region.kind,
            RegionKind::Paragraph
                | RegionKind::Heading
                | RegionKind::Code
                | RegionKind::InlineCode
                | RegionKind::Table
                | RegionKind::Html
                | RegionKind::Definition
                | RegionKind::ThematicBreak
                | RegionKind::ListItem
                | RegionKind::Blockquote
        ) {
            continue;
        }
        for line in crate::md013::covered_lines(source, region.span.clone()) {
            if chosen[line].is_none_or(|prior| depths[index] > depths[prior]) {
                chosen[line] = Some(index);
            }
        }
    }
    let mut output = vec![];
    // Code/item spans can start at their markers, omitting relative indentation.
    let mut fence_indents = vec![0; parsed.regions.len()];
    let mut list_widths = vec![0; parsed.regions.len()];
    for (line, terminal) in chosen.into_iter().enumerate() {
        let Some(terminal) = terminal else {
            continue;
        };
        let terminal_region = &parsed.regions[terminal];
        let mut path = vec![];
        let mut parent = Some(terminal);
        while let Some(index) = parent {
            let region = &parsed.regions[index];
            if matches!(region.kind, RegionKind::ListItem | RegionKind::Blockquote) {
                path.push(index);
            }
            parent = region.parent;
        }
        path.reverse();
        let html_flow = terminal_region.kind == RegionKind::Html
            && terminal_region.parent.is_some_and(|parent| {
                matches!(
                    parsed.regions[parent].kind,
                    RegionKind::Root | RegionKind::Blockquote | RegionKind::ListItem
                )
            });
        let continuation_line =
            source.position(terminal_region.span.start).unwrap().line < line + 1;
        if html_flow && continuation_line {
            continue;
        }
        let inline_code_continuation =
            terminal_region.kind == RegionKind::InlineCode && continuation_line;
        let range = source.line_range(line + 1).unwrap();
        let text = &source.text()[range.clone()];
        let mut cursor = usize::from(line == 0 && text.starts_with('\u{feff}')) * 3;
        for (step, index) in path.iter().copied().enumerate() {
            let region = &parsed.regions[index];
            if region.kind == RegionKind::ListItem {
                let raw = &source.text()[region.span.clone()];
                let trimmed = raw.trim_start_matches(' ');
                let Some(width) = marker_width(trimmed) else {
                    break;
                };
                if source.position(region.span.start).unwrap().line == line + 1 {
                    let end = spaces(text, cursor);
                    list_widths[index] = end - cursor + width;
                    cursor = (end + width).min(text.len());
                } else if spaces(text, cursor) - cursor >= list_widths[index] {
                    cursor += list_widths[index];
                }
                continue;
            }
            let marker = spaces(text, cursor);
            if marker - cursor > 3 {
                break;
            }
            cursor = marker;
            if text.as_bytes().get(cursor) != Some(&b'>') {
                break;
            }
            cursor += 1;
            cursor += usize::from(text.as_bytes().get(cursor) == Some(&b' '));
            let end = spaces(text, cursor);
            let continuation = path.get(step + 1).is_some_and(|index| {
                let next = &parsed.regions[*index];
                next.kind == RegionKind::ListItem
                    && source.position(next.span.start).unwrap().line < line + 1
                    && end - cursor >= list_widths[*index]
            });
            if continuation || cursor == end {
                continue;
            }
            let next_quote = path
                .get(step + 1)
                .is_some_and(|index| parsed.regions[*index].kind == RegionKind::Blockquote)
                && end - cursor <= 3
                && text.as_bytes().get(end) == Some(&b'>');
            if inline_code_continuation && !next_quote {
                continue;
            }
            if step + 1 == path.len() {
                if terminal_region.kind == RegionKind::Table
                    && source.position(terminal_region.span.start).unwrap().line + 1 == line + 1
                {
                    continue;
                }
                if terminal_region.kind == RegionKind::Html
                    && terminal_region.parent.is_some_and(|parent| {
                        matches!(
                            parsed.regions[parent].kind,
                            RegionKind::Root | RegionKind::Blockquote | RegionKind::ListItem
                        )
                    })
                {
                    continue;
                }
            }
            let mut length = end - cursor;
            if step + 1 == path.len() && terminal_region.kind == RegionKind::Code {
                let raw = &source.text()[terminal_region.span.clone()];
                let trimmed = raw.trim_start_matches(' ');
                let indent = raw.len() - trimmed.len();
                let first_line = source.position(terminal_region.span.start).unwrap().line;
                let Some(marker @ (b'`' | b'~')) = trimmed.as_bytes().first().copied() else {
                    continue;
                };
                let count = trimmed.bytes().take_while(|byte| *byte == marker).count();
                if indent > 3 || count < 3 {
                    continue;
                }
                if line + 1 == first_line {
                    fence_indents[terminal] = length;
                } else {
                    let rest = &text[end..];
                    let closing = rest.bytes().take_while(|byte| *byte == marker).count();
                    if length <= 3
                        && closing >= count
                        && rest[closing..].trim_matches(' ').is_empty()
                    {
                        continue;
                    }
                    length = length.min(fence_indents[terminal]);
                }
            }
            if length > 0 {
                let in_list = path[..step]
                    .iter()
                    .any(|index| parsed.regions[*index].kind == RegionKind::ListItem)
                    || path
                        .get(step + 1)
                        .is_some_and(|index| parsed.regions[*index].kind == RegionKind::ListItem);
                output.push((range.start + cursor..range.start + cursor + length, in_list));
            }
        }
    }
    output
}

fn findings(source: &Source, document: &DocumentConfig) -> Vec<(Range<usize>, Severity)> {
    let (expanded, offsets) = crate::parse::expanded_tabs(source);
    let Ok(parsed) = parse(&expanded) else {
        return vec![];
    };
    spans_in(&expanded, &parsed)
        .into_iter()
        .filter_map(|(span, in_list)| {
            let start = offsets[span.start]
                + usize::from(span.start > 0 && offsets[span.start] == offsets[span.start - 1]);
            let end = offsets[span.end]
                + usize::from(span.end > 0 && offsets[span.end] == offsets[span.end - 1]);
            if start >= end {
                return None;
            }
            let span = start..end;
            let line = source.position(start).ok()?.line;
            let state = document
                .state("MD027", line)
                .filter(|state| state.enabled)?;
            if in_list
                && state
                    .options
                    .get("list_items")
                    .and_then(|value| value.as_bool())
                    == Some(false)
            {
                return None;
            }
            Some((span, state.severity))
        })
        .collect()
}

pub(crate) fn propose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Edit> {
    if crate::parse::has_unmodeled_syntax(source, parsed) {
        return vec![];
    }
    let edits: Vec<_> = findings(source, document)
        .into_iter()
        .filter(|(span, _)| {
            document.allows_source("MD027", span.clone(), source)
                && !parsed.regions.iter().any(|region| {
                    region.kind.is_protected()
                        && region.span.start < span.end
                        && span.start < region.span.end
                })
        })
        .map(|(span, _)| Edit {
            span,
            replacement: String::new(),
            origin: "MD027".into(),
        })
        .collect();
    if edits.is_empty() {
        return edits;
    }
    let safe = crate::edit::apply_authorized_edits(source, parsed, edits.clone(), &edits)
        .ok()
        .and_then(|output| {
            let candidate = source.derive(output);
            let reparsed = parse(&candidate).ok()?;
            Some(equivalent(source, parsed, &candidate, &reparsed))
        })
        .unwrap_or(false);
    if safe { edits } else { vec![] }
}

pub(crate) fn diagnose(
    source: &Source,
    _parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    findings(source, document)
        .into_iter()
        .map(|(span, severity)| {
            let line = source.position(span.start).unwrap().line;
            let text = &source.text()[source.line_range(line).unwrap()];
            let text = if line == 1 {
                text.strip_prefix('\u{feff}').unwrap_or(text)
            } else {
                text
            };
            let context = if crate::utf16_len(text) > 30 {
                format!(
                    "{}...",
                    String::from_utf16_lossy(&text.encode_utf16().take(30).collect::<Vec<_>>())
                )
            } else {
                text.into()
            };
            Diagnostic {
                code: "MD027".into(),
                rule_id: Some("MD027".into()),
                severity,
                message: format!(
                    "Multiple spaces after blockquote symbol [Context: \"{context}\"]"
                ),
                span: Some(span),
            }
        })
        .collect()
}
