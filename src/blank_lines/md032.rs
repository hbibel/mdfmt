use super::Insertion;
use crate::{
    Diagnostic, Source,
    config::{DocumentConfig, js_space},
    parse::{ParsedDocument, RegionKind},
};
use markdown::mdast::Node;
use std::ops::Range;

struct List {
    span: Range<usize>,
    visual_end: usize,
}
struct Finding {
    list: Range<usize>,
    line: usize,
    insertion: usize,
}

fn lists(source: &Source) -> Vec<List> {
    let Some((_expanded, parsed, offsets)) = crate::parse::diagnostic_lists(source) else {
        return vec![];
    };
    let base = parsed
        .regions
        .iter()
        .find(|r| r.kind == RegionKind::Root && r.parent.is_some())
        .map_or(0, |r| r.span.start);
    fn visual_end(node: &Node) -> Option<usize> {
        if matches!(
            node,
            Node::Root(_) | Node::List(_) | Node::ListItem(_) | Node::Blockquote(_)
        ) && let Some(children) = node.children()
            && let Some(end) = children.iter().rev().find_map(visual_end)
        {
            return Some(end);
        }
        node.position().map(|p| p.end.offset)
    }
    fn visit(node: &Node, base: usize, offsets: &[usize], out: &mut Vec<List>) {
        if matches!(node, Node::List(_))
            && let Some(p) = node.position()
        {
            out.push(List {
                span: offsets[base + p.start.offset]..offsets[base + p.end.offset],
                visual_end: offsets[base + visual_end(node).unwrap_or(p.end.offset)],
            });
            return;
        }
        if let Some(children) = node.children() {
            for child in children {
                visit(child, base, offsets, out);
            }
        }
    }
    let mut out = vec![];
    visit(&parsed.tree, base, &offsets, &mut out);
    out
}

fn findings(source: &Source, parsed: &ParsedDocument, document: &DocumentConfig) -> Vec<Finding> {
    if !(1..=source.line_count())
        .any(|line| document.state("MD032", line).is_some_and(|s| s.enabled))
    {
        return vec![];
    }
    let front = parsed
        .regions
        .iter()
        .find(|r| r.kind == RegionKind::FrontMatter)
        .map(|r| r.span.clone());
    let blank = |line: Option<usize>| {
        line.and_then(|line| source.line_range(line))
            .is_none_or(|span| {
                front
                    .as_ref()
                    .is_some_and(|f| f.start <= span.start && span.end <= f.end)
                    || super::is_blank(&source.text()[span])
            })
    };
    let mut result = vec![];
    for list in lists(source) {
        let first = source.position(list.span.start).unwrap().line;
        let last = crate::md013::covered_lines(source, list.span.start..list.visual_end).end;
        for (line, insertion, needed) in [
            (
                first,
                first,
                !blank(first.checked_sub(1).filter(|line| *line > 0)),
            ),
            (last, last + 1, !blank(Some(last + 1))),
        ] {
            if needed && document.state("MD032", line).is_some_and(|s| s.enabled) {
                result.push(Finding {
                    list: list.span.clone(),
                    line,
                    insertion,
                });
            }
        }
    }
    result
}

pub(super) fn insertions(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Insertion> {
    findings(source, parsed, document)
        .into_iter()
        .filter_map(|f| {
            Some(Insertion {
                at: source.line_range(f.insertion)?.start,
                prefix: super::prefix_for(source, parsed, RegionKind::List, &f.list)?,
                missing: 1,
                rule: "MD032",
            })
        })
        .collect()
}

pub(super) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    let display = crate::parse::comment_masked_source(source, parsed);
    findings(source, parsed, document)
        .into_iter()
        .map(|f| {
            let context = crate::heading_structure::context(
                display.text()[display.line_range(f.line).unwrap()].trim_matches(js_space),
            );
            let start = source.line_range(f.line).unwrap().start
                + usize::from(f.line == 1 && source.text().starts_with('\u{feff}')) * 3;
            Diagnostic {
                code: "MD032".into(),
                rule_id: Some("MD032".into()),
                severity: document.state("MD032", f.line).unwrap().severity,
                message: format!(
                    "Lists should be surrounded by blank lines [Context: \"{context}\"]"
                ),
                span: Some(start..start),
            }
        })
        .collect()
}
