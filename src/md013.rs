use crate::{
    Diagnostic, Source,
    config::{DocumentConfig, js_space},
    parse::{ParsedDocument, RegionKind},
    utf16_len,
};
use std::ops::Range;

const HEADING: u8 = 1;
const CODE: u8 = 2;
const TABLE: u8 = 4;
const DEFINITION: u8 = 8;
const FRONT: u8 = 16;
const LINK: u8 = 32;
const DATA: u8 = 64;

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    let classes = classify(source, parsed);
    let mut diagnostics = vec![];
    for line in 1..=source.line_count() {
        let mut span = source.line_range(line).unwrap();
        if line == 1 && source.text().starts_with('\u{feff}') {
            span.start += 3;
        }
        let text = &source.text()[span.clone()];
        let Some(state) = document.state("MD013", line) else {
            continue;
        };
        let class = classes[line - 1];
        let number = |name: &str, fallback: usize| {
            state
                .options
                .get(name)
                .and_then(|v| v.as_u64())
                .and_then(|value| usize::try_from(value).ok())
                .unwrap_or(fallback)
        };
        let flag = |name: &str, fallback: bool| {
            state
                .options
                .get(name)
                .and_then(|v| v.as_bool())
                .unwrap_or(fallback)
        };
        let base = number("line_length", 80);
        let width = if class & CODE != 0 {
            number("code_block_line_length", base)
        } else if class & HEADING != 0 {
            number("heading_line_length", base)
        } else {
            base
        };
        let actual = utf16_len(text);
        if !state.enabled
            || actual <= width
            || class & (DEFINITION | FRONT) != 0
            || (class & CODE != 0 && !flag("code_blocks", true))
            || (class & TABLE != 0 && !flag("tables", true))
            || (class & HEADING != 0 && !flag("headings", true))
        {
            continue;
        }
        let strict = flag("strict", false);
        let stern = flag("stern", false);
        if !strict && ((class & LINK != 0 && class & DATA == 0) || (stern && not_wrappable(text))) {
            continue;
        }
        if !strict && !stern {
            // RE2JS \s is ASCII and excludes vertical tab, unlike JavaScript \s.
            let tail = text
                .char_indices()
                .rev()
                .find(|(_, ch)| matches!(ch, ' ' | '\t' | '\n' | '\r' | '\u{c}'))
                .map_or(0, |(index, ch)| index + ch.len_utf8());
            if utf16_len(&text[..tail]) < width {
                continue;
            }
        }
        let mut units = 0;
        let excess = text
            .char_indices()
            .find_map(|(index, ch)| {
                units += ch.len_utf16();
                (units > width).then_some(index)
            })
            .unwrap_or(text.len());
        diagnostics.push(Diagnostic {
            code: "MD013".into(),
            rule_id: Some("MD013".into()),
            severity: state.severity,
            message: format!("Line length [Expected: {width}; Actual: {actual}]"),
            span: Some(span.start + excess..span.end),
        });
    }
    diagnostics
}

fn not_wrappable(text: &str) -> bool {
    let Some((index, ch)) = text.char_indices().rev().find(|(_, ch)| js_space(*ch)) else {
        return true;
    };
    text[..index + ch.len_utf8()]
        .chars()
        .all(|ch| matches!(ch, '#' | '>') || js_space(ch))
}

pub(crate) fn covered_lines(source: &Source, span: Range<usize>) -> Range<usize> {
    let start = source.position(span.start).unwrap().line - 1;
    let last = source.text()[span.clone()]
        .char_indices()
        .next_back()
        .map_or(span.start, |(index, _)| span.start + index);
    let end = source
        .position(last)
        .or_else(|_| source.position(last - 1))
        .unwrap()
        .line;
    start..end
}

fn classify(source: &Source, parsed: &ParsedDocument) -> Vec<u8> {
    let mut classes = vec![0; source.line_count()];
    for region in &parsed.regions {
        let class = match region.kind {
            RegionKind::Heading => HEADING,
            RegionKind::Code => CODE,
            RegionKind::Table => TABLE,
            RegionKind::Definition => DEFINITION,
            RegionKind::FrontMatter => FRONT,
            RegionKind::Link | RegionKind::Image => LINK,
            _ => 0,
        };
        if class != 0 {
            for line in covered_lines(source, region.span.clone()) {
                classes[line] |= class;
            }
        }
        if region.kind != RegionKind::Text
            || !region
                .parent
                .is_some_and(|parent| parsed.regions[parent].kind == RegionKind::Paragraph)
        {
            continue;
        }
        for line in covered_lines(source, region.span.clone()) {
            let physical = source.line_range(line + 1).unwrap();
            let start = physical.start.max(region.span.start);
            let end = physical.end.min(region.span.end);
            let mut data = &source.text()[start..end];
            // markdown-rs can start an escaped Text node at the escaped character,
            // excluding its backslash from the node's positioned source range.
            if start == region.span.start
                && start > 0
                && source.text().as_bytes()[start - 1] == b'\\'
                && data
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_punctuation)
            {
                data = &data[1..];
            }
            if start == physical.start && region.span.start < start {
                data = data.trim_start_matches([' ', '\t', '>']);
            }
            if end == physical.end {
                data = data.trim_end_matches([' ', '\t']);
            }
            if has_data(data) {
                classes[line] |= DATA;
            }
        }
    }
    classes
}

fn has_data(mut text: &str) -> bool {
    while !text.is_empty() {
        let bytes = text.as_bytes();
        if bytes[0] == b'\\' && bytes.get(1).is_some_and(u8::is_ascii_punctuation) {
            text = &text[2..];
            continue;
        }
        if bytes[0] == b'&'
            && let Some(end) = bytes.iter().take(34).position(|&byte| byte == b';')
        {
            let name = &text[1..end];
            let numeric =
                if let Some(hex) = name.strip_prefix("#x").or_else(|| name.strip_prefix("#X")) {
                    !hex.is_empty()
                        && hex.len() <= 6
                        && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
                } else if let Some(decimal) = name.strip_prefix('#') {
                    !decimal.is_empty()
                        && decimal.len() <= 7
                        && decimal.bytes().all(|byte| byte.is_ascii_digit())
                } else {
                    false
                };
            if numeric || markdown::decode_named(name, true).is_some() {
                text = &text[end + 1..];
                continue;
            }
        }
        return true;
    }
    false
}
