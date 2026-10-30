use crate::{
    Diagnostic, Source,
    config::{DocumentConfig, js_space},
    parse::{ParsedDocument, RegionKind, parse},
};
use std::{collections::HashMap, ops::Range};

fn label_end(text: &str, code: bool) -> Option<usize> {
    if !text.starts_with('[') {
        return None;
    }
    let bytes = text.as_bytes();
    let mut cursor = 1;
    let mut depth = 1;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b'\\' => {
                cursor += 1;
            }
            b'`' if code => {
                let count = bytes[cursor..]
                    .iter()
                    .take_while(|byte| **byte == b'`')
                    .count();
                let mut end = cursor + count;
                while end < bytes.len() {
                    if bytes[end] == b'`' {
                        let closing = bytes[end..]
                            .iter()
                            .take_while(|byte| **byte == b'`')
                            .count();
                        if closing == count {
                            cursor = end + count - 1;
                            break;
                        }
                        end += closing;
                    } else {
                        end += 1;
                    }
                }
                if end == bytes.len() {
                    cursor += count - 1;
                }
            }
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(cursor);
                }
            }
            _ => {}
        }
        cursor += 1;
    }
    None
}

fn destination(text: &str, literal: bool) -> &str {
    let text = text.trim_start_matches([' ', '\t', '\r', '\n']);
    if let Some(inner) = text.strip_prefix('<') {
        if !literal {
            return "";
        }
        return inner.split('>').next().unwrap_or("");
    }
    text.split([' ', '\t', '\r', '\n', ')'])
        .next()
        .unwrap_or("")
}

fn regions(source: &Source, parsed: &ParsedDocument) -> Vec<(RegionKind, Range<usize>)> {
    let select = |parsed: &ParsedDocument| {
        parsed
            .regions
            .iter()
            .filter(|region| matches!(region.kind, RegionKind::Link | RegionKind::Definition))
            .map(|region| (region.kind, region.span.clone()))
            .collect::<Vec<_>>()
    };
    if source.text().contains('\t') {
        let (expanded, offsets) = crate::parse::expanded_tabs(source);
        let Ok(parsed) = parse(&expanded) else {
            return vec![];
        };
        return select(&parsed)
            .into_iter()
            .map(|(kind, span)| (kind, offsets[span.start]..offsets[span.end]))
            .collect();
    }
    select(parsed)
}

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    let regions = regions(source, parsed);
    let mut definitions = HashMap::new();
    for (kind, span) in &regions {
        if *kind != RegionKind::Definition {
            continue;
        }
        let raw = source.text()[span.clone()].trim_start_matches([' ', '\t']);
        let Some(end) = label_end(raw, false) else {
            continue;
        };
        let Some(rest) = raw[end + 1..].strip_prefix(':') else {
            continue;
        };
        let key = raw[1..end]
            .to_lowercase()
            .split(js_space)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        definitions
            .entry(key)
            .or_insert_with(|| destination(rest, false).to_owned());
    }
    let mut result = vec![];
    for (kind, span) in regions {
        if kind != RegionKind::Link {
            continue;
        }
        let raw = &source.text()[span.clone()];
        let Some(end) = label_end(raw, true) else {
            continue;
        };
        let rest = &raw[end + 1..];
        let empty = if let Some(resource) = rest.strip_prefix('(') {
            matches!(destination(resource, true).trim_matches(js_space), "" | "#")
        } else {
            let label = if rest.starts_with('[') {
                let Some(reference_end) = label_end(rest, false) else {
                    continue;
                };
                if reference_end > 1 {
                    &rest[1..reference_end]
                } else {
                    &raw[1..end]
                }
            } else {
                &raw[1..end]
            };
            definitions
                .get(label.trim_matches(js_space))
                .is_some_and(|destination| destination == "#")
        };
        if !empty {
            continue;
        }
        let line = source.position(span.start).unwrap().line;
        let Some(state) = document.state("MD042", line).filter(|state| state.enabled) else {
            continue;
        };
        let context = raw.replace("\r\n", "\n").replace('\r', "\n");
        let context = if crate::utf16_len(&context) > 30 {
            format!(
                "{}...",
                String::from_utf16_lossy(&context.encode_utf16().take(30).collect::<Vec<_>>())
            )
        } else {
            context
        };
        result.push(Diagnostic {
            code: "MD042".into(),
            rule_id: Some("MD042".into()),
            severity: state.severity,
            message: format!("No empty links [Context: \"{context}\"]"),
            span: Some(span),
        });
    }
    result
}
