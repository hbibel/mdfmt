use crate::{
    Diagnostic, Source,
    config::DocumentConfig,
    parse::{ParsedDocument, RegionKind, label_end},
};
use markdown::mdast::Node;
use std::collections::HashSet;

fn normalize(text: &str) -> String {
    let mut output = String::new();
    let mut space = false;
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            if space && !output.is_empty() {
                output.push(' ');
            }
            output.push(ch.to_ascii_lowercase());
            space = false;
        } else {
            space = true;
        }
    }
    output
}

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    let recovered = crate::parse::diagnostic_parse(source);
    let parsed = recovered.as_deref().unwrap_or(parsed);
    let definitions = parsed
        .regions
        .iter()
        .filter(|r| r.kind == RegionKind::Definition)
        .map(|r| {
            let mut quotes = 0;
            let mut parent = r.parent;
            while let Some(index) = parent {
                let ancestor = &parsed.regions[index];
                quotes += usize::from(ancestor.kind == RegionKind::Blockquote);
                parent = ancestor.parent;
            }
            crate::md045::mask_quote_prefixes(&source.text()[r.span.clone()], quotes)
        })
        .collect::<Vec<_>>()
        .join("\n");
    fn visit(
        node: &Node,
        source: &Source,
        base: usize,
        document: &DocumentConfig,
        definitions: &str,
        quotes: usize,
        output: &mut Vec<Diagnostic>,
    ) {
        let quotes = quotes + usize::from(matches!(node, Node::Blockquote(_)));
        if matches!(node, Node::Link(_) | Node::LinkReference(_))
            && let Some(position) = node.position()
        {
            let start = base + position.start.offset;
            let raw = &source.text()[start..base + position.end.offset];
            if let Some(end) = label_end(raw, true) {
                let children = node.children().unwrap();
                if !children
                    .iter()
                    .any(|child| matches!(child, Node::InlineCode(_) | Node::Html(_)))
                {
                    let label = &raw[1..end];
                    let line = source.position(start + 1).unwrap().line;
                    if let Some(state) = document.state("MD059", line).filter(|state| state.enabled)
                    {
                        let prohibited: HashSet<_> = if let Some(texts) = state
                            .options
                            .get("prohibited_texts")
                            .and_then(|v| v.as_array())
                        {
                            texts
                                .iter()
                                .filter_map(|v| v.as_str())
                                .map(normalize)
                                .collect()
                        } else {
                            ["click here", "here", "link", "more"]
                                .into_iter()
                                .map(str::to_owned)
                                .collect()
                        };
                        if prohibited.contains(&normalize(label)) {
                            let multiline = label.contains(['\r', '\n']);
                            let span = if multiline || label.is_empty() {
                                let start = source.line_range(line).unwrap().start
                                    + usize::from(
                                        line == 1 && source.text().starts_with('\u{feff}'),
                                    ) * 3;
                                start..start
                            } else {
                                start + 1..start + end
                            };
                            let context = crate::heading_structure::context(
                                &raw[..=end].replace("\r\n", " ").replace(['\r', '\n'], " "),
                            );
                            output.push(Diagnostic {
                                code: "MD059".into(),
                                rule_id: Some("MD059".into()),
                                severity: state.severity,
                                message: format!(
                                    "Link text should be descriptive [Context: \"{context}\"]"
                                ),
                                span: Some(span),
                            });
                        }
                    }
                }
            }
        }
        if matches!(node, Node::Image(_) | Node::ImageReference(_))
            && let Some(position) = node.position()
        {
            let start = base + position.start.offset;
            let raw = &source.text()[start..base + position.end.offset];
            if let Some(end) = raw
                .strip_prefix('!')
                .and_then(|label| label_end(label, true))
            {
                let label = crate::md045::mask_quote_prefixes(&raw[2..end + 1], quotes);
                let options = markdown::ParseOptions {
                    constructs: markdown::Constructs {
                        math_text: true,
                        html_flow: false,
                        ..markdown::Constructs::gfm()
                    },
                    ..markdown::ParseOptions::gfm()
                };
                if let Ok(tree) =
                    crate::parse::mdast(&format!("x {label}\n\n{definitions}"), &options)
                    && let Some(first) = tree.children().and_then(|children| children.first())
                {
                    visit(first, source, start, document, definitions, quotes, output);
                }
            }
        }
        if let Some(children) = node.children() {
            for child in children {
                visit(child, source, base, document, definitions, quotes, output);
            }
        }
    }
    let base = parsed
        .regions
        .iter()
        .find(|r| r.kind == RegionKind::Root && r.parent.is_some())
        .map_or(0, |r| r.span.start);
    let mut output = vec![];
    visit(
        &parsed.tree,
        source,
        base,
        document,
        &definitions,
        0,
        &mut output,
    );
    output
}
