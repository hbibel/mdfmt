use crate::{
    Diagnostic, Source,
    config::DocumentConfig,
    edit::{Edit, apply_authorized_edits},
    parse::{ParsedDocument, RegionKind, diagnostic_parse, parse},
};
use markdown::mdast::Node;
use std::ops::Range;

#[derive(Clone, PartialEq, Eq)]
struct Pair {
    opening: Range<usize>,
    closing: Range<usize>,
}

fn pairs(source: &Source, parsed: &ParsedDocument) -> Vec<Pair> {
    let base = parsed
        .regions
        .iter()
        .find(|r| r.kind == RegionKind::Root && r.parent.is_some())
        .map_or(0, |r| r.span.start);
    fn visit(node: &Node, source: &Source, base: usize, quotes: usize, output: &mut Vec<Pair>) {
        let quotes = quotes + usize::from(matches!(node, Node::Blockquote(_)));
        if matches!(node, Node::Emphasis(_))
            && let Some(position) = node.position()
        {
            let start = base + position.start.offset;
            let end = base + position.end.offset;
            if end >= start + 2
                && matches!(source.text().as_bytes().get(start), Some(b'*' | b'_'))
                && source.text().as_bytes()[start] == source.text().as_bytes()[end - 1]
            {
                let pair = Pair {
                    opening: start..start + 1,
                    closing: end - 1..end,
                };
                if source.text().as_bytes()[start] != b'_' || !astral_neighbors(source, &pair) {
                    output.push(pair);
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
                // Image alt text is flattened by mdast, but upstream retains
                // inline emphasis tokens. The two-byte prefix preserves offsets.
                if let Ok(tree) = crate::parse::mdast(&format!("x {label}"), &options)
                    && let Some(first) = tree.children().and_then(|children| children.first())
                {
                    visit(first, source, start, 0, output);
                }
            }
        }
        if let Some(children) = node.children() {
            for child in children {
                visit(child, source, base, quotes, output);
            }
        }
    }
    let mut output = vec![];
    visit(&parsed.tree, source, base, 0, &mut output);
    output
}

fn style(marker: u8) -> &'static str {
    if marker == b'*' {
        "asterisk"
    } else {
        "underscore"
    }
}

fn target(document: &DocumentConfig, source: &Source, at: usize, anchor: u8) -> Option<u8> {
    let state = document.state("MD049", source.position(at).ok()?.line)?;
    if !state.enabled {
        return None;
    }
    Some(
        match state
            .options
            .get("style")
            .and_then(|v| v.as_str())
            .unwrap_or("consistent")
        {
            "asterisk" => b'*',
            "underscore" => b'_',
            _ => anchor,
        },
    )
}

fn intraword(source: &Source, pair: &Pair) -> bool {
    let word = |ch: char| ch.is_ascii_alphanumeric() || ch == '_';
    source.text()[..pair.opening.start]
        .chars()
        .next_back()
        .is_some_and(word)
        || source.text()[pair.closing.end..]
            .chars()
            .next()
            .is_some_and(word)
}

fn astral_neighbors(source: &Source, pair: &Pair) -> bool {
    // Pinned micromark classifies adjacent UTF-16 code units, not Unicode scalars.
    // An emoji's surrogate therefore acts like a word character for underscores.
    source.text()[..pair.opening.start]
        .chars()
        .next_back()
        .is_some_and(|ch| ch.len_utf16() == 2)
        || source.text()[pair.closing.end..]
            .chars()
            .next()
            .is_some_and(|ch| ch.len_utf16() == 2)
}

fn safe(source: &Source, parsed: &ParsedDocument, edits: &[Edit]) -> bool {
    let Ok(output) = apply_authorized_edits(source, parsed, edits.to_vec(), edits) else {
        return false;
    };
    let candidate = Source::new(output);
    let Ok(reparsed) = parse(&candidate) else {
        return false;
    };
    fn normalize(node: &mut Node) {
        node.position_set(None);
        if let Some(children) = node.children_mut() {
            for child in children {
                normalize(child);
            }
        }
    }
    let mut before = parsed.tree.clone();
    let mut after = reparsed.tree.clone();
    normalize(&mut before);
    normalize(&mut after);
    before == after
}

pub(crate) fn propose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Edit> {
    if !(1..=source.line_count())
        .any(|line| document.state("MD049", line).is_some_and(|s| s.enabled))
        || crate::parse::has_unmodeled_syntax(source, parsed)
    {
        return vec![];
    }
    let diagnostic = diagnostic_parse(source);
    let selected = pairs(source, diagnostic.as_ref().unwrap_or(parsed));
    let Some(first) = selected.first() else {
        return vec![];
    };
    let anchor = source.text().as_bytes()[first.opening.start];
    let native = pairs(source, parsed);
    let mut groups = vec![];
    for pair in selected {
        let actual = source.text().as_bytes()[pair.opening.start];
        let Some(expected) = target(document, source, pair.opening.start, anchor) else {
            continue;
        };
        if expected == actual
            || target(document, source, pair.closing.start, anchor) != Some(expected)
            || (expected == b'_' && (intraword(source, &pair) || astral_neighbors(source, &pair)))
            || !native.contains(&pair)
            || [pair.opening.clone(), pair.closing.clone()]
                .iter()
                .any(|span| {
                    !document.allows_source("MD049", span.clone(), source)
                        || parsed.regions.iter().any(|region| {
                            matches!(
                                region.kind,
                                RegionKind::Link
                                    | RegionKind::Image
                                    | RegionKind::InlineCode
                                    | RegionKind::Table
                                    | RegionKind::Code
                                    | RegionKind::Html
                                    | RegionKind::Definition
                                    | RegionKind::FrontMatter
                            ) && span.start < region.span.end
                                && region.span.start < span.end
                        })
                })
        {
            continue;
        }
        let edits: Vec<_> = [pair.opening, pair.closing]
            .into_iter()
            .map(|span| Edit {
                span,
                replacement: char::from(expected).to_string(),
                origin: "MD049".into(),
            })
            .collect();
        groups.push(edits);
    }
    let combined: Vec<_> = groups.iter().flatten().cloned().collect();
    if safe(source, parsed, &combined) {
        return combined;
    }
    if groups.len() > 16 {
        return vec![];
    }
    let output: Vec<_> = groups
        .into_iter()
        .filter(|edits| safe(source, parsed, edits))
        .flatten()
        .collect();
    if safe(source, parsed, &output) {
        output
    } else {
        vec![]
    }
}

pub(crate) fn diagnose(
    source: &Source,
    _parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    if !(1..=source.line_count())
        .any(|line| document.state("MD049", line).is_some_and(|s| s.enabled))
    {
        return vec![];
    }
    let Some(parsed) = diagnostic_parse(source) else {
        return vec![];
    };
    let selected = pairs(source, &parsed);
    let Some(first) = selected.first() else {
        return vec![];
    };
    let anchor = source.text().as_bytes()[first.opening.start];
    let mut output = vec![];
    for pair in selected {
        let actual = source.text().as_bytes()[pair.opening.start];
        let intraword = intraword(source, &pair);
        for span in [pair.opening, pair.closing] {
            let Some(expected) = target(document, source, span.start, anchor) else {
                continue;
            };
            if expected == actual || (expected == b'_' && intraword) {
                continue;
            }
            output.push(Diagnostic {
                code: "MD049".into(),
                rule_id: Some("MD049".into()),
                severity: document
                    .state("MD049", source.position(span.start).unwrap().line)
                    .unwrap()
                    .severity,
                message: format!(
                    "Emphasis style [Expected: {}; Actual: {}]",
                    style(expected),
                    style(actual)
                ),
                span: Some(span),
            });
        }
    }
    output
}
