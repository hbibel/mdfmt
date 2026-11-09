use super::{ReferenceUse, key};
use crate::Source;
use regex::Regex;
use std::{ops::Range, sync::LazyLock};

static ENTITY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^&(?:#[0-9]{1,7}|#[xX][0-9a-fA-F]{1,6}|[A-Za-z][A-Za-z0-9]{1,31});").unwrap()
});

struct Frame {
    start: usize,
    body: usize,
    nested: bool,
}

fn entity(text: &str) -> bool {
    crate::parse::mdast(text, &markdown::ParseOptions::gfm())
        .ok()
        .is_some_and(|tree| {
            tree.children()
                .and_then(|children| children.first())
                .and_then(markdown::mdast::Node::children)
                .and_then(|children| children.first())
                .is_some_and(
                    |node| matches!(node,markdown::mdast::Node::Text(value) if value.value!=text),
                )
        })
}

fn project(text: &str, span: Range<usize>, excluded: &[Range<usize>]) -> (String, bool) {
    let mut output = String::new();
    let mut cursor = span.start;
    let mut index = excluded.partition_point(|range| range.end <= cursor);
    while cursor < span.end {
        while index < excluded.len() && excluded[index].end <= cursor {
            index += 1;
        }
        if let Some(range) = excluded.get(index).filter(|range| range.start <= cursor) {
            cursor = range.end.min(span.end);
            continue;
        }
        let bytes = text.as_bytes();
        if bytes[cursor] == b'\\' && matches!(bytes.get(cursor + 1), Some(b'\r' | b'\n')) {
            cursor += 1;
            continue;
        }
        if bytes[cursor] == b'\\' && bytes.get(cursor + 1).is_some_and(u8::is_ascii_punctuation) {
            cursor += 2;
            continue;
        }
        if matches!(bytes[cursor], b'*' | b'_') {
            cursor += 1;
            continue;
        }
        if bytes[cursor] == b'&'
            && let Some(candidate) = ENTITY.find(&text[cursor..span.end])
            && entity(candidate.as_str())
        {
            cursor += candidate.end();
            continue;
        }
        let ch = text[cursor..].chars().next().unwrap();
        output.push(ch);
        cursor += ch.len_utf8();
    }
    let data = !output.is_empty();
    (output, data)
}

pub(super) fn scan(
    source: &Source,
    logical: &str,
    span: Range<usize>,
    excluded: &[Range<usize>],
) -> Vec<ReferenceUse> {
    let text = source.text();
    let bytes = text.as_bytes();
    let mut cursor = span.start;
    let mut exclusion = 0;
    let mut stack = vec![];
    let mut output: Vec<ReferenceUse> = vec![];
    while cursor < span.end {
        while exclusion < excluded.len() && excluded[exclusion].end <= cursor {
            exclusion += 1;
        }
        if let Some(range) = excluded
            .get(exclusion)
            .filter(|range| range.start <= cursor)
        {
            cursor = range.end.min(span.end);
            continue;
        }
        if bytes[cursor] == b'\\' && bytes.get(cursor + 1).is_some_and(u8::is_ascii_punctuation) {
            cursor += 2;
            continue;
        }
        if bytes[cursor] == b'!' && bytes.get(cursor + 1) == Some(&b'[') {
            stack.push(Frame {
                start: cursor,
                body: cursor + 2,
                nested: false,
            });
            cursor += 2;
            continue;
        } else if bytes[cursor] == b'[' {
            stack.push(Frame {
                start: cursor,
                body: cursor + 1,
                nested: false,
            });
        } else if bytes[cursor] == b']'
            && let Some(Frame {
                start,
                body,
                nested,
            }) = stack.pop()
        {
            if let Some(parent) = stack.last_mut() {
                parent.nested = true;
            }
            // A failed nested label leaves a literal ']', which the shim rejects.
            if nested {
                cursor += 1;
                continue;
            }
            let previous = output
                .last()
                .is_some_and(|previous| previous.span.end == start);
            let reference_context =
                body == start + 1 && start > span.start && bytes[start - 1] == b']';
            let (label, has_data) = project(
                if reference_context { text } else { logical },
                body..cursor,
                excluded,
            );
            let end = cursor + 1;
            let mut candidate = ReferenceUse {
                key: key(&label),
                span: start..end,
                shortcut: true,
            };
            if previous {
                if !has_data {
                    let previous = output.last_mut().unwrap();
                    previous.span.end = end;
                    previous.shortcut = false;
                    cursor = end;
                    continue;
                }
                candidate.span.start = output.pop().unwrap().span.start;
                candidate.shortcut = false;
            }
            if !candidate.key.is_empty() && !label.contains(']') {
                output.push(candidate);
            }
        }
        cursor += text[cursor..].chars().next().unwrap().len_utf8();
    }
    output
}
