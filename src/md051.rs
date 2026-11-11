use crate::{
    Diagnostic, Source,
    config::DocumentConfig,
    parse::{
        ParsedDocument,
        fragments::{encode_component, fragment_snapshot},
    },
};
use regex::Regex;
use std::{
    collections::{HashMap, HashSet},
    sync::LazyLock,
};
static LINE_FRAGMENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^#(?:L[0-9]+(?:C[0-9]+)?-L[0-9]+(?:C[0-9]+)?|L[0-9]+)$").unwrap()
});

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    if !(1..=source.line_count()).any(|line| {
        document
            .state("MD051", line)
            .is_some_and(|state| state.enabled)
    }) {
        return vec![];
    }
    let snapshot = fragment_snapshot(source, parsed);
    let targets: HashSet<_> = snapshot.targets.iter().map(String::as_str).collect();
    let mut lower = HashMap::new();
    for target in &snapshot.targets {
        lower.entry(target.to_lowercase()).or_insert(target);
    }
    let mut output = vec![];
    for link in snapshot.destinations {
        if link.destination.is_empty() || link.inline && link.docfx_tab {
            continue;
        }
        let Some(tail) = link.value.strip_prefix('#').filter(|tail| !tail.is_empty()) else {
            continue;
        };
        let encoded = format!("#{}", encode_component(tail));
        if targets.contains(encoded.as_str()) || LINE_FRAGMENT.is_match(&encoded) {
            continue;
        }
        let line = source.position(link.span.start).unwrap().line;
        let Some(state) = document.state("MD051", line).filter(|state| state.enabled) else {
            continue;
        };
        let expected = lower.get(&link.value.to_lowercase());
        if expected.is_some_and(|expected| expected.as_str() == link.value)
            || expected.is_some()
                && state
                    .options
                    .get("ignore_case")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false)
        {
            continue;
        }
        let single = source.position(link.span.end).unwrap().line == line;
        let span = if single {
            link.span.clone()
        } else {
            let start = source.line_range(line).unwrap().start
                + usize::from(line == 1 && source.text().starts_with('\u{feff}')) * 3;
            start..start
        };
        let detail = expected.map_or(String::new(), |expected| {
            format!(" [Expected: {expected}; Actual: {}]", link.value)
        });
        let context = if single {
            format!(" [Context: \"{}\"]", &source.text()[link.span])
        } else {
            String::new()
        };
        output.push(Diagnostic {
            code: "MD051".into(),
            rule_id: Some("MD051".into()),
            severity: state.severity,
            message: format!("Link fragments should be valid{detail}{context}"),
            span: Some(span),
        });
    }
    output
}
