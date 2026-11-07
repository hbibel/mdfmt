use super::{ParsedDocument, RegionKind, diagnostic_parse, label_end};
use crate::Source;
use markdown::mdast::Node;
use std::ops::Range;

pub(crate) struct HtmlToken {
    pub span: Range<usize>,
    pub in_table: bool,
}

#[derive(Clone)]
struct Candidate {
    span: Range<usize>,
    image: bool,
    flow: bool,
    quotes: usize,
    in_table: bool,
}

fn diagnostic_view(source: &Source, parsed: &ParsedDocument) -> (Source, Vec<usize>) {
    let front_end = parsed
        .regions
        .iter()
        .find(|r| r.kind == RegionKind::FrontMatter)
        .map_or(
            usize::from(source.text().starts_with('\u{feff}')) * 3,
            |r| r.span.end,
        );
    let mut text = String::new();
    let mut offsets = vec![0];
    let mut chars = source.text().char_indices().peekable();
    while let Some((start, ch)) = chars.next() {
        let mut end = start + ch.len_utf8();
        if ch == '\r' {
            if chars.peek().is_some_and(|(_, ch)| *ch == '\n') {
                chars.next();
                end += 1;
            }
            text.push('\n');
            offsets.push(end);
        } else if start < front_end && ch != '\n' {
            text.extend(std::iter::repeat_n(' ', ch.len_utf16()));
            offsets.extend(std::iter::repeat_n(start, ch.len_utf16() - 1));
            offsets.push(end);
        } else {
            text.push(ch);
            offsets.extend(start + 1..=end);
        }
    }
    (Source::new(text), offsets)
}

fn candidates(tree: &Node, base: usize, inherited_table: bool) -> Vec<Candidate> {
    fn visit(
        node: &Node,
        base: usize,
        quotes: usize,
        table: bool,
        parent: Option<&Node>,
        out: &mut Vec<Candidate>,
    ) {
        let quotes = quotes + usize::from(matches!(node, Node::Blockquote(_)));
        let table = table || matches!(node, Node::Table(_));
        if matches!(
            node,
            Node::Html(_) | Node::Image(_) | Node::ImageReference(_)
        ) && let Some(p) = node.position()
        {
            out.push(Candidate {
                span: base + p.start.offset..base + p.end.offset,
                image: !matches!(node, Node::Html(_)),
                flow: matches!(
                    parent,
                    Some(Node::Root(_) | Node::Blockquote(_) | Node::ListItem(_))
                ),
                quotes,
                in_table: table,
            });
        }
        if let Some(children) = node.children() {
            for child in children {
                visit(child, base, quotes, table, Some(node), out);
            }
        }
    }
    let mut out = vec![];
    visit(tree, base, 0, inherited_table, None, &mut out);
    out
}

fn recover(text: &str, table: bool) -> Vec<Candidate> {
    let options = markdown::ParseOptions {
        constructs: markdown::Constructs {
            html_flow: false,
            code_indented: false,
            math_text: true,
            math_flow: true,
            ..markdown::Constructs::gfm()
        },
        ..markdown::ParseOptions::gfm()
    };
    markdown::to_mdast(text, &options)
        .map(|tree| candidates(&tree, 0, table))
        .unwrap_or_default()
}

pub(crate) fn html_tokens(source: &Source, parsed: &ParsedDocument) -> Vec<HtmlToken> {
    if !source.text().contains('<') {
        return vec![];
    }
    let (view, offsets) = diagnostic_view(source, parsed);
    let Some(parsed) = diagnostic_parse(&view) else {
        return vec![];
    };
    let base = parsed
        .regions
        .iter()
        .find(|r| r.kind == RegionKind::Root && r.parent.is_some())
        .map_or(0, |r| r.span.start);
    let definitions = parsed
        .regions
        .iter()
        .filter(|r| r.kind == RegionKind::Definition)
        .map(|r| {
            let mut quotes = 0;
            let mut parent = r.parent;
            while let Some(index) = parent {
                quotes += usize::from(parsed.regions[index].kind == RegionKind::Blockquote);
                parent = parsed.regions[index].parent;
            }
            crate::md045::mask_quote_prefixes(&view.text()[r.span.clone()], quotes)
        })
        .collect::<Vec<_>>()
        .join("\n");
    let mut queue = candidates(&parsed.tree, base, false);
    let mut result = vec![];
    let mut cursor = 0;
    while cursor < queue.len() {
        let item = queue[cursor].clone();
        cursor += 1;
        let raw = &view.text()[item.span.clone()];
        if item.image {
            let Some(end) = raw
                .strip_prefix('!')
                .and_then(|label| label_end(label, true))
            else {
                continue;
            };
            let label =
                crate::md045::mask_quote_prefixes(&raw[2..end + 1], item.quotes).replace('\n', " ");
            let input = format!("x {label}\n\n{definitions}");
            queue.extend(
                recover(&input, item.in_table)
                    .into_iter()
                    .filter(|inner| inner.span.start >= 2 && inner.span.end <= label.len() + 2)
                    .map(|mut inner| {
                        inner.span =
                            item.span.start + inner.span.start..item.span.start + inner.span.end;
                        inner.quotes = item.quotes;
                        inner.flow = false;
                        inner
                    }),
            );
        } else if item.flow {
            if let Some(comment) = raw.strip_prefix("<!--").and_then(|s| s.strip_suffix("-->"))
                && !comment.starts_with('>')
                && !comment.starts_with("->")
                && !comment.ends_with('-')
            {
                continue;
            }
            let first = view.position(item.span.start).unwrap().line;
            let last = view.position(item.span.end).unwrap().line;
            let whole = view.line_range(first).unwrap().start..view.line_range(last).unwrap().end;
            queue.extend(
                recover(&view.text()[whole.clone()], item.in_table)
                    .into_iter()
                    .map(|mut inner| {
                        inner.span = whole.start + inner.span.start..whole.start + inner.span.end;
                        inner.flow = false;
                        inner
                    }),
            );
        } else {
            result.push(HtmlToken {
                span: offsets[item.span.start]..offsets[item.span.end],
                in_table: item.in_table,
            });
        }
    }
    result.sort_by_key(|token| (token.span.start, token.span.end));
    result.dedup_by_key(|token| (token.span.start, token.span.end));
    result
}
