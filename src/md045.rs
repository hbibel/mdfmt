use crate::{
    Diagnostic, Source,
    config::{DocumentConfig, js_space},
    parse::{ParsedDocument, RegionKind, label_end, parse},
};
use markdown::mdast::Node;
use std::ops::Range;

#[derive(Clone)]
struct Candidate {
    kind: RegionKind,
    span: Range<usize>,
    flow: bool,
    html_only: bool,
    quotes: usize,
}

fn selected(source: &Source, parsed: &ParsedDocument) -> Vec<Candidate> {
    let select = |parsed: &ParsedDocument| {
        parsed
            .regions
            .iter()
            .filter(|region| {
                matches!(
                    region.kind,
                    RegionKind::Image | RegionKind::Html | RegionKind::Definition
                )
            })
            .map(|region| {
                let mut quotes = 0;
                let mut parent = region.parent;
                while let Some(index) = parent {
                    let ancestor = &parsed.regions[index];
                    quotes += usize::from(ancestor.kind == RegionKind::Blockquote);
                    parent = ancestor.parent;
                }
                Candidate {
                    kind: region.kind,
                    span: region.span.clone(),
                    quotes,
                    html_only: false,
                    flow: region.parent.is_some_and(|parent| {
                        matches!(
                            parsed.regions[parent].kind,
                            RegionKind::Root | RegionKind::Blockquote | RegionKind::ListItem
                        )
                    }),
                }
            })
            .collect::<Vec<_>>()
    };
    if source.text().contains('\t') {
        let (expanded, offsets) = crate::parse::expanded_tabs(source);
        let Ok(parsed) = parse(&expanded) else {
            return vec![];
        };
        return select(&parsed)
            .into_iter()
            .map(|mut item| {
                item.span = offsets[item.span.start]..offsets[item.span.end];
                item
            })
            .collect();
    }
    select(parsed)
}

fn attribute<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    for (index, ch) in text.char_indices().filter(|(_, ch)| js_space(*ch)) {
        let rest = &text[index + ch.len_utf8()..];
        if !rest
            .get(..name.len())
            .is_some_and(|value| value.eq_ignore_ascii_case(name))
        {
            continue;
        }
        let Some(rest) = rest[name.len()..]
            .trim_start_matches(js_space)
            .strip_prefix('=')
        else {
            continue;
        };
        let rest = rest.trim_start_matches(js_space);
        let rest = rest.strip_prefix(['\'', '"']).unwrap_or(rest);
        let end = rest
            .char_indices()
            .find(|(_, ch)| matches!(ch, '\'' | '"' | '>') || js_space(*ch))
            .map_or(rest.len(), |(index, _)| index);
        return Some(&rest[..end]);
    }
    None
}

fn mask_quote_prefixes(text: &str, quotes: usize) -> String {
    let mut bytes = text.as_bytes().to_vec();
    let mut cursor = 0;
    while cursor < bytes.len() {
        if matches!(bytes[cursor], b'\r' | b'\n') {
            let mut start = cursor + 1;
            for _ in 0..quotes {
                while matches!(bytes.get(start), Some(b' ' | b'\t')) {
                    start += 1;
                }
                if bytes.get(start) != Some(&b'>') {
                    break;
                }
                bytes[start] = b' ';
                start += 1;
            }
        }
        cursor += 1;
    }
    String::from_utf8(bytes).unwrap()
}

fn inline_candidates(text: &str) -> Vec<Candidate> {
    let source = Source::new(text.into());
    let (expanded, offsets) = crate::parse::expanded_tabs(&source);
    // Pinned non-comment HTML flow is reparsed as Markdown, not as a DOM.
    let options = markdown::ParseOptions {
        constructs: markdown::Constructs {
            html_flow: false,
            code_indented: false,
            ..markdown::Constructs::gfm()
        },
        ..markdown::ParseOptions::gfm()
    };
    let Ok(tree) = markdown::to_mdast(expanded.text(), &options) else {
        return vec![];
    };
    fn visit(node: &Node, quotes: usize, offsets: &[usize], output: &mut Vec<Candidate>) {
        let quotes = quotes + usize::from(matches!(node, Node::Blockquote(_)));
        let kind = match node {
            Node::Html(_) => Some(RegionKind::Html),
            Node::Image(_) | Node::ImageReference(_) => Some(RegionKind::Image),
            _ => None,
        };
        if let Some(kind) = kind
            && let Some(position) = node.position()
        {
            output.push(Candidate {
                kind,
                span: offsets[position.start.offset]..offsets[position.end.offset],
                flow: false,
                html_only: false,
                quotes,
            });
        }
        if let Some(children) = node.children() {
            for child in children {
                visit(child, quotes, offsets, output);
            }
        }
    }
    let mut output = vec![];
    visit(&tree, 0, &offsets, &mut output);
    output
}

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    let mut queue = selected(source, parsed);
    let definitions = queue
        .iter()
        .filter(|item| item.kind == RegionKind::Definition)
        .map(|item| mask_quote_prefixes(&source.text()[item.span.clone()], item.quotes))
        .collect::<Vec<_>>()
        .join("\n");
    let mut result = vec![];
    let mut cursor = 0;
    while cursor < queue.len() {
        let item = queue[cursor].clone();
        cursor += 1;
        let span = item.span.clone();
        let raw = &source.text()[span.clone()];
        let finding = if item.kind == RegionKind::Image {
            let Some(end) = raw
                .strip_prefix('!')
                .and_then(|label| label_end(label, true))
            else {
                continue;
            };
            let label = &raw[2..end + 1];
            if !label.is_empty() {
                // Mask only container markers, preserving byte offsets in the
                // original flattened description for nested image/HTML findings.
                let logical = mask_quote_prefixes(label, item.quotes);
                let candidate = format!("x {logical}\n\n{definitions}");
                queue.extend(
                    inline_candidates(&candidate)
                        .into_iter()
                        .filter(|inner| 2 <= inner.span.start && inner.span.end <= 2 + label.len())
                        .map(|mut inner| {
                            inner.span = span.start + inner.span.start..span.start + inner.span.end;
                            inner.html_only = item.html_only;
                            inner.quotes = item.quotes;
                            inner
                        }),
                );
                continue;
            }
            if item.html_only {
                continue;
            }
            if raw.contains(['\r', '\n']) {
                let line = source.position(span.start).unwrap().line;
                let start = source.line_range(line).unwrap().start
                    + usize::from(line == 1 && source.text().starts_with('\u{feff}')) * 3;
                start..start
            } else {
                span
            }
        } else if item.kind == RegionKind::Html {
            if item.flow && !raw.starts_with("<!--") {
                let first = source.position(span.start).unwrap().line;
                let last = crate::md013::covered_lines(source, span.clone()).end;
                let whole =
                    source.line_range(first).unwrap().start..source.line_range(last).unwrap().end;
                queue.extend(
                    inline_candidates(&source.text()[whole.clone()])
                        .into_iter()
                        .map(|mut inner| {
                            inner.span =
                                whole.start + inner.span.start..whole.start + inner.span.end;
                            inner.html_only = true;
                            inner
                        }),
                );
                continue;
            }
            let Some(tag) = raw.strip_prefix('<') else {
                continue;
            };
            let name = tag
                .split(|ch| matches!(ch, '/' | '>') || js_space(ch))
                .next()
                .unwrap_or("");
            if !name.eq_ignore_ascii_case("img")
                || attribute(raw, "alt").is_some()
                || attribute(raw, "aria-hidden")
                    .is_some_and(|value| value.eq_ignore_ascii_case("true"))
            {
                continue;
            }
            let line = source.position(span.start).unwrap().line;
            span.start..span.end.min(source.line_range(line).unwrap().end)
        } else {
            continue;
        };
        let line = source.position(finding.start).unwrap().line;
        let Some(state) = document.state("MD045", line).filter(|state| state.enabled) else {
            continue;
        };
        result.push(Diagnostic {
            code: "MD045".into(),
            rule_id: Some("MD045".into()),
            severity: state.severity,
            message: "Images should have alternate text (alt text)".into(),
            span: Some(finding),
        });
    }
    result
}
