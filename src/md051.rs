use crate::{
    Diagnostic, ProcessingError, Source,
    config::{DocumentConfig, RuleState, pattern::PatternFlags},
    parse::{
        ParsedDocument,
        fragments::{encode_component, fragment_snapshot},
    },
};
use crate::{
    edit::Edit,
    parse::{RegionKind, parse},
};
use markdown::mdast::Node;
use regex::Regex;
use std::{
    collections::{HashMap, HashSet},
    sync::LazyLock,
};

fn ascii_fragment(text: &str) -> bool {
    text.strip_prefix('#').is_some_and(|tail| {
        !tail.is_empty()
            && tail
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    })
}

fn valid_fragment(value: &str, targets: &HashSet<&str>) -> bool {
    value.strip_prefix('#').is_some_and(|tail| {
        let encoded = format!("#{}", encode_component(tail));
        targets.contains(encoded.as_str()) || LINE_FRAGMENT.is_match(&encoded)
    })
}

fn ignored(
    value: &str,
    state: &RuleState,
    document: &DocumentConfig,
) -> Result<bool, ProcessingError> {
    let pattern = state
        .options
        .get("ignored_pattern")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    if pattern.is_empty() {
        return Ok(false);
    }
    document
        .pattern(pattern, PatternFlags::default(), "MD051.ignored_pattern")?
        .is_match(value.strip_prefix('#').unwrap_or(value))
}

fn same_except_fragment_case(before: &Node, after: &Node) -> bool {
    fn normalize(node: &mut Node) {
        node.position_set(None);
        if let Node::Link(link) = node
            && ascii_fragment(&link.url)
        {
            link.url.make_ascii_lowercase();
        }
        if let Some(children) = node.children_mut() {
            for child in children {
                normalize(child);
            }
        }
    }
    let mut before = before.clone();
    let mut after = after.clone();
    normalize(&mut before);
    normalize(&mut after);
    before == after
}

pub(crate) fn propose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Result<Vec<Edit>, ProcessingError> {
    if crate::parse::has_uncertain_container_prefixes(source, parsed)
        || !(1..=source.line_count()).any(|line| {
            document
                .state("MD051", line)
                .is_some_and(|state| state.enabled)
        })
    {
        return Ok(vec![]);
    }
    let snapshot = fragment_snapshot(source, parsed);
    let targets: HashSet<_> = snapshot.targets.iter().map(String::as_str).collect();
    let mut unique = HashMap::new();
    for target in &snapshot.targets {
        unique
            .entry(target.to_lowercase())
            .and_modify(|entry| *entry = None)
            .or_insert(Some(target));
    }
    let owned: HashSet<_> = parsed
        .regions
        .iter()
        .filter(|region| region.kind == RegionKind::Link)
        .map(|region| region.span.clone())
        .collect();
    let mut blocked: Vec<_> = parsed
        .regions
        .iter()
        .filter(|region| {
            matches!(
                region.kind,
                RegionKind::Code
                    | RegionKind::InlineCode
                    | RegionKind::Table
                    | RegionKind::FrontMatter
                    | RegionKind::Html
                    | RegionKind::Image
                    | RegionKind::Definition
                    | RegionKind::Protected
            )
        })
        .map(|region| region.span.clone())
        .collect();
    blocked.sort_by_key(|span| (span.start, span.end));
    let mut merged: Vec<std::ops::Range<usize>> = vec![];
    for span in blocked {
        if let Some(last) = merged.last_mut()
            && span.start <= last.end
        {
            last.end = last.end.max(span.end);
        } else {
            merged.push(span);
        }
    }
    let mut edits = vec![];
    for link in snapshot.destinations {
        if !link.inline
            || link.docfx_tab
            || !owned.contains(&link.span)
            || !ascii_fragment(&link.value)
            || source.text()[link.destination.clone()] != link.value
            || valid_fragment(&link.value, &targets)
        {
            continue;
        }
        let line = source.position(link.span.start).unwrap().line;
        if source.position(link.span.end).unwrap().line != line {
            continue;
        }
        let Some(state) = document.state("MD051", line).filter(|state| state.enabled) else {
            continue;
        };
        if ignored(&link.value, &state, document)? {
            continue;
        }
        if state
            .options
            .get("ignore_case")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
        {
            continue;
        }
        let Some(Some(target)) = unique.get(&link.value.to_lowercase()) else {
            continue;
        };
        if !ascii_fragment(target)
            || !document.allows_source("MD051", link.destination.clone(), source)
        {
            continue;
        }
        let prior = merged.partition_point(|span| span.start < link.destination.end);
        if prior > 0 && merged[prior - 1].end > link.destination.start {
            continue;
        }
        edits.push(Edit {
            span: link.destination,
            replacement: (*target).clone(),
            origin: "MD051".into(),
        });
    }
    edits.sort_by_key(|edit| (edit.span.start, edit.span.end));
    edits.dedup();
    if edits.is_empty() {
        return Ok(edits);
    }
    let safe = crate::edit::apply_authorized_edits(source, parsed, edits.clone(), &edits)
        .ok()
        .and_then(|text| {
            let candidate = source.derive(text);
            let reparsed = parse(&candidate).ok()?;
            Some(same_except_fragment_case(&parsed.tree, &reparsed.tree))
        })
        .unwrap_or(false);
    Ok(if safe { edits } else { vec![] })
}
static LINE_FRAGMENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^#(?:L[0-9]+(?:C[0-9]+)?-L[0-9]+(?:C[0-9]+)?|L[0-9]+)$").unwrap()
});

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Result<Vec<Diagnostic>, ProcessingError> {
    if !(1..=source.line_count()).any(|line| {
        document
            .state("MD051", line)
            .is_some_and(|state| state.enabled)
    }) {
        return Ok(vec![]);
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
        if !link.value.starts_with('#') || link.value.len() == 1 {
            continue;
        }
        if valid_fragment(&link.value, &targets) {
            continue;
        }
        let line = source.position(link.span.start).unwrap().line;
        let Some(state) = document.state("MD051", line).filter(|state| state.enabled) else {
            continue;
        };
        if ignored(&link.value, &state, document)? {
            continue;
        }
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
    Ok(output)
}
