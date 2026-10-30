use crate::{
    Diagnostic, Source,
    config::{DocumentConfig, js_space},
    parse::{ParsedDocument, RegionKind, parse},
};
use markdown::mdast::Node;
use std::collections::HashSet;

struct Heading {
    start: usize,
    level: usize,
    text: String,
}

fn headings_in(
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
            for child in &heading.children {
                if !matches!(child, Node::Html(_))
                    && let Some(position) = child.position()
                {
                    text.push_str(
                        &source.text()[offset(base + position.start.offset)
                            ..offset(base + position.end.offset)],
                    );
                }
            }
            output.push(Heading {
                start: offset(base + position.start.offset),
                level: usize::from(heading.depth),
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

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    let headings = if source.text().contains('\t') {
        let (expanded, offsets) = crate::parse::expanded_tabs(source);
        match parse(&expanded) {
            Ok(parsed) => headings_in(source, &parsed, Some(&offsets)),
            Err(_) => vec![],
        }
    } else {
        headings_in(source, parsed, None)
    };
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
