use crate::{
    Diagnostic, Severity, Source,
    config::DocumentConfig,
    edit::Edit,
    parse::{ParsedDocument, RegionKind, parse},
};
use std::ops::Range;

struct Fence {
    opening: Range<usize>,
    closing: Option<Range<usize>>,
    info: Range<usize>,
    body: Range<usize>,
}

fn fences(source: &Source) -> Vec<Fence> {
    let (expanded, offsets) = crate::parse::expanded_tabs(source);
    let Ok(parsed) = parse(&expanded) else {
        return vec![];
    };
    let mut output = vec![];
    for region in &parsed.regions {
        if region.kind != RegionKind::Code {
            continue;
        }
        let raw = &expanded.text()[region.span.clone()];
        let trimmed = raw.trim_start_matches(' ');
        if raw.len() - trimmed.len() > 3 {
            continue;
        }
        let Some(marker @ (b'`' | b'~')) = trimmed.as_bytes().first().copied() else {
            continue;
        };
        let length = trimmed.bytes().take_while(|byte| *byte == marker).count();
        if length < 3 {
            continue;
        }
        let start = region.span.start + raw.len() - trimmed.len();
        let opening = offsets[start]..offsets[start + length];
        let first_line = source.position(opening.start).unwrap().line;
        let last_line = crate::md013::covered_lines(&expanded, region.span.clone()).end;
        // Closing-looking literal lines are only candidates; exact code-value
        // validation below rejects a batch that would rewrite one as a fence.
        let closing = if last_line > first_line {
            let range = source.line_range(last_line).unwrap();
            let text = &source.text()[range.clone()];
            text.find(char::from(marker)).and_then(|index| {
                let count = text[index..]
                    .bytes()
                    .take_while(|byte| *byte == marker)
                    .count();
                (count >= length
                    && text[..index]
                        .bytes()
                        .all(|byte| matches!(byte, b' ' | b'\t' | b'>'))
                    && text[index + count..].trim_matches([' ', '\t']).is_empty())
                .then_some(range.start + index..range.start + index + count)
            })
        } else {
            None
        };
        let body_start = source
            .line_range(first_line + 1)
            .map_or(offsets[region.span.end], |range| range.start);
        let body_end = closing.as_ref().map_or(offsets[region.span.end], |_| {
            source.line_range(last_line).unwrap().start
        });
        output.push(Fence {
            info: opening.end..source.line_range(first_line).unwrap().end,
            opening,
            closing,
            body: body_start..body_end.max(body_start),
        });
    }
    output
}

fn style(marker: u8) -> &'static str {
    if marker == b'`' { "backtick" } else { "tilde" }
}

fn findings(source: &Source, document: &DocumentConfig) -> Vec<(Fence, u8, Severity)> {
    let mut expected = None;
    let mut output = vec![];
    for fence in fences(source) {
        let line = source.position(fence.opening.start).unwrap().line;
        let Some(state) = document.state("MD048", line) else {
            continue;
        };
        let actual = source.text().as_bytes()[fence.opening.start];
        let expected = *expected.get_or_insert_with(|| {
            match state
                .options
                .get("style")
                .and_then(|value| value.as_str())
                .unwrap_or("consistent")
            {
                "backtick" => b'`',
                "tilde" => b'~',
                _ => actual,
            }
        });
        if state.enabled && actual != expected {
            output.push((fence, expected, state.severity));
        }
    }
    output
}

pub(crate) fn propose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Edit> {
    if crate::parse::has_unmodeled_syntax(source, parsed) {
        return vec![];
    }
    let mut edits = vec![];
    for (fence, target, _) in findings(source, document) {
        if (target == b'`' && source.text()[fence.info.clone()].contains('`'))
            || source.text()[fence.info.clone()].starts_with(char::from(target))
            || !document.allows_source("MD048", fence.opening.clone(), source)
            || fence
                .closing
                .as_ref()
                .is_some_and(|span| !document.allows_source("MD048", span.clone(), source))
        {
            continue;
        }
        let mut longest = 0;
        let mut run = 0;
        for byte in source.text()[fence.body].bytes() {
            run = if byte == target { run + 1 } else { 0 };
            longest = longest.max(run);
        }
        let length = 3.max(fence.opening.len()).max(longest + 1);
        edits.push(Edit {
            span: fence.opening,
            replacement: char::from(target).to_string().repeat(length),
            origin: "MD048".into(),
        });
        if let Some(span) = fence.closing {
            let length = span.len().max(length);
            edits.push(Edit {
                span,
                replacement: char::from(target).to_string().repeat(length),
                origin: "MD048".into(),
            });
        }
    }
    if edits.is_empty() {
        return edits;
    }
    let safe = crate::edit::apply_authorized_edits(source, parsed, edits.clone(), &edits)
        .ok()
        .and_then(|output| {
            let candidate = source.derive(output);
            let reparsed = parse(&candidate).ok()?;
            Some(crate::parse::equivalent_with_fence_styles(
                source, parsed, &candidate, &reparsed,
            ))
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
        .map(|(fence, expected, severity)| {
            let actual = source.text().as_bytes()[fence.opening.start];
            let line = source.position(fence.opening.start).unwrap().line;
            let start = source.line_range(line).unwrap().start
                + usize::from(line == 1 && source.text().starts_with('\u{feff}')) * 3;
            Diagnostic {
                code: "MD048".into(),
                rule_id: Some("MD048".into()),
                severity,
                message: format!(
                    "Code fence style [Expected: {}; Actual: {}]",
                    style(expected),
                    style(actual)
                ),
                span: Some(start..start),
            }
        })
        .collect()
}
