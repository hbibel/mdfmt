use crate::{
    Diagnostic, Source,
    config::{DocumentConfig, js_space},
    edit::Edit,
    parse::{ParsedDocument, RegionKind},
};
use regex::Regex;
use std::{ops::Range, sync::LazyLock};

const DEFAULT_PUNCTUATION: &str = ".,;:!。，；：！";

static ENTITY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"&(?:#\d+|#[xX][\da-fA-F]+|[a-zA-Z]{2,31}|blk\d{2}|emsp1[34]|frac\d{2}|sup\d|there4);$",
    )
    .unwrap()
});
static EMOJI: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r":(?:[abmovx]|[-+]1|100|1234|(?:1st|2nd|3rd)_place_medal|8ball|clock[0-9]{1,4}|e-mail|non-potable_water|o2|t-rex|u5272|u5408|u55b6|u6307|u6708|u6709|u6e80|u7121|u7533|u7981|u7a7a|[a-z]{2,15}2?|[a-z]{1,14}(?:_[a-z0-9]{1,16})+):$").unwrap()
});

fn heading_text(source: &Source, span: Range<usize>) -> Option<Range<usize>> {
    let raw = &source.text()[span.clone()];
    if raw.starts_with('#') {
        let markers = raw.bytes().take_while(|&byte| byte == b'#').count();
        let text = raw[markers..].trim_start_matches([' ', '\t']);
        let start = span.end - text.len();
        let mut text = text.trim_end_matches([' ', '\t']);
        let without_hashes = text.trim_end_matches('#');
        if without_hashes.len() < text.len()
            && (without_hashes.is_empty() || without_hashes.ends_with([' ', '\t']))
        {
            text = without_hashes.trim_end_matches([' ', '\t']);
        }
        Some(start..start + text.len())
    } else {
        let underline_line = source.position(span.end).ok()?.line;
        let line = source.line_range(underline_line.checked_sub(1)?)?;
        // Restrict deletion to the final content line. Upstream can produce an
        // invalid (negative-column) range for a multiline punctuation-only tail.
        let start = line.start.max(span.start);
        let text = source.text()[start..line.end].trim_start_matches([' ', '\t', '>']);
        Some(line.end - text.len()..line.end)
    }
}

fn suffix(text: &str, punctuation: &str) -> Option<usize> {
    if text.is_empty() || ENTITY.is_match(text) || EMOJI.is_match(text) {
        return None;
    }
    let end = text.trim_end_matches(|ch| punctuation.contains(ch));
    if end.len() == text.len() {
        return None;
    }
    Some(end.trim_end_matches(js_space).len())
}

fn findings(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
    settle: bool,
) -> Vec<(Range<usize>, crate::Severity)> {
    let mut findings = vec![];
    for region in &parsed.regions {
        if region.kind != RegionKind::Heading {
            continue;
        }
        let Some(content) = heading_text(source, region.span.clone()) else {
            continue;
        };
        let line = source.position(content.end).unwrap().line;
        let Some(state) = document.state("MD026", line).filter(|state| state.enabled) else {
            continue;
        };
        let punctuation = state
            .options
            .get("punctuation")
            .and_then(|v| v.as_str())
            .unwrap_or(DEFAULT_PUNCTUATION);
        let text = &source.text()[content.clone()];
        let Some(mut start) = suffix(text, punctuation) else {
            continue;
        };
        if settle {
            while let Some(next) = suffix(&text[..start], punctuation) {
                start = next;
            }
        }
        let span = content.start + start..content.end;
        if document.allows_source("MD026", span.clone(), source) {
            findings.push((span, state.severity));
        }
    }
    findings
}

pub(crate) fn propose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Edit> {
    findings(source, parsed, document, true)
        .into_iter()
        .map(|(span, _)| Edit {
            span,
            replacement: String::new(),
            origin: "MD026".into(),
        })
        .collect()
}

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    findings(source, parsed, document, false)
        .into_iter()
        .map(|(span, severity)| Diagnostic {
            code: "MD026".into(),
            rule_id: Some("MD026".into()),
            severity,
            message: format!(
                "Trailing punctuation in heading [Punctuation: '{}']",
                &source.text()[span.clone()]
            ),
            span: Some(span),
        })
        .collect()
}
