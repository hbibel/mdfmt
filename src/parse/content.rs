use super::{ParsedDocument, RegionKind, diagnostic_parse};
use crate::{Source, config::js_space};
use markdown::mdast::Node;
use std::{collections::HashMap, ops::Range};

pub(crate) struct ContentFields {
    pub bare_urls: bool,
}

#[derive(Default)]
pub(crate) struct ContentAnalysis {
    pub bare_urls: Vec<Range<usize>>,
}

fn tag(node: &Node) -> Option<(&str, bool)> {
    let Node::Html(html) = node else {
        return None;
    };
    let raw = html.value.strip_prefix('<')?;
    if raw.starts_with(['!', '>']) {
        return None;
    }
    let close = raw.starts_with('/');
    let raw = raw.strip_prefix('/').unwrap_or(raw);
    let end = raw
        .find(|ch| matches!(ch, '/' | '>') || js_space(ch))
        .unwrap_or(raw.len());
    (!raw[..end].is_empty()).then_some((&raw[..end], close))
}

fn visit(source: &Source, node: &Node, base: usize, output: &mut ContentAnalysis) {
    match node {
        Node::Link(_) => {
            if let Some(position) = node.position() {
                let span = base + position.start.offset..base + position.end.offset;
                if !source.text()[span.clone()].starts_with(['[', '<']) {
                    output.bare_urls.push(span);
                }
            }
            return;
        }
        Node::Image(_)
        | Node::ImageReference(_)
        | Node::LinkReference(_)
        | Node::InlineCode(_)
        | Node::Code(_)
        | Node::InlineMath(_)
        | Node::Math(_)
        | Node::Html(_)
        | Node::Definition(_) => return,
        _ => {}
    }
    let Some(children) = node.children() else {
        return;
    };
    let mut opens: HashMap<&str, Vec<usize>> = HashMap::new();
    let mut pairs = HashMap::new();
    for (index, child) in children.iter().enumerate() {
        if let Some((name, close)) = tag(child) {
            let stack = opens.entry(name).or_default();
            if close {
                if let Some(open) = stack.pop() {
                    pairs.insert(open, index);
                }
            } else {
                stack.push(index);
            }
        }
    }
    let mut index = 0;
    while index < children.len() {
        if tag(&children[index]).is_some_and(|(_, close)| !close) {
            index = pairs.get(&index).map_or(index + 1, |end| end + 1);
            continue;
        }
        let previous_data = index > 0
            && matches!(&children[index - 1], Node::Text(_))
            && children[index - 1].position().is_some_and(|position| {
                let raw = &source.text()[base + position.start.offset..base + position.end.offset];
                raw.ends_with('<')
                    && source.text()[..base + position.end.offset - 1]
                        .bytes()
                        .rev()
                        .take_while(|byte| *byte == b'\\')
                        .count()
                        % 2
                        == 0
            });
        let next_data = index + 1 < children.len()
            && matches!(&children[index + 1], Node::Text(_))
            && children[index + 1].position().is_some_and(|position| {
                source.text()[base + position.start.offset..base + position.end.offset]
                    .starts_with('>')
            });
        let surrounded = matches!(&children[index], Node::Link(_)) && previous_data && next_data;
        if !surrounded {
            visit(source, &children[index], base, output);
        }
        index += 1;
    }
}

pub(crate) fn content_analysis(
    source: &Source,
    parsed: &ParsedDocument,
    fields: ContentFields,
) -> ContentAnalysis {
    let mut output = ContentAnalysis::default();
    if !fields.bare_urls {
        return output;
    }
    let (view, offsets) = super::html::diagnostic_view(source, parsed);
    let Some(parsed) = diagnostic_parse(&view) else {
        return output;
    };
    let base = parsed
        .regions
        .iter()
        .find(|region| region.kind == RegionKind::Root && region.parent.is_some())
        .map_or(0, |region| region.span.start);
    visit(&view, &parsed.tree, base, &mut output);
    for span in &mut output.bare_urls {
        *span = offsets[span.start]..offsets[span.end];
    }
    output.bare_urls.sort_by_key(|span| span.start);
    output
}
