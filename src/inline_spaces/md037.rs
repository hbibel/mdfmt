use crate::{
    Diagnostic, Source,
    config::{DocumentConfig, js_space},
    parse::{BareMarker, InlineAnalysis},
};
use std::collections::BTreeMap;

fn byte_at(text: &str, units: usize) -> Option<usize> {
    let mut column = 0;
    for (index, ch) in text.char_indices() {
        if column == units {
            return Some(index);
        }
        column += ch.len_utf16();
        if column > units {
            return None;
        }
    }
    (column == units).then_some(text.len())
}

fn finding(
    source: &Source,
    display: &Source,
    document: &DocumentConfig,
    token: &BareMarker,
    start: bool,
) -> Option<Diagnostic> {
    let at = if start {
        token.span.end
    } else {
        token.span.start
    };
    let position = source.position(at).ok()?;
    let state = document
        .state("MD037", position.line)
        .filter(|state| state.enabled)?;
    let raw_range = source.line_range(position.line)?;
    let raw = &source.text()[raw_range.clone()];
    let shown = &display.text()[display.line_range(position.line)?];
    let column = position.column_utf16 - 1;
    let offset = byte_at(shown, column)?;
    let marker = &source.text()[token.span.clone()];
    let (span, context) = if start {
        let rest = &shown[offset..];
        let text = rest.trim_start_matches(js_space);
        let spaces = &rest[..rest.len() - text.len()];
        if spaces.is_empty() || text.is_empty() {
            return None;
        }
        let count = crate::utf16_len(spaces);
        let end = raw_range.start + byte_at(raw, column + count)?;
        let character = String::from_utf16_lossy(&[text.encode_utf16().next()?]);
        (at..end, format!("{marker}{spaces}{character}"))
    } else {
        let before = &shown[..offset];
        let text = before.trim_end_matches(js_space);
        let spaces = &before[text.len()..];
        if spaces.is_empty() || text.is_empty() {
            return None;
        }
        let count = crate::utf16_len(spaces);
        let begin = raw_range.start + byte_at(raw, column.checked_sub(count)?)?;
        let character = String::from_utf16_lossy(&[text.encode_utf16().last()?]);
        (begin..at, format!("{character}{spaces}{marker}"))
    };
    Some(Diagnostic {
        code: "MD037".into(),
        rule_id: Some("MD037".into()),
        severity: state.severity,
        message: format!("Spaces inside emphasis markers [Context: \"{context}\"]"),
        span: Some(span),
    })
}

pub(super) fn diagnose(
    source: &Source,
    display: &Source,
    document: &DocumentConfig,
    analysis: &InlineAnalysis,
) -> Vec<Diagnostic> {
    let mut parents: BTreeMap<(usize, usize), Vec<&BareMarker>> = BTreeMap::new();
    for token in analysis
        .bare
        .iter()
        .filter(|token| (1..=3).contains(&token.span.len()))
    {
        parents
            .entry((token.parent.start, token.parent.end))
            .or_default()
            .push(token);
    }
    let mut output = vec![];
    for tokens in parents.values() {
        for marker in ["_", "__", "___", "*", "**", "***"] {
            let matches: Vec<_> = tokens
                .iter()
                .copied()
                .filter(|token| &source.text()[token.span.clone()] == marker)
                .collect();
            for pair in matches.as_chunks::<2>().0 {
                output.extend(finding(source, display, document, pair[0], true));
                output.extend(finding(source, display, document, pair[1], false));
            }
        }
    }
    output
}
