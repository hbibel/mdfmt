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
    inactive: bool,
}

pub(super) struct Exclusion {
    pub span: Range<usize>,
    pub link: bool,
}

#[derive(Default)]
pub(super) struct Projection {
    pub skipped: Vec<Range<usize>>,
    pub literal: Vec<Range<usize>>,
}

impl Projection {
    pub fn extend(&mut self, other: Self) {
        self.skipped.extend(other.skipped);
        self.literal.extend(other.literal);
    }
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

fn project(text: &str, span: Range<usize>, projection: &Projection) -> (String, bool) {
    let excluded = &projection.skipped;
    let mut output = String::new();
    let mut cursor = span.start;
    let mut index = excluded.partition_point(|range| range.end <= cursor);
    let mut literal = projection
        .literal
        .partition_point(|range| range.end <= cursor);
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
        while literal < projection.literal.len() && projection.literal[literal].end <= cursor {
            literal += 1;
        }
        let data = projection
            .literal
            .get(literal)
            .is_some_and(|range| range.start <= cursor);
        if !data && matches!(bytes[cursor], b'*' | b'_') {
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
    excluded: &[Exclusion],
    projection: &Projection,
) -> Vec<ReferenceUse> {
    let text = source.text();
    let bytes = text.as_bytes();
    let mut cursor = span.start;
    let mut exclusion = 0;
    let mut stack = vec![];
    let mut output: Vec<ReferenceUse> = vec![];
    while cursor < span.end {
        while exclusion < excluded.len() && excluded[exclusion].span.end <= cursor {
            exclusion += 1;
        }
        if let Some(range) = excluded
            .get(exclusion)
            .filter(|range| range.span.start <= cursor)
        {
            if range.link {
                for frame in stack.iter_mut().rev() {
                    let frame: &mut Frame = frame;
                    if frame.body == frame.start + 1 {
                        if frame.inactive {
                            break;
                        }
                        frame.inactive = true;
                    }
                }
            }
            cursor = range.span.end.min(span.end);
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
                inactive: false,
            });
            cursor += 2;
            continue;
        } else if bytes[cursor] == b'[' {
            stack.push(Frame {
                start: cursor,
                body: cursor + 1,
                nested: false,
                inactive: false,
            });
        } else if bytes[cursor] == b']'
            && let Some(Frame {
                start,
                body,
                nested,
                inactive,
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
                projection,
            );
            let end = cursor + usize::from(!inactive);
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
                    cursor += 1;
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
