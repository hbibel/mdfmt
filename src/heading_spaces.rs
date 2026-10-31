use crate::{
    Diagnostic, Severity, Source,
    config::DocumentConfig,
    edit::Edit,
    parse::{ParsedDocument, RegionKind},
};
use std::ops::Range;

pub(crate) fn excluded_flow_lines(source: &Source, parsed: &ParsedDocument) -> Vec<bool> {
    fn collect(source: &Source, parsed: &ParsedDocument) -> Vec<bool> {
        let mut ignored = vec![false; source.line_count()];
        let mut siblings = std::collections::HashMap::new();
        for (index, region) in parsed.regions.iter().enumerate() {
            let previous = region
                .parent
                .and_then(|parent| siblings.insert(parent, index));
            let html_flow = region.kind == RegionKind::Html
                && region.parent.is_some_and(|parent| {
                    matches!(
                        parsed.regions[parent].kind,
                        RegionKind::Root | RegionKind::Blockquote | RegionKind::ListItem
                    )
                });
            let first = source.position(region.span.start).unwrap().line;
            let after_container = html_flow
                && previous.is_some_and(|previous| {
                    let previous = &parsed.regions[previous];
                    matches!(previous.kind, RegionKind::List | RegionKind::Blockquote)
                        && source
                            .position(
                                previous.span.start
                                    + source.text()[previous.span.clone()]
                                        .trim_end_matches([' ', '\t', '\r', '\n'])
                                        .len(),
                            )
                            .unwrap()
                            .line
                            + 1
                            == first
                });
            if after_container {
                let probe = Source::new(format!("x\n{}", &source.text()[region.span.clone()]));
                let generic = crate::parse::parse(&probe).ok().is_some_and(|parsed| {
                    parsed.regions.iter().any(|region| {
                        region.kind == RegionKind::Paragraph
                            && probe.position(region.span.start).unwrap().line == 1
                            && probe.position(region.span.end).unwrap().line > 1
                    })
                });
                if generic {
                    ignored[first - 1] = true;
                    let next = source
                        .line_range(first + 1)
                        .map(|span| span.start)
                        .unwrap_or(region.span.end);
                    if next < region.span.end {
                        // The generic HTML line still belongs to the old container;
                        // its remaining native HTML span must restart as root flow.
                        let suffix =
                            Source::new(format!("\n{}", &source.text()[next..region.span.end]));
                        if let Ok(parsed) = crate::parse::parse_flow_diagnostics(&suffix) {
                            for (line, excluded) in
                                collect(&suffix, &parsed).into_iter().skip(1).enumerate()
                            {
                                if first + line < ignored.len() {
                                    ignored[first + line] = excluded;
                                }
                            }
                        }
                    }
                    continue;
                }
            }
            if matches!(region.kind, RegionKind::Code | RegionKind::FrontMatter) || html_flow {
                for line in crate::md013::covered_lines(source, region.span.clone()) {
                    ignored[line] = true;
                }
            }
        }
        ignored
    }
    if source.text().contains("$$") {
        crate::parse::parse_flow_diagnostics(source)
            .map(|parsed| collect(source, &parsed))
            .unwrap_or_else(|_| collect(source, parsed))
    } else {
        collect(source, parsed)
    }
}

fn findings(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<(Range<usize>, &'static str, Severity)> {
    let mut result = vec![];
    for region in &parsed.regions {
        if region.kind != RegionKind::Heading {
            continue;
        }
        let raw = source.text()[region.span.clone()].trim_start_matches([' ', '\t']);
        if !raw.starts_with('#') || raw.contains(['\r', '\n']) {
            continue;
        }
        let base = region.span.end - raw.len();
        let opening_end = raw.bytes().take_while(|&byte| byte == b'#').count();
        let opening_gap_end = raw.len() - raw[opening_end..].trim_start_matches([' ', '\t']).len();
        let trimmed = raw.trim_end_matches([' ', '\t']);
        let closing_start = trimmed.trim_end_matches('#').len();
        let closed = closing_start < trimmed.len()
            && closing_start >= opening_end
            && raw[..closing_start].ends_with([' ', '\t']);
        let rule = if closed { "MD021" } else { "MD019" };
        let line = source.position(region.span.start).unwrap().line;
        let Some(state) = document.state(rule, line).filter(|state| state.enabled) else {
            continue;
        };
        let mut gaps = Vec::with_capacity(2);
        gaps.push(opening_end..opening_gap_end);
        if closed {
            let start = raw[..closing_start].trim_end_matches([' ', '\t']).len();
            gaps.push(start..closing_start);
        }
        for gap in gaps {
            if gap.len() > 1 {
                result.push((base + gap.start + 1..base + gap.end, rule, state.severity));
            }
        }
    }
    result
}

pub(crate) fn propose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Edit> {
    let mut edits: Vec<_> = findings(source, parsed, document)
        .into_iter()
        .map(|(span, rule, _)| Edit {
            span,
            replacement: String::new(),
            origin: rule.into(),
        })
        .collect();
    edits.sort_by_key(|edit| (edit.span.start, edit.span.end));
    edits.dedup();
    edits
}

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    findings(source, parsed, document)
        .into_iter()
        .map(|(span, rule, severity)| Diagnostic {
            code: rule.into(),
            rule_id: Some(rule.into()),
            severity,
            message: if rule == "MD019" {
                "Multiple spaces after hash on atx style heading"
            } else {
                "Multiple spaces inside hashes on closed atx style heading"
            }
            .into(),
            span: Some(span),
        })
        .collect()
}
