use super::Pair;
use crate::{
    Source,
    config::js_space,
    parse::{ParsedDocument, RegionKind},
};
use markdown::mdast::Node;
use regex::Regex;
use std::{ops::Range, sync::LazyLock};

static PUNCTUATION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[\p{P}\p{S}]$").unwrap());

#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    Other,
    Punctuation,
    Whitespace,
}

fn classify(ch: Option<char>) -> Class {
    match ch {
        None => Class::Whitespace,
        Some(ch) if js_space(ch) => Class::Whitespace,
        // Pinned micromark receives one UTF-16 code unit at these boundaries.
        Some(ch)
            if ch.len_utf16() == 1
                && (ch.is_ascii_punctuation() || PUNCTUATION.is_match(&ch.to_string())) =>
        {
            Class::Punctuation
        }
        _ => Class::Other,
    }
}

struct Run {
    span: Range<usize>,
    marker: u8,
    open: bool,
    close: bool,
    active: bool,
}

fn resolve(text: &str, span: Range<usize>, excluded: &[Range<usize>], isolated: bool) -> Vec<Pair> {
    let mut runs = vec![];
    let mut cursor = span.start;
    let bytes = text.as_bytes();
    let mut exclusion = 0;
    while cursor < span.end {
        while exclusion < excluded.len() && excluded[exclusion].end <= cursor {
            exclusion += 1;
        }
        if let Some(range) = excluded
            .get(exclusion)
            .filter(|range| range.start <= cursor && cursor < range.end)
        {
            cursor = range.end.min(span.end);
            continue;
        }
        if bytes[cursor] == b'\\' && bytes.get(cursor + 1).is_some_and(u8::is_ascii_punctuation) {
            cursor += 2;
            continue;
        }
        let marker = bytes[cursor];
        if matches!(marker, b'*' | b'_') {
            let start = cursor;
            while cursor < span.end && bytes[cursor] == marker {
                cursor += 1;
            }
            let before = classify(if isolated && start == span.start {
                None
            } else {
                text[..start].chars().next_back()
            });
            let after = classify(if isolated && cursor == span.end {
                None
            } else {
                text[cursor..].chars().next()
            });
            let open =
                after == Class::Other || (after == Class::Punctuation && before != Class::Other);
            let close =
                before == Class::Other || (before == Class::Punctuation && after != Class::Other);
            runs.push(Run {
                span: start..cursor,
                marker,
                open: open && (marker == b'*' || before != Class::Other || !close),
                close: close && (marker == b'*' || after != Class::Other || !open),
                active: true,
            });
        } else {
            cursor += text[cursor..].chars().next().unwrap().len_utf8();
        }
    }
    let mut output = vec![];
    let mut index = 0;
    while index < runs.len() {
        if !runs[index].active || !runs[index].close {
            index += 1;
            continue;
        }
        let opening = (0..index).rev().find(|&open| {
            runs[open].active
                && runs[open].open
                && runs[open].marker == runs[index].marker
                && !((runs[open].close || runs[index].open)
                    && runs[index].span.len() % 3 != 0
                    && (runs[open].span.len() + runs[index].span.len()).is_multiple_of(3))
        });
        let Some(open) = opening else {
            index += 1;
            continue;
        };
        let width = if runs[open].span.len() > 1 && runs[index].span.len() > 1 {
            2
        } else {
            1
        };
        output.push(Pair {
            rule: if width == 1 { "MD049" } else { "MD050" },
            opening: runs[open].span.end - width..runs[open].span.end,
            closing: runs[index].span.start..runs[index].span.start + width,
        });
        runs[open].span.end -= width;
        runs[index].span.start += width;
        for run in &mut runs[open + 1..index] {
            run.active = false;
        }
        if runs[open].span.is_empty() {
            runs[open].active = false;
        }
        if runs[index].span.is_empty() {
            runs[index].active = false;
            index += 1;
        }
    }
    output
}

struct Scopes<'a> {
    source: &'a Source,
    definitions: String,
    logical: std::collections::HashMap<usize, String>,
    output: Vec<Pair>,
}

impl Scopes<'_> {
    fn scope(
        &mut self,
        node: &Node,
        base: usize,
        span: Range<usize>,
        quotes: usize,
        isolated: bool,
    ) {
        let mut excluded = vec![];
        if let Some(children) = node.children() {
            for child in children {
                self.children(child, base, quotes, &mut excluded);
            }
        }
        excluded.sort_by_key(|range| range.start);
        let logical = if quotes == 0 {
            self.source.text()
        } else {
            self.logical
                .entry(quotes)
                .or_insert_with(|| crate::md045::mask_quote_prefixes(self.source.text(), quotes))
        };
        self.output
            .extend(resolve(logical, span, &excluded, isolated));
    }

    fn children(
        &mut self,
        node: &Node,
        base: usize,
        quotes: usize,
        excluded: &mut Vec<Range<usize>>,
    ) {
        let Some(position) = node.position() else {
            return;
        };
        let span = base + position.start.offset..base + position.end.offset;
        match node {
            Node::InlineCode(_)
            | Node::InlineMath(_)
            | Node::Html(_)
            | Node::FootnoteReference(_) => excluded.push(span),
            Node::Link(_) | Node::LinkReference(_) => {
                excluded.push(span.clone());
                let raw = &self.source.text()[span.clone()];
                if let Some(end) = crate::parse::label_end(raw, true) {
                    self.scope(node, base, span.start + 1..span.start + end, quotes, false);
                }
            }
            Node::Image(_) | Node::ImageReference(_) => {
                excluded.push(span.clone());
                let raw = &self.source.text()[span.clone()];
                if let Some(end) = raw
                    .strip_prefix('!')
                    .and_then(|label| crate::parse::label_end(label, true))
                {
                    let label = crate::md045::mask_quote_prefixes(&raw[2..end + 1], quotes);
                    let options = markdown::ParseOptions {
                        constructs: markdown::Constructs {
                            html_flow: false,
                            math_text: true,
                            ..markdown::Constructs::gfm()
                        },
                        ..markdown::ParseOptions::gfm()
                    };
                    if let Ok(tree) =
                        crate::parse::mdast(&format!("x {label}\n\n{}", self.definitions), &options)
                        && let Some(first) = tree.children().and_then(|children| children.first())
                    {
                        self.scope(
                            first,
                            span.start,
                            span.start + 2..span.start + end + 1,
                            quotes,
                            false,
                        );
                    }
                }
            }
            _ => {
                if let Some(children) = node.children() {
                    for child in children {
                        self.children(child, base, quotes, excluded);
                    }
                }
            }
        }
    }

    fn blocks(&mut self, node: &Node, base: usize, quotes: usize) {
        let quotes = quotes + usize::from(matches!(node, Node::Blockquote(_)));
        if matches!(
            node,
            Node::Paragraph(_) | Node::Heading(_) | Node::TableCell(_)
        ) && let Some(position) = node.position()
        {
            let span = if matches!(node, Node::TableCell(_)) {
                // Native cell positions include edge pipes; inline chunks do not.
                let Some(children) = node.children() else {
                    return;
                };
                let Some((first, last)) = children
                    .first()
                    .and_then(Node::position)
                    .zip(children.last().and_then(Node::position))
                else {
                    return;
                };
                base + first.start.offset..base + last.end.offset
            } else {
                base + position.start.offset..base + position.end.offset
            };
            self.scope(node, base, span, quotes, true);
        } else if !matches!(
            node,
            Node::Code(_) | Node::Math(_) | Node::Html(_) | Node::Definition(_)
        ) && let Some(children) = node.children()
        {
            for child in children {
                self.blocks(child, base, quotes);
            }
        }
    }
}

pub(super) fn pairs(source: &Source, parsed: &ParsedDocument) -> Vec<Pair> {
    let base = parsed
        .regions
        .iter()
        .find(|r| r.kind == RegionKind::Root && r.parent.is_some())
        .map_or(0, |r| r.span.start);
    let definitions = parsed
        .regions
        .iter()
        .filter(|r| r.kind == RegionKind::Definition)
        .map(|r| &source.text()[r.span.clone()])
        .collect::<Vec<_>>()
        .join("\n");
    let mut scopes = Scopes {
        source,
        definitions,
        logical: std::collections::HashMap::new(),
        output: vec![],
    };
    scopes.blocks(&parsed.tree, base, 0);
    scopes.output.sort_by_key(|pair| pair.opening.start);
    scopes.output
}
