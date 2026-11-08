use super::Insertion;
use crate::{
    Diagnostic, Source,
    config::{DocumentConfig, js_space},
    parse::{ParsedDocument, RegionKind},
};
use std::ops::Range;

struct Finding {
    span: Range<usize>,
    line: usize,
    insertion: usize,
}

fn findings(source: &Source, parsed: &ParsedDocument, document: &DocumentConfig) -> Vec<Finding> {
    if !(1..=source.line_count())
        .any(|line| document.state("MD031", line).is_some_and(|s| s.enabled))
    {
        return vec![];
    }
    let (expanded, offsets) = crate::parse::expanded_tabs(source);
    let Ok(recovered) = crate::parse::parse_flow_diagnostics(&expanded) else {
        return vec![];
    };
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
    for region in crate::heading_spaces::literal_exclusions(&expanded, &recovered, true).code {
        let raw = &expanded.text()[region.span.clone()];
        let trimmed = raw.trim_start_matches(' ');
        if raw.len() - trimmed.len() > 3 {
            continue;
        }
        let Some(marker @ (b'`' | b'~')) = trimmed.as_bytes().first().copied() else {
            continue;
        };
        if trimmed.bytes().take_while(|byte| *byte == marker).count() < 3 {
            continue;
        }
        let span = offsets[region.span.start]..offsets[region.span.end];
        let first = source.position(span.start).unwrap().line;
        let last = crate::md013::covered_lines(source, span.clone()).end;
        let Some(settings) = document.state("MD031", first) else {
            continue;
        };
        if settings.options.get("list_items").and_then(|v| v.as_bool()) == Some(false)
            && region.in_list
        {
            continue;
        }
        for (line, insertion, needed) in [
            (
                first,
                first,
                !blank(first.checked_sub(1).filter(|line| *line > 0)),
            ),
            (last, last + 1, !blank(Some(last + 1)) && !blank(Some(last))),
        ] {
            if needed && document.state("MD031", line).is_some_and(|s| s.enabled) {
                result.push(Finding {
                    span: span.clone(),
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
        .filter_map(|finding| {
            Some(Insertion {
                at: source.line_range(finding.insertion)?.start,
                prefix: super::prefix_for(source, parsed, RegionKind::Code, &finding.span)?,
                missing: 1,
                rule: "MD031",
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
        let raw=display.text()[display.line_range(finding.line).unwrap()].trim_matches(js_space);
        let context=crate::heading_structure::context(raw);
        let start=source.line_range(finding.line).unwrap().start+usize::from(finding.line==1&&source.text().starts_with('\u{feff}'))*3;
        Diagnostic{code:"MD031".into(),rule_id:Some("MD031".into()),severity:document.state("MD031",finding.line).unwrap().severity,message:format!("Fenced code blocks should be surrounded by blank lines [Context: \"{context}\"]"),span:Some(start..start)}
    }).collect()
}
