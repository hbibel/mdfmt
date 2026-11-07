use crate::{
    Diagnostic, Severity, Source,
    config::DocumentConfig,
    edit::Edit,
    parse::{ParsedDocument, RegionKind, parse},
};
use markdown::mdast::Node;
use std::ops::Range;

struct Heading {
    span: Range<usize>,
    content: Range<usize>,
    level: usize,
    style: &'static str,
    native_content_matches: bool,
}

fn headings(source: &Source, parsed: &ParsedDocument) -> Vec<Heading> {
    fn visit(node: &Node, source: &Source, base: usize, output: &mut Vec<Heading>) {
        if let Node::Heading(heading) = node
            && let Some(position) = node.position()
        {
            let span = base + position.start.offset..base + position.end.offset;
            let raw = &source.text()[span.clone()];
            let trimmed = raw.trim_start_matches([' ', '\t']);
            let hashes = trimmed.bytes().take_while(|byte| *byte == b'#').count();
            let style = if raw.contains(['\r', '\n']) {
                "setext"
            } else {
                let text = trimmed[hashes..].trim_matches([' ', '\t']);
                let without_hashes = text.trim_end_matches('#');
                if text.len() != without_hashes.len()
                    && (without_hashes.is_empty() || without_hashes.ends_with([' ', '\t']))
                {
                    "atx_closed"
                } else {
                    "atx"
                }
            };
            let native_content = match (
                heading.children.first().and_then(Node::position),
                heading.children.last().and_then(Node::position),
            ) {
                (Some(first), Some(last)) => base + first.start.offset..base + last.end.offset,
                _ => {
                    span.start + raw.len() - trimmed.len() + hashes
                        ..span.start + raw.len() - trimmed.len() + hashes
                }
            };
            let content = crate::parse::heading_content(source, span.clone(), style == "setext");
            // Native child ranges can omit literal hashes; disagreement cannot authorize edits.
            let native_content_matches =
                content == native_content || (content.is_empty() && native_content.is_empty());
            output.push(Heading {
                span,
                content,
                level: usize::from(heading.depth),
                style,
                native_content_matches,
            });
        }
        if let Some(children) = node.children() {
            for child in children {
                visit(child, source, base, output);
            }
        }
    }
    let base = parsed
        .regions
        .iter()
        .find(|r| r.kind == RegionKind::Root && r.parent.is_some())
        .map_or(0, |r| r.span.start);
    let mut output = vec![];
    visit(&parsed.tree, source, base, &mut output);
    output
}

pub(crate) fn source_contents(source: &Source, parsed: &ParsedDocument) -> Vec<(usize, String)> {
    headings(source, parsed)
        .into_iter()
        .map(|heading| (heading.level, source.text()[heading.content].to_owned()))
        .collect()
}

fn findings(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<(Heading, String, Severity)> {
    let mut consistent = None;
    let mut output = vec![];
    for heading in headings(source, parsed) {
        let line = source.position(heading.span.start).unwrap().line;
        let Some(state) = document.state("MD003", line) else {
            continue;
        };
        let configured = state
            .options
            .get("style")
            .and_then(|v| v.as_str())
            .unwrap_or("consistent");
        let expected = match configured {
            "consistent" => *consistent.get_or_insert(heading.style),
            "setext_with_atx" => {
                if heading.level <= 2 {
                    "setext"
                } else {
                    "atx"
                }
            }
            "setext_with_atx_closed" => {
                if heading.level <= 2 {
                    "setext"
                } else {
                    "atx_closed"
                }
            }
            style => style,
        };
        if state.enabled && heading.style != expected {
            output.push((heading, expected.into(), state.severity));
        }
    }
    output
}

fn safe(source: &Source, parsed: &ParsedDocument, edits: &[Edit]) -> bool {
    crate::edit::apply_authorized_edits(source, parsed, edits.to_vec(), edits)
        .ok()
        .and_then(|output| {
            let candidate = Source::new(output);
            let reparsed = parse(&candidate).ok()?;
            Some(crate::parse::equivalent_with_heading_styles(
                source, parsed, &candidate, &reparsed,
            ))
        })
        .unwrap_or(false)
}

pub(crate) fn propose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Edit> {
    if crate::parse::has_unmodeled_syntax(source, parsed) {
        return vec![];
    }
    let mut edits = vec![];
    for (heading, target, _) in findings(source, parsed, document) {
        let content = &source.text()[heading.content.clone()];
        if !heading.native_content_matches
            || content.contains(['\r', '\n'])
            || !document.allows_source("MD003", heading.span.clone(), source)
        {
            continue;
        }
        let first = source.position(heading.span.start).unwrap().line;
        let last = source.position(heading.span.end).unwrap().line;
        let raw = &source.text()[heading.span.clone()];
        let indent = &raw[..raw.len() - raw.trim_start_matches([' ', '\t']).len()];
        let first_range = source.line_range(first).unwrap();
        let start = first_range.start
            + usize::from(first == 1 && source.text().starts_with('\u{feff}')) * 3;
        let prefix = &source.text()[start..heading.span.start + indent.len()];
        if prefix.contains('\t') {
            continue;
        }
        let replacement = if target == "setext" {
            if heading.level > 2
                || content.is_empty()
                || !prefix.chars().all(|ch| matches!(ch, ' ' | '>'))
            {
                continue;
            }
            let newline = if first_range.end < source.text().len()
                && source.text()[first_range.end..].starts_with("\r\n")
            {
                "\r\n"
            } else if source.text().as_bytes().get(first_range.end) == Some(&b'\r') {
                "\r"
            } else {
                crate::md047::preferred_newline(source.text())
            };
            format!(
                "{indent}{content}{newline}{prefix}{}",
                if heading.level == 1 { "===" } else { "---" }
            )
        } else {
            let hashes = "#".repeat(heading.level);
            if target == "atx_closed" && content.is_empty() {
                format!("{indent}{hashes} {hashes}")
            } else if target == "atx_closed" {
                format!("{indent}{hashes} {content} {hashes}")
            } else if content.is_empty() {
                format!("{indent}{hashes}")
            } else {
                format!("{indent}{hashes} {content}")
            }
        };
        let lines_changed = source.text()[heading.span.clone()]
            .bytes()
            .filter(|byte| *byte == b'\n' || *byte == b'\r')
            .count()
            != replacement
                .bytes()
                .filter(|byte| *byte == b'\n' || *byte == b'\r')
                .count();
        if lines_changed && (first..=last).any(|line| document.has_any_line_override(line)) {
            continue;
        }
        let edit = Edit {
            span: heading.span,
            replacement,
            origin: "MD003".into(),
        };
        edits.push(edit);
    }
    if edits.is_empty() || safe(source, parsed, &edits) {
        return edits;
    }
    let edits = edits
        .into_iter()
        .filter(|edit| safe(source, parsed, std::slice::from_ref(edit)))
        .collect::<Vec<_>>();
    if edits.is_empty() || safe(source, parsed, &edits) {
        edits
    } else {
        vec![]
    }
}

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    let recovered = crate::parse::diagnostic_parse(source);
    let parsed = recovered.as_ref().unwrap_or(parsed);
    findings(source, parsed, document)
        .into_iter()
        .map(|(heading, expected, severity)| {
            let line = source.position(heading.span.start).unwrap().line;
            let start = source.line_range(line).unwrap().start
                + usize::from(line == 1 && source.text().starts_with('\u{feff}')) * 3;
            Diagnostic {
                code: "MD003".into(),
                rule_id: Some("MD003".into()),
                severity,
                message: format!(
                    "Heading style [Expected: {expected}; Actual: {}]",
                    heading.style
                ),
                span: Some(start..start),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_style_equivalence_cannot_hide_literal_hash_deletions() {
        for (before, after, expected) in [
            ("# # Title\n", "# Title #\n", false),
            ("# Title # #\n", "# Title\n", false),
            ("Title #\n===\n", "# Title #\n", false),
            ("# Title\n", "# Title #\n", true),
        ] {
            let source = Source::new(before.into());
            let candidate = Source::new(after.into());
            assert_eq!(
                crate::parse::equivalent_with_heading_styles(
                    &source,
                    &parse(&source).unwrap(),
                    &candidate,
                    &parse(&candidate).unwrap()
                ),
                expected,
                "{before:?} -> {after:?}"
            );
        }
    }
}
