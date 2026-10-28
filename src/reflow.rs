mod layout;

use crate::{
    Source,
    config::DocumentConfig,
    edit::{Edit, apply_edits},
    parse::{ParsedDocument, equivalent, parse},
    utf16_len,
};
pub(crate) use layout::Layout;

pub(crate) fn propose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Edit> {
    let text = source.text();
    // GFM reparsing cannot validate upstream math/directives or HTML rendering.
    // Preserve these documents until their complete source boundaries are modeled.
    if crate::parse::has_unmodeled_syntax(source, parsed) {
        return vec![];
    }
    let layout = Layout::new(source, parsed);
    let mut edits = vec![];
    for paragraph in &layout.paragraphs {
        if !paragraph.eligible
            || !document.allows_source("MD013", paragraph.span.clone(), source)
            || document.has_line_override("MD047", paragraph.span.clone(), source)
            || document.has_line_override("MD009", paragraph.span.clone(), source)
        {
            continue;
        }
        let position = source.position(paragraph.span.start).unwrap();
        let last_line = source.position(paragraph.span.end).unwrap().line;
        // Some Unicode-whitespace lines are paragraph content to GFM but blank
        // to MD012. Joining them can make a rejected blank-line batch fixable.
        if (position.line..=last_line).any(|line| {
            document
                .state("MD012", line)
                .is_some_and(|state| state.enabled)
                && source.text()[source.line_range(line).unwrap()]
                    .chars()
                    .all(crate::config::js_space)
        }) {
            continue;
        }
        let Some(state) = document.state("MD013", position.line) else {
            continue;
        };
        let width = state
            .options
            .get("line_length")
            .and_then(|v| v.as_u64())
            .unwrap_or(80);
        if !state.enabled || width == 0 {
            continue;
        }
        let width = usize::try_from(width).unwrap_or(usize::MAX);
        let mut column = position.column_utf16 - 1;
        if position.line == 1 && text.starts_with('\u{feff}') {
            column -= 1;
        }
        let mut cursor = paragraph.span.start;
        for (index, gap) in paragraph.gaps.iter().enumerate() {
            let token = &text[cursor..gap.span.start];
            column = if let Some(end) = token.rfind(['\r', '\n']) {
                utf16_len(&token[end + 1..])
            } else {
                column + utf16_len(token)
            };
            let next_end = paragraph
                .gaps
                .get(index + 1)
                .map_or(paragraph.span.end, |next| next.span.start);
            let next = &text[gap.span.end..next_end];
            let next_width = utf16_len(next.split(['\r', '\n']).next().unwrap_or(""));
            let replacement = if column + utf16_len(&gap.flat) + next_width > width {
                column = utf16_len(&paragraph.continuation);
                &gap.wrapped
            } else {
                column += utf16_len(&gap.flat);
                &gap.flat
            };
            if replacement != &text[gap.span.clone()] {
                edits.push(Edit {
                    span: gap.span.clone(),
                    replacement: replacement.clone(),
                    origin: "MD013".into(),
                });
            }
            cursor = gap.span.end;
        }
    }
    if edits.is_empty() {
        return edits;
    }
    // Wrapping can turn literal text into a heading, list, or another block.
    // Validate the whole batch once, including interactions with neighboring blocks.
    let safe = apply_edits(source, parsed, edits.clone())
        .ok()
        .and_then(|output| {
            let candidate = Source::new(output);
            let reparsed = parse(&candidate).ok()?;
            Some(equivalent(source, parsed, &candidate, &reparsed))
        })
        .unwrap_or(false);
    if safe { edits } else { vec![] }
}

fn newline(text: &str, end: usize) -> &'static str {
    let suffix = &text[end..];
    if suffix.starts_with("\r\n") {
        "\r\n"
    } else if suffix.starts_with('\r') {
        "\r"
    } else if suffix.starts_with('\n') {
        "\n"
    } else if let Some(index) = text[..end].rfind(['\r', '\n']) {
        if text.as_bytes()[index] == b'\r' {
            "\r"
        } else if index > 0 && text.as_bytes()[index - 1] == b'\r' {
            "\r\n"
        } else {
            "\n"
        }
    } else {
        "\n"
    }
}
