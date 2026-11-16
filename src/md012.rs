use crate::{
    Diagnostic, Severity, Source,
    config::{DocumentConfig, js_space},
    edit::Edit,
    parse::{ParsedDocument, RegionKind, equivalent, parse},
};

struct Finding {
    line: usize,
    maximum: usize,
    actual: usize,
    severity: Severity,
}

fn findings(source: &Source, parsed: &ParsedDocument, document: &DocumentConfig) -> Vec<Finding> {
    let mut ignored = vec![false; source.line_count()];
    for region in &parsed.regions {
        if matches!(region.kind, RegionKind::Code | RegionKind::FrontMatter) {
            for line in crate::md013::covered_lines(source, region.span.clone()) {
                ignored[line] = true;
            }
        }
    }
    let mut count = 0;
    let mut result = vec![];
    for line in 1..=source.line_count() {
        let range = source.line_range(line).unwrap();
        if ignored[line - 1] || !source.text()[range].chars().all(js_space) {
            count = 0;
            continue;
        }
        count += 1;
        let Some(state) = document.state("MD012", line).filter(|state| state.enabled) else {
            continue;
        };
        let maximum = state
            .options
            .get("maximum")
            .and_then(|value| value.as_u64())
            .unwrap_or(1);
        let maximum = usize::try_from(maximum).unwrap_or(usize::MAX);
        if count > maximum {
            result.push(Finding {
                line,
                maximum,
                actual: count,
                severity: state.severity,
            });
        }
    }
    result
}

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    findings(source, parsed, document)
        .into_iter()
        .map(|finding| {
            let mut span = source.line_range(finding.line).unwrap();
            if finding.line == 1 && source.text().starts_with('\u{feff}') {
                span.start += 3;
            }
            Diagnostic {
                code: "MD012".into(),
                rule_id: Some("MD012".into()),
                severity: finding.severity,
                message: format!(
                    "Multiple consecutive blank lines [Expected: {}; Actual: {}]",
                    finding.maximum, finding.actual
                ),
                span: Some(span),
            }
        })
        .collect()
}

pub(crate) fn propose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Edit> {
    if crate::parse::has_unmodeled_syntax(source, parsed) {
        return vec![];
    }
    let mut spans = vec![];
    for finding in findings(source, parsed, document) {
        let line = source.line_range(finding.line).unwrap();
        let mut span = if let Some(next) = source.line_range(finding.line + 1) {
            line.start..next.start
        } else if line.is_empty() && finding.line > 1 {
            source.line_range(finding.line - 1).unwrap().end..line.end
        } else {
            line
        };
        if source.text().starts_with('\u{feff}') {
            span.start = span.start.max(3);
        }
        if !span.is_empty() && document.allows_source("MD012", span.clone(), source) {
            spans.push(span);
        }
    }
    spans.sort_by_key(|span| (span.start, span.end));
    let mut edits: Vec<Edit> = vec![];
    for span in spans {
        if let Some(last) = edits.last_mut()
            && span.start <= last.span.end
        {
            last.span.end = last.span.end.max(span.end);
        } else {
            edits.push(Edit {
                span,
                replacement: String::new(),
                origin: "MD012".into(),
            });
        }
    }
    if edits.is_empty() {
        return edits;
    }
    let safe = crate::edit::apply_authorized_edits(source, parsed, edits.clone(), &edits)
        .ok()
        .and_then(|output| {
            let candidate = source.derive(output);
            let reparsed = parse(&candidate).ok()?;
            Some(equivalent(source, parsed, &candidate, &reparsed))
        })
        .unwrap_or(false);
    if safe { edits } else { vec![] }
}
