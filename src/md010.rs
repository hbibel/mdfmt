use crate::{
    Diagnostic, Severity, Source,
    config::DocumentConfig,
    edit::Edit,
    parse::{ParsedDocument, RegionKind, equivalent, parse},
};
use std::ops::Range;

struct CodeRange {
    span: Range<usize>,
    language: Option<String>,
}

fn code_ranges(source: &Source) -> Vec<CodeRange> {
    let (expanded, offsets) = crate::parse::expanded_tabs(source);
    let Ok(parsed) = parse(&expanded) else {
        return vec![];
    };
    parsed
        .regions
        .iter()
        .filter_map(|region| {
            if !matches!(region.kind, RegionKind::Code | RegionKind::InlineCode) {
                return None;
            }
            let raw = &expanded.text()[region.span.clone()];
            let trimmed = raw.trim_start_matches(' ');
            let fenced = region.kind == RegionKind::Code
                && raw.len() - trimmed.len() <= 3
                && trimmed.as_bytes().first().is_some_and(|marker| {
                    matches!(marker, b'`' | b'~')
                        && trimmed.bytes().take_while(|byte| byte == marker).count() >= 3
                });
            if fenced {
                let marker = trimmed.as_bytes()[0];
                let info = trimmed
                    .trim_start_matches(char::from(marker))
                    .split(['\r', '\n'])
                    .next()
                    .unwrap_or("");
                let language = info
                    .trim_start_matches([' ', '\t'])
                    .split([' ', '\t'])
                    .next()
                    .unwrap_or("")
                    .to_lowercase();
                let start_line = expanded.position(region.span.start).unwrap().line + 1;
                let mut end_line = expanded
                    .position(region.span.end)
                    .unwrap()
                    .line
                    .saturating_sub(1);
                let mut parent = region.parent;
                while let Some(index) = parent {
                    let ancestor = &parsed.regions[index];
                    if ancestor.kind == RegionKind::Blockquote {
                        end_line = end_line.min(
                            crate::md013::covered_lines(&expanded, region.span.clone())
                                .end
                                .saturating_sub(1),
                        );
                        break;
                    }
                    parent = ancestor.parent;
                }
                if start_line > end_line {
                    return None;
                }
                let start = expanded.line_range(start_line)?.start;
                let end = expanded.line_range(end_line)?.end;
                Some(CodeRange {
                    span: offsets[start]..offsets[end],
                    language: Some(language),
                })
            } else {
                Some(CodeRange {
                    span: offsets[region.span.start]..offsets[region.span.end],
                    language: None,
                })
            }
        })
        .collect()
}

struct Finding {
    span: Range<usize>,
    column: usize,
    multiplier: u64,
    severity: Severity,
}

fn findings(source: &Source, parsed: &ParsedDocument, document: &DocumentConfig) -> Vec<Finding> {
    let code = code_ranges(source);
    let mut result = vec![];
    for line in 1..=source.line_count() {
        let Some(state) = document.state("MD010", line).filter(|state| state.enabled) else {
            continue;
        };
        let include_code = state
            .options
            .get("code_blocks")
            .and_then(|value| value.as_bool())
            .unwrap_or(true);
        let ignored: Vec<_> = state
            .options
            .get("ignore_code_languages")
            .and_then(|value| value.as_array())
            .into_iter()
            .flatten()
            .filter_map(|value| value.as_str())
            .map(str::to_lowercase)
            .collect();
        let range = source.line_range(line).unwrap();
        let text = &source.text()[range.clone()];
        let mut cursor = 0;
        while let Some(index) = text[cursor..].find('\t') {
            let start = cursor + index;
            cursor = start
                + text[start..]
                    .bytes()
                    .take_while(|byte| *byte == b'\t')
                    .count();
            let span = range.start + start..range.start + cursor;
            if parsed.regions.iter().any(|region| {
                region.kind == RegionKind::FrontMatter
                    && region.span.start <= span.start
                    && span.end <= region.span.end
            }) {
                continue;
            }
            // Upstream's inclusive overlap includes a tab immediately after code.
            if code.iter().any(|code| {
                let excluded = match &code.language {
                    Some(language) if !ignored.is_empty() => {
                        language.is_empty() || ignored.contains(language)
                    }
                    _ => !include_code,
                };
                excluded && span.start <= code.span.end && code.span.start < span.end
            }) {
                continue;
            }
            let column = source.position(span.start).unwrap().column_utf16
                - usize::from(line == 1 && source.text().starts_with('\u{feff}'));
            result.push(Finding {
                span,
                column,
                multiplier: state
                    .options
                    .get("spaces_per_tab")
                    .and_then(|value| value.as_u64())
                    .unwrap_or(1),
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
    if crate::parse::has_unmodeled_syntax(source, parsed)
        || crate::parse::has_uncertain_container_prefixes(source, parsed)
    {
        return vec![];
    }
    let budget = source
        .text()
        .len()
        .saturating_mul(4)
        .saturating_add(1 << 20);
    let mut total = 0usize;
    let mut edits = vec![];
    for finding in findings(source, parsed, document) {
        if !document.allows_source("MD010", finding.span.clone(), source)
            || parsed.regions.iter().any(|region| {
                region.kind.is_protected()
                    && region.span.start < finding.span.end
                    && finding.span.start < region.span.end
            })
        {
            continue;
        }
        let Some(length) = usize::try_from(finding.multiplier)
            .ok()
            .and_then(|multiplier| finding.span.len().checked_mul(multiplier))
        else {
            return vec![];
        };
        let Some(next) = total.checked_add(length).filter(|next| *next <= budget) else {
            return vec![];
        };
        total = next;
        edits.push(Edit {
            span: finding.span,
            replacement: " ".repeat(length),
            origin: "MD010".into(),
        });
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

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    findings(source, parsed, document)
        .into_iter()
        .map(|finding| Diagnostic {
            code: "MD010".into(),
            rule_id: Some("MD010".into()),
            severity: finding.severity,
            message: format!("Hard tabs [Column: {}]", finding.column),
            span: Some(finding.span),
        })
        .collect()
}
