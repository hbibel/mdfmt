use super::{LinkKind, LinkStyle};
use markdown::mdast::{Node, ReferenceKind};
use std::ops::Range;

fn whitespace(text: &str, mut cursor: usize, end: usize) -> usize {
    while cursor < end && matches!(text.as_bytes()[cursor], b' ' | b'\t' | b'\r' | b'\n') {
        cursor += 1;
    }
    cursor
}

fn quoted(text: &str, mut cursor: usize, end: usize, close: u8) -> Option<usize> {
    while cursor < end {
        let byte = text.as_bytes()[cursor];
        if byte == b'\\' && cursor + 1 < end && text.as_bytes()[cursor + 1].is_ascii_punctuation() {
            cursor += 2;
            continue;
        }
        if byte == close {
            return Some(cursor);
        }
        cursor += text[cursor..].chars().next().unwrap().len_utf8();
    }
    None
}

fn destination(text: &str, mut cursor: usize, end: usize) -> Option<(Range<usize>, usize)> {
    if text.as_bytes().get(cursor) == Some(&b'<') {
        let start = cursor + 1;
        let end = quoted(text, start, end, b'>')?;
        Some((start..end, end + 1))
    } else {
        let start = cursor;
        let mut depth = 0;
        while cursor < end {
            let byte = text.as_bytes()[cursor];
            if byte == b'\\'
                && cursor + 1 < end
                && text.as_bytes()[cursor + 1].is_ascii_punctuation()
            {
                cursor += 2;
                continue;
            }
            if matches!(byte, b' ' | b'\t' | b'\r' | b'\n') {
                break;
            }
            if byte == b'(' {
                depth += 1;
            }
            if byte == b')' {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
            cursor += text[cursor..].chars().next().unwrap().len_utf8();
        }
        Some((start..cursor, cursor))
    }
}

pub(super) fn definition_destination(text: &str) -> Option<Range<usize>> {
    let end = crate::parse::label_end(text, true)?;
    if text.as_bytes().get(end + 1) != Some(&b':') {
        return None;
    }
    let start = whitespace(text, end + 2, text.len());
    destination(text, start, text.len()).map(|(span, _)| span)
}

pub(super) fn record(
    node: &Node,
    text: &str,
    logical: &str,
    span: Range<usize>,
) -> Option<LinkStyle> {
    let raw = &text[span.clone()];
    if raw.starts_with('<') && raw.ends_with('>') {
        let data = span.start + 1..span.end - 1;
        return Some(LinkStyle {
            span,
            kind: LinkKind::Autolink,
            label: Some(data.clone()),
            destination: Some(data),
            title: None,
        });
    }
    let image = usize::from(raw.starts_with('!'));
    let end = crate::parse::label_end_math(&raw[image..])?;
    let label = Some(span.start + image + 1..span.start + image + end);
    let reference = match node {
        Node::LinkReference(link) => Some(link.reference_kind),
        Node::ImageReference(image) => Some(image.reference_kind),
        _ => None,
    };
    if let Some(reference) = reference {
        let kind = match reference {
            ReferenceKind::Full => LinkKind::Full,
            ReferenceKind::Collapsed => LinkKind::Collapsed,
            ReferenceKind::Shortcut => LinkKind::Shortcut,
        };
        return Some(LinkStyle {
            span,
            kind,
            label,
            destination: None,
            title: None,
        });
    }
    let mut cursor = span.start + image + end + 1;
    if logical.as_bytes().get(cursor) != Some(&b'(') {
        return None;
    }
    cursor = whitespace(logical, cursor + 1, span.end - 1);
    let (destination, end) = destination(logical, cursor, span.end - 1)?;
    let destination = Some(destination);
    cursor = end;
    cursor = whitespace(logical, cursor, span.end - 1);
    let title =
        if cursor < span.end - 1 && matches!(logical.as_bytes()[cursor], b'\'' | b'"' | b'(') {
            let close = if logical.as_bytes()[cursor] == b'(' {
                b')'
            } else {
                logical.as_bytes()[cursor]
            };
            quoted(logical, cursor + 1, span.end - 1, close).map(|end| cursor + 1..end)
        } else {
            None
        };
    Some(LinkStyle {
        span,
        kind: LinkKind::Inline,
        label,
        destination,
        title,
    })
}
