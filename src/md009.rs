use crate::{
    Diagnostic, Severity, Source,
    config::{DocumentConfig, js_space},
    edit::Edit,
    parse::{ParsedDocument, RegionKind, equivalent, parse},
    utf16_len,
};
use std::ops::Range;

const CODE: u8 = 1;
const LIST: u8 = 2;
const PARAGRAPH: u8 = 4;
const INLINE_CODE: u8 = 8;
const FRONT: u8 = 16;

fn classify_in(source: &Source, parsed: &ParsedDocument) -> Vec<u8> {
    let mut classes = vec![0; source.line_count()];
    let mut content_ends = vec![0; parsed.regions.len()];
    for (index, region) in parsed.regions.iter().enumerate().rev() {
        if matches!(
            region.kind,
            RegionKind::Paragraph
                | RegionKind::Code
                | RegionKind::Heading
                | RegionKind::Table
                | RegionKind::Html
                | RegionKind::Definition
                | RegionKind::ThematicBreak
        ) {
            content_ends[index] = content_ends[index]
                .max(crate::md013::covered_lines(source, region.span.clone()).end);
        }
        if let Some(parent) = region.parent {
            content_ends[parent] = content_ends[parent].max(content_ends[index]);
        }
    }
    for (index, region) in parsed.regions.iter().enumerate() {
        let covered = crate::md013::covered_lines(source, region.span.clone());
        let (class, lines) = match region.kind {
            RegionKind::Code => {
                let raw = &source.text()[region.span.clone()];
                let trimmed = raw.trim_start_matches([' ', '\t']);
                let fenced = raw.len() - trimmed.len() <= 3
                    && trimmed.as_bytes().first().is_some_and(|marker| {
                        matches!(marker, b'`' | b'~')
                            && trimmed.bytes().take_while(|byte| byte == marker).count() >= 3
                    });
                let lines = if fenced {
                    covered.start + 1
                        ..source
                            .position(region.span.end)
                            .unwrap()
                            .line
                            .saturating_sub(1)
                } else {
                    covered
                };
                (CODE, lines)
            }
            RegionKind::ListItem => (
                LIST,
                covered.start..content_ends[index].max(covered.start + 1),
            ),
            RegionKind::Paragraph => (PARAGRAPH, covered.start..covered.end.saturating_sub(1)),
            RegionKind::InlineCode => (INLINE_CODE, covered.start..covered.end.saturating_sub(1)),
            RegionKind::FrontMatter => (FRONT, covered),
            _ => continue,
        };
        for line in lines {
            classes[line] |= class;
        }
    }
    classes
}

fn classify(source: &Source, parsed: &ParsedDocument) -> Vec<u8> {
    if source.text().contains('\t') {
        let (expanded, _) = crate::parse::expanded_tabs(source);
        if let Ok(parsed) = parse(&expanded) {
            return classify_in(&expanded, &parsed);
        }
    }
    classify_in(source, parsed)
}

struct Finding {
    span: Range<usize>,
    expected: usize,
    actual: usize,
    severity: Severity,
}

fn findings(source: &Source, parsed: &ParsedDocument, document: &DocumentConfig) -> Vec<Finding> {
    let classes = classify(source, parsed);
    let mut result = vec![];
    for line in 1..=source.line_count() {
        let Some(state) = document.state("MD009", line).filter(|state| state.enabled) else {
            continue;
        };
        let mut range = source.line_range(line).unwrap();
        if line == 1 && source.text().starts_with('\u{feff}') {
            range.start += 3;
        }
        let text = &source.text()[range.clone()];
        let trimmed = text.trim_end_matches(js_space);
        let actual = utf16_len(&text[trimmed.len()..]);
        let flag = |name| {
            state
                .options
                .get(name)
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
        };
        let br_spaces = state
            .options
            .get("br_spaces")
            .and_then(|value| value.as_u64())
            .unwrap_or(2);
        let expected =
            usize::try_from(if br_spaces < 2 { 0 } else { br_spaces }).unwrap_or(usize::MAX);
        let class = classes[line - 1];
        if actual > 0
            && class & FRONT == 0
            && (flag("code_blocks") || class & CODE == 0)
            && (!flag("list_item_empty_lines") || class & LIST == 0)
            && (actual != expected
                || (flag("strict") && (class & PARAGRAPH == 0 || class & INLINE_CODE != 0)))
        {
            result.push(Finding {
                span: range.start + trimmed.len()..range.end,
                expected,
                actual,
                severity: state.severity,
            });
        }
    }
    result
}

pub(crate) fn propose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Edit> {
    if crate::parse::has_unmodeled_syntax(source, parsed) {
        return vec![];
    }
    let mut blocked = vec![false; source.line_count()];
    for region in &parsed.regions {
        if matches!(
            region.kind,
            RegionKind::Code | RegionKind::Heading | RegionKind::Table | RegionKind::FrontMatter
        ) {
            for line in crate::md013::covered_lines(source, region.span.clone()) {
                blocked[line] = true;
            }
        }
    }
    let edits: Vec<_> = findings(source, parsed, document)
        .into_iter()
        .filter(|finding| {
            let line = source.position(finding.span.start).unwrap().line;
            !blocked[line - 1]
                && document.allows_source("MD009", finding.span.clone(), source)
                && !parsed.regions.iter().any(|region| {
                    region.kind.is_protected()
                        && finding.span.start < region.span.end
                        && region.span.start < finding.span.end
                })
        })
        .map(|finding| Edit {
            span: finding.span,
            replacement: String::new(),
            origin: "MD009".into(),
        })
        .collect();
    if edits.is_empty() {
        return edits;
    }
    let safe = crate::edit::apply_authorized_edits(source, parsed, edits.clone(), &edits)
        .ok()
        .and_then(|output| {
            let candidate = Source::new(output);
            let reparsed = parse(&candidate).ok()?;
            Some(equivalent(source, parsed, &candidate, &reparsed))
        })
        .unwrap_or(false);
    if safe { edits } else { vec![] }
}

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    findings(source, parsed, document)
        .into_iter()
        .map(|finding| Diagnostic {
            code: "MD009".into(),
            rule_id: Some("MD009".into()),
            severity: finding.severity,
            message: format!(
                "Trailing spaces [Expected: {}{}; Actual: {}]",
                if finding.expected == 0 { "" } else { "0 or " },
                finding.expected,
                finding.actual
            ),
            span: Some(finding.span),
        })
        .collect()
}
