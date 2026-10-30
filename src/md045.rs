use crate::{
    Diagnostic, Source,
    config::{DocumentConfig, js_space},
    parse::{ParsedDocument, RegionKind, label_end, parse},
};
use markdown::mdast::Node;
use std::ops::Range;

fn selected(source: &Source, parsed: &ParsedDocument) -> Vec<(RegionKind, Range<usize>, bool)> {
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
                (
                    region.kind,
                    region.span.clone(),
                    region.parent.is_some_and(|parent| {
                        matches!(
                            parsed.regions[parent].kind,
                            RegionKind::Root | RegionKind::Blockquote | RegionKind::ListItem
                        )
                    }),
                )
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
            .map(|(kind, span, flow)| (kind, offsets[span.start]..offsets[span.end], flow))
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

fn html_text_spans(text: &str) -> Vec<Range<usize>> {
    // Pinned HTML-flow diagnostics reparse non-comment blocks as Markdown,
    // including apparent tags inside script/style strings, not as a DOM.
    let options = markdown::ParseOptions {
        constructs: markdown::Constructs {
            html_flow: false,
            code_indented: false,
            ..markdown::Constructs::gfm()
        },
        ..markdown::ParseOptions::gfm()
    };
    let Ok(tree) = markdown::to_mdast(text, &options) else {
        return vec![];
    };
    fn visit(node: &Node, spans: &mut Vec<Range<usize>>) {
        if matches!(node, Node::Html(_))
            && let Some(position) = node.position()
        {
            spans.push(position.start.offset..position.end.offset);
        }
        if let Some(children) = node.children() {
            for child in children {
                visit(child, spans);
            }
        }
    }
    let mut spans = vec![];
    visit(&tree, &mut spans);
    spans
}

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    let mut queue = selected(source, parsed);
    let definitions = queue
        .iter()
        .filter(|(kind, _, _)| *kind == RegionKind::Definition)
        .map(|(_, span, _)| &source.text()[span.clone()])
        .collect::<Vec<_>>()
        .join("\n");
    let mut result = vec![];
    let mut cursor = 0;
    while cursor < queue.len() {
        let (kind, span, flow) = queue[cursor].clone();
        cursor += 1;
        let raw = &source.text()[span.clone()];
        let finding = if kind == RegionKind::Image {
            let Some(end) = raw
                .strip_prefix('!')
                .and_then(|label| label_end(label, true))
            else {
                continue;
            };
            let label = &raw[2..end + 1];
            if !label.is_empty() {
                let candidate = Source::new(format!("x {label}\n\n{definitions}"));
                if let Ok(parsed) = parse(&candidate) {
                    queue.extend(
                        selected(&candidate, &parsed)
                            .into_iter()
                            .filter(|(kind, inner, _)| {
                                matches!(kind, RegionKind::Image | RegionKind::Html)
                                    && 2 <= inner.start
                                    && inner.end <= 2 + label.len()
                            })
                            .map(|(kind, inner, flow)| {
                                (kind, span.start + inner.start..span.start + inner.end, flow)
                            }),
                    );
                }
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
        } else if kind == RegionKind::Html {
            if flow && !raw.starts_with("<!--") {
                queue.extend(html_text_spans(raw).into_iter().map(|inner| {
                    (
                        RegionKind::Html,
                        span.start + inner.start..span.start + inner.end,
                        false,
                    )
                }));
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
