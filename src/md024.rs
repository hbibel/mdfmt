use crate::{
    Diagnostic, Source,
    config::{DocumentConfig, js_space},
    parse::{ParsedDocument, RegionKind, parse},
};
use markdown::mdast::Node;
use std::collections::HashSet;

pub(crate) struct Heading {
    pub start: usize,
    pub level: usize,
    pub text: String,
    pub docfx_tab: bool,
}

pub(crate) fn headings_in(
    source: &Source,
    parsed: &ParsedDocument,
    offsets: Option<&[usize]>,
) -> Vec<Heading> {
    let base = parsed
        .regions
        .iter()
        .find(|region| region.kind == RegionKind::Root && region.parent.is_some())
        .map_or(0, |region| region.span.start);
    fn visit(
        node: &Node,
        source: &Source,
        base: usize,
        offsets: Option<&[usize]>,
        output: &mut Vec<Heading>,
    ) {
        let offset = |index| offsets.map_or(index, |offsets| offsets[index]);
        if let Node::Heading(heading) = node
            && let Some(position) = node.position()
        {
            let mut text = String::new();
            let span = offset(base + position.start.offset)..offset(base + position.end.offset);
            let raw = &source.text()[span.clone()];
            let atx =
                !raw.contains(['\r', '\n']) && raw.trim_start_matches([' ', '\t']).starts_with('#');
            let lexical = atx.then(|| crate::parse::heading_content(source, span.clone(), false));
            fn html_tokens(node: &Node, output: &mut Vec<std::ops::Range<usize>>) {
                if matches!(node, Node::Html(_))
                    && let Some(position) = node.position()
                {
                    output.push(position.start.offset..position.end.offset);
                } else if matches!(node, Node::Delete(_))
                    && let Some(children) = node.children()
                {
                    for child in children {
                        html_tokens(child, output);
                    }
                }
            }
            let content = lexical.clone().or_else(|| {
                heading
                    .children
                    .first()
                    .and_then(Node::position)
                    .zip(heading.children.last().and_then(Node::position))
                    .map(|(first, last)| {
                        offset(base + first.start.offset)..offset(base + last.end.offset)
                    })
            });
            if let Some(content) = content {
                // mdast splits at hard breaks and groups strikethrough; pinned
                // heading tokens keep the intervening source and expose its HTML.
                let mut cursor = content.start;
                let end = content.end;
                let mut excluded = vec![];
                for child in &heading.children {
                    html_tokens(child, &mut excluded);
                }
                for span in excluded {
                    let start = offset(base + span.start);
                    let finish = offset(base + span.end);
                    if start < cursor || finish > end {
                        continue;
                    }
                    text.push_str(&source.text()[cursor..start]);
                    cursor = finish;
                }
                text.push_str(&source.text()[cursor..end]);
            }
            let docfx_tab = atx
                && heading.children.len() == 1
                && matches!(heading.children[0], Node::Link(_))
                && heading.children[0].position().is_some_and(|link| {
                    let link = offset(base + link.start.offset)..offset(base + link.end.offset);
                    if lexical.as_ref() != Some(&link) {
                        return false;
                    }
                    let raw = &source.text()[link];
                    crate::parse::label_end(raw, true)
                        .and_then(|end| raw[end + 1..].strip_prefix('('))
                        .is_some_and(|destination| {
                            destination
                                .trim_start_matches([' ', '\t', '\r', '\n'])
                                .strip_prefix('<')
                                .unwrap_or(destination.trim_start_matches([' ', '\t', '\r', '\n']))
                                .starts_with("#tab/")
                        })
                });
            output.push(Heading {
                start: offset(base + position.start.offset),
                level: usize::from(heading.depth),
                docfx_tab,
                text: text
                    .replace("\r\n", " ")
                    .replace(['\r', '\n'], " ")
                    .trim_matches(js_space)
                    .into(),
            });
        }
        if let Some(children) = node.children() {
            for child in children {
                visit(child, source, base, offsets, output);
            }
        }
    }
    let mut output = vec![];
    visit(&parsed.tree, source, base, offsets, &mut output);
    output
}

pub(crate) fn headings(source: &Source, parsed: &ParsedDocument) -> Vec<Heading> {
    if source.text().contains('\t') {
        let (expanded, offsets) = crate::parse::expanded_tabs(source);
        match parse(&expanded) {
            Ok(parsed) => headings_in(source, &parsed, Some(&offsets)),
            Err(_) => vec![],
        }
    } else {
        headings_in(source, parsed, None)
    }
}

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    let headings = headings(source, parsed);
    let mut global = HashSet::new();
    let mut levels: Vec<HashSet<String>> = (0..7).map(|_| HashSet::new()).collect();
    let mut last_level = 1;
    let mut output = vec![];
    for heading in headings {
        while last_level < heading.level {
            last_level += 1;
            levels[last_level].clear();
        }
        while last_level > heading.level {
            levels[last_level].clear();
            last_level -= 1;
        }
        let global_duplicate = !global.insert(heading.text.clone());
        let sibling_duplicate = !levels[heading.level].insert(heading.text.clone());
        let line = source.position(heading.start).unwrap().line;
        let Some(state) = document.state("MD024", line).filter(|state| state.enabled) else {
            continue;
        };
        let siblings_only = state
            .options
            .get("siblings_only")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        if !(if siblings_only {
            sibling_duplicate
        } else {
            global_duplicate
        }) {
            continue;
        }
        let context = if crate::utf16_len(&heading.text) > 30 {
            format!(
                "{}...",
                String::from_utf16_lossy(&heading.text.encode_utf16().take(30).collect::<Vec<_>>())
            )
        } else {
            heading.text
        };
        let start = source.line_range(line).unwrap().start
            + usize::from(line == 1 && source.text().starts_with('\u{feff}')) * 3;
        output.push(Diagnostic {
            code: "MD024".into(),
            rule_id: Some("MD024".into()),
            severity: state.severity,
            message: if context.is_empty() {
                "Multiple headings with the same content".into()
            } else {
                format!("Multiple headings with the same content [Context: \"{context}\"]")
            },
            span: Some(start..start),
        });
    }
    output
}
