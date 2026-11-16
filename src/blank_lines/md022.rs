use super::Insertion;
use crate::{
    Diagnostic, Source,
    config::{DocumentConfig, js_space},
    parse::{ParsedDocument, RegionKind},
};
use markdown::mdast::Node;
use std::ops::Range;

struct Heading {
    span: Range<usize>,
    level: usize,
}
struct Finding {
    heading: Heading,
    line: usize,
    side: &'static str,
    expected: usize,
    actual: usize,
    insertion: usize,
}

fn requirement(
    options: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    level: usize,
) -> Option<usize> {
    let value = options.get(key);
    let value = value.and_then(|v| {
        if let Some(values) = v.as_array() {
            values.get(level - 1)
        } else {
            Some(v)
        }
    });
    if value.and_then(|v| v.as_i64()).is_some_and(|n| n < 0) {
        return None;
    }
    Some(
        value
            .and_then(|v| v.as_u64())
            .map(|n| usize::try_from(n).unwrap_or(usize::MAX))
            .unwrap_or(1),
    )
}

fn findings(source: &Source, parsed: &ParsedDocument, document: &DocumentConfig) -> Vec<Finding> {
    if !(1..=source.line_count())
        .any(|line| document.state("MD022", line).is_some_and(|s| s.enabled))
    {
        return vec![];
    }
    let recovered = crate::parse::diagnostic_parse(source);
    let parsed = recovered.as_deref().unwrap_or(parsed);
    let base = parsed
        .regions
        .iter()
        .find(|r| r.kind == RegionKind::Root && r.parent.is_some())
        .map_or(0, |r| r.span.start);
    let mut headings = vec![];
    fn visit(node: &Node, base: usize, output: &mut Vec<Heading>) {
        if let Node::Heading(h) = node
            && let Some(position) = &h.position
        {
            output.push(Heading {
                span: base + position.start.offset..base + position.end.offset,
                level: h.depth.into(),
            });
        }
        if let Some(children) = node.children() {
            for child in children {
                visit(child, base, output);
            }
        }
    }
    visit(&parsed.tree, base, &mut headings);
    // Container recovery can promote a one-dash setext underline to an empty
    // list. Keep its physical heading boundary without granting native ownership.
    let mut siblings = std::collections::HashMap::new();
    for (index, region) in parsed.regions.iter().enumerate() {
        let previous = region
            .parent
            .and_then(|parent| siblings.insert(parent, index));
        let first = source.position(region.span.start).unwrap().line;
        let end = source.line_range(first).unwrap().end;
        if region.kind != RegionKind::List
            || source.text()[region.span.start..end].trim_matches([' ', '\t']) != "-"
        {
            continue;
        }
        let Some(paragraph) = previous
            .map(|index| &parsed.regions[index])
            .filter(|r| r.kind == RegionKind::Paragraph)
        else {
            continue;
        };
        if crate::md013::covered_lines(source, paragraph.span.clone()).end + 1 == first {
            headings.push(Heading {
                span: paragraph.span.start..end,
                level: 2,
            });
        }
    }
    headings.sort_by_key(|heading| heading.span.start);
    let front = parsed
        .regions
        .iter()
        .find(|r| r.kind == RegionKind::FrontMatter)
        .map(|r| r.span.clone());
    let mut output = vec![];
    for heading in headings {
        let first = source.position(heading.span.start).unwrap().line;
        let last = crate::md013::covered_lines(source, heading.span.clone()).end;
        let Some(state) = document.state("MD022", first).filter(|s| s.enabled) else {
            continue;
        };
        let include = state
            .options
            .get("include_front_matter")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        for (side, key, insertion) in [
            ("Above", "lines_above", first),
            ("Below", "lines_below", last + 1),
        ] {
            let Some(expected) = requirement(&state.options, key, heading.level) else {
                continue;
            };
            let mut actual = 0;
            for i in 0..expected {
                let neighbor = if side == "Above" {
                    first.checked_sub(i + 1).filter(|line| *line > 0)
                } else {
                    last.checked_add(i + 1)
                };
                let Some(span) = neighbor.and_then(|line| source.line_range(line)) else {
                    actual = expected;
                    break;
                };
                let blank = {
                    (side == "Above"
                        && !include
                        && front
                            .as_ref()
                            .is_some_and(|f| f.start <= span.start && span.end <= f.end))
                        || super::is_blank(&source.text()[span])
                };
                if !blank {
                    break;
                }
                actual += 1;
            }
            if actual < expected {
                output.push(Finding {
                    heading: Heading {
                        span: heading.span.clone(),
                        level: heading.level,
                    },
                    line: first,
                    side,
                    expected,
                    actual,
                    insertion,
                });
            }
        }
    }
    output
}

pub(super) fn insertions(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Insertion> {
    findings(source, parsed, document)
        .into_iter()
        .filter_map(|finding| {
            let prefix =
                super::prefix_for(source, parsed, RegionKind::Heading, &finding.heading.span)?;
            Some(Insertion {
                at: source.line_range(finding.insertion)?.start,
                prefix,
                missing: finding.expected - finding.actual,
                rule: "MD022",
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
    findings(source,parsed,document).into_iter().map(|finding|{
        let context=display.text()[display.line_range(finding.line).unwrap()].trim_matches(js_space);
        let start=source.line_range(finding.line).unwrap().start+usize::from(finding.line==1&&source.text().starts_with('\u{feff}'))*3;
        Diagnostic{code:"MD022".into(),rule_id:Some("MD022".into()),severity:document.state("MD022",finding.line).unwrap().severity,message:format!("Headings should be surrounded by blank lines [Expected: {}; Actual: {}; {}; Context: \"{context}\"]",finding.expected,finding.actual,finding.side),span:Some(start..start)}
    }).collect()
}
