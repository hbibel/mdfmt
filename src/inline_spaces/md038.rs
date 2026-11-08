use crate::{
    Diagnostic, Source,
    config::{DocumentConfig, js_space},
    parse::{CodeSpan, InlineAnalysis},
};
use std::ops::Range;

fn newline(text: &str) -> bool {
    text.starts_with(['\r', '\n'])
}

fn parts(source: &Source, code: &CodeSpan) -> Vec<Range<usize>> {
    let raw = &source.text()[code.span.clone()];
    let ticks = raw.bytes().take_while(|byte| *byte == b'`').count();
    let body = code.span.start + ticks..code.span.end - ticks;
    let mut parts = vec![];
    for index in crate::md013::covered_lines(source, body.clone()) {
        let line = source.line_range(index + 1).unwrap();
        let prefix = code
            .prefixes
            .binary_search_by_key(&line.start, |prefix| prefix.start)
            .ok()
            .map(|index| &code.prefixes[index])
            .map_or(line.start, |prefix| prefix.end);
        let start = body.start.max(prefix);
        let end = body.end.min(line.end);
        if start < end {
            parts.push(start..end);
        }
        if line.end >= body.start && line.end < body.end {
            let next = source
                .line_range(index + 2)
                .map_or(source.text().len(), |line| line.start);
            if next <= body.end {
                parts.push(line.end..next);
            }
        }
    }
    parts
}

fn spaces(
    source: &Source,
    span: Range<usize>,
    start: bool,
    padding: bool,
) -> Option<(Range<usize>, bool)> {
    let text = &source.text()[span.clone()];
    let data = if start {
        text.trim_start_matches(js_space)
    } else {
        text.trim_end_matches(js_space)
    };
    if data.is_empty() || data.len() == text.len() {
        return None;
    }
    let backtick = if start {
        data.starts_with('`')
    } else {
        data.ends_with('`')
    };
    let whitespace = if start {
        &text[..text.len() - data.len()]
    } else {
        &text[data.len()..]
    };
    let count = crate::utf16_len(whitespace) - usize::from(backtick && !padding);
    if count == 0 {
        return None;
    }
    let bytes: usize = if start {
        whitespace.chars().take(count).map(char::len_utf8).sum()
    } else {
        whitespace
            .chars()
            .rev()
            .take(count)
            .map(char::len_utf8)
            .sum()
    };
    Some((
        if start {
            span.start..span.start + bytes
        } else {
            span.end - bytes..span.end
        },
        backtick,
    ))
}

pub(super) fn diagnose(
    source: &Source,
    document: &DocumentConfig,
    analysis: &InlineAnalysis,
) -> Vec<Diagnostic> {
    let mut output = vec![];
    for code in &analysis.code {
        let mut segments = parts(source, code);
        let Some(first) = segments.first() else {
            continue;
        };
        let last = segments.last().unwrap();
        let head = &source.text()[first.clone()];
        let tail = &source.text()[last.clone()];
        let padded = (head.starts_with(' ') || newline(head))
            && (tail.ends_with(' ') || newline(tail))
            && segments.iter().any(|span| {
                source.text()[span.clone()].contains(|ch| !matches!(ch, ' ' | '\r' | '\n'))
            });
        let mut start_padding = None;
        let mut end_padding = None;
        if padded {
            let first = segments.first_mut().unwrap();
            let end = if newline(&source.text()[first.clone()]) {
                first.end
            } else {
                first.start + 1
            };
            start_padding = Some(first.start..end);
            first.start = end;
            let last = segments.last_mut().unwrap();
            let begin = if newline(&source.text()[last.clone()]) {
                last.start
            } else {
                last.end - 1
            };
            end_padding = Some(begin..last.end);
            last.end = begin;
        }
        let data: Vec<_> = segments
            .into_iter()
            .filter(|span| !span.is_empty() && !newline(&source.text()[span.clone()]))
            .collect();
        let Some(first) = data.first() else {
            continue;
        };
        let last = data.last().unwrap();
        let start = spaces(source, first.clone(), true, padded);
        let end = spaces(source, last.clone(), false, padded);
        let remove_padding = start.as_ref().is_some_and(|(_, backtick)| !backtick)
            && end.as_ref().is_some_and(|(_, backtick)| !backtick)
            && padded;
        for (finding, padding, is_start) in
            [(start, start_padding, true), (end, end_padding, false)]
        {
            let Some((mut span, _)) = finding else {
                continue;
            };
            if remove_padding && let Some(padding) = padding {
                // Newline padding belongs to another line; upstream's columns can be invalid.
                if !newline(&source.text()[padding.clone()]) {
                    if is_start {
                        span.start = padding.start;
                    } else {
                        span.end = padding.end;
                    }
                }
            }
            let line = source.position(span.start).unwrap().line;
            let Some(state) = document.state("MD038", line).filter(|state| state.enabled) else {
                continue;
            };
            let context = super::context(&source.text()[code.span.clone()], is_start);
            output.push(Diagnostic {
                code: "MD038".into(),
                rule_id: Some("MD038".into()),
                severity: state.severity,
                message: format!("Spaces inside code span elements [Context: \"{context}\"]"),
                span: Some(span),
            });
        }
    }
    output
}
