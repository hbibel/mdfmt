use crate::{
    Diagnostic, Source,
    config::{DocumentConfig, js_space},
    parse::{ParsedDocument, RegionKind},
};
use markdown::mdast::Node;

pub(crate) fn punctuation_regex(punctuation: &str) -> Result<regress::Regex, String> {
    let mut escaped = false;
    for ch in punctuation.chars() {
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == ']' {
            return Err(
                "punctuation must remain inside a single JavaScript character class".into(),
            );
        }
    }
    if escaped {
        return Err("unterminated character-class escape".into());
    }
    regress::Regex::from_unicode(
        format!("[{punctuation}]$").encode_utf16().map(u32::from),
        regress::Flags::default(),
    )
    .map_err(|error| error.to_string())
}

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    let Some(state) = document.state("MD036", 1) else {
        return vec![];
    };
    let punctuation = state
        .options
        .get("punctuation")
        .and_then(|v| v.as_str())
        .unwrap_or(".,;:!?。，；：！？");
    let punctuation =
        punctuation_regex(punctuation).expect("validated punctuation character class");
    let recovered = crate::parse::diagnostic_parse(source);
    let parsed = recovered.as_deref().unwrap_or(parsed);
    let base = parsed
        .regions
        .iter()
        .find(|r| r.kind == RegionKind::Root && r.parent.is_some())
        .map_or(0, |r| r.span.start);
    let mut output = vec![];
    let mut paragraphs = vec![];
    for node in parsed.tree.children().into_iter().flatten() {
        if let Node::Html(html) = node
            && !html.value.starts_with("<!--")
        {
            let options = markdown::ParseOptions {
                constructs: markdown::Constructs {
                    html_flow: false,
                    ..markdown::Constructs::gfm()
                },
                ..markdown::ParseOptions::gfm()
            };
            let position = node.position().unwrap();
            let offset = base + position.start.offset;
            let raw = &source.text()[offset..base + position.end.offset];
            if let Ok(tree) = crate::parse::mdast(raw, &options) {
                paragraphs.extend(
                    tree.children()
                        .into_iter()
                        .flatten()
                        .cloned()
                        .map(|node| (node, offset)),
                );
            }
        } else {
            paragraphs.push((node.clone(), base));
        }
    }
    for (node, base) in &paragraphs {
        let Node::Paragraph(paragraph) = node else {
            continue;
        };
        let position = node.position().unwrap();
        if position.start.line != position.end.line {
            continue;
        }
        let meaningful:Vec<_>=paragraph.children.iter().filter(|child| {
            !matches!(child,Node::Html(_)) && !matches!(child,Node::Text(text) if text.value.trim_matches(js_space).is_empty())
        }).collect();
        let [emphasis] = meaningful.as_slice() else {
            continue;
        };
        if !matches!(emphasis, Node::Emphasis(_) | Node::Strong(_)) {
            continue;
        }
        let Some(children) = emphasis.children() else {
            continue;
        };
        let [Node::Text(text)] = children.as_slice() else {
            continue;
        };
        let text_position = children[0].position().unwrap();
        let raw =
            &source.text()[base + text_position.start.offset..base + text_position.end.offset];
        // Entities and escapes are distinct pinned inline tokens, not plain data.
        if !raw
            .split_ascii_whitespace()
            .eq(text.value.split_ascii_whitespace())
        {
            continue;
        }
        let line = source
            .position(base + text_position.start.offset)
            .unwrap()
            .line;
        let Some(state) = document.state("MD036", line).filter(|state| state.enabled) else {
            continue;
        };
        // The class consumes one UTF-16 unit; $ can also precede final CR/LF.
        let tail = raw
            .char_indices()
            .rev()
            .nth(2)
            .map_or(0, |(index, _)| index);
        let suffix = raw[tail..].encode_utf16().collect::<Vec<_>>();
        if punctuation.find_from_ucs2(&suffix, 0).next().is_some() {
            continue;
        }
        let start = source.line_range(line).unwrap().start
            + usize::from(line == 1 && source.text().starts_with('\u{feff}')) * 3;
        output.push(Diagnostic {
            code: "MD036".into(),
            rule_id: Some("MD036".into()),
            severity: state.severity,
            message: format!(
                "Emphasis used instead of a heading [Context: \"{}\"]",
                crate::heading_structure::context(raw)
            ),
            span: Some(start..start),
        });
    }
    output
}
