mod md022;
mod md031;
mod md032;

use crate::{
    Diagnostic, Source,
    config::{DocumentConfig, js_space},
    edit::{Edit, apply_authorized_edits},
    parse::{ParsedDocument, equivalent, parse},
};
use std::collections::BTreeMap;

pub(crate) struct Insertion {
    pub at: usize,
    pub prefix: String,
    pub missing: usize,
    pub rule: &'static str,
}

pub(crate) fn is_blank(text: &str) -> bool {
    let mut text = text.to_owned();
    loop {
        match (text.find("<!--"), text.find("-->")) {
            (start, Some(end)) if start.is_none_or(|start| end < start) => {
                text = text[end + 3..].into()
            }
            (Some(start), Some(end)) => text.replace_range(start..end + 3, ""),
            (Some(start), None) => text.truncate(start),
            (None, None) => return text.chars().all(|ch| ch == '>' || js_space(ch)),
            _ => unreachable!(),
        }
    }
}

fn prefix_for(
    source: &Source,
    parsed: &ParsedDocument,
    kind: crate::parse::RegionKind,
    span: &std::ops::Range<usize>,
) -> Option<String> {
    let index = parsed
        .regions
        .iter()
        .position(|r| r.kind == kind && r.span == *span)?;
    let line = source.position(span.start).ok()?.line;
    let raw = &source.text()[source.line_range(line)?];
    let end = crate::parse::ContainerPath::for_region(source, parsed, index)
        .content_start_at(raw, line)?;
    Some(
        raw[..end]
            .chars()
            .map(|ch| if ch == '>' { '>' } else { ' ' })
            .collect::<String>()
            .trim_end()
            .to_owned(),
    )
}

fn budget(source: &Source, document: &DocumentConfig, insertion: &Insertion) -> usize {
    if !insertion.prefix.chars().all(js_space) {
        return insertion.missing;
    }
    let line = source.position(insertion.at).unwrap().line;
    let Some(state) = document.state("MD012", line).filter(|s| s.enabled) else {
        return insertion.missing;
    };
    let maximum = state
        .options
        .get("maximum")
        .and_then(|v| v.as_u64())
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(1);
    let blank = |line| {
        source
            .line_range(line)
            .is_some_and(|span| source.text()[span].chars().all(js_space))
    };
    let before = (1..line).rev().take_while(|line| blank(*line)).count();
    let after = (line..=source.line_count())
        .take_while(|line| blank(*line))
        .count();
    insertion
        .missing
        .min(maximum.saturating_sub(before + after))
}

fn safe(source: &Source, parsed: &ParsedDocument, edits: &[Edit]) -> bool {
    apply_authorized_edits(source, parsed, edits.to_vec(), edits)
        .ok()
        .is_some_and(|output| {
            let output = Source::new(output);
            parse(&output).is_ok_and(|reparsed| equivalent(source, parsed, &output, &reparsed))
        })
}

pub(crate) fn propose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Edit> {
    if crate::parse::has_unmodeled_syntax(source, parsed) {
        return vec![];
    }
    let mut selected: BTreeMap<usize, Option<Insertion>> = BTreeMap::new();
    for mut candidate in md022::insertions(source, parsed, document)
        .into_iter()
        .chain(md031::insertions(source, parsed, document))
        .chain(md032::insertions(source, parsed, document))
    {
        let line = source.position(candidate.at).unwrap().line;
        if document.has_any_line_override(line)
            || !document.allows_source(candidate.rule, candidate.at..candidate.at, source)
        {
            continue;
        }
        let mut line = line;
        while let Some(previous) = line
            .checked_sub(1)
            .filter(|line| *line > 0)
            .and_then(|line| source.line_range(line))
        {
            if source.text()[previous.clone()].trim_end_matches(js_space) != candidate.prefix {
                break;
            }
            candidate.at = previous.start;
            line -= 1;
        }
        if document.has_any_line_override(line)
            || !document.allows_source(candidate.rule, candidate.at..candidate.at, source)
        {
            continue;
        }
        candidate.missing = budget(source, document, &candidate);
        if candidate.missing == 0 {
            continue;
        }
        match selected.entry(candidate.at) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(Some(candidate));
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                if let Some(prior) = entry.get_mut() {
                    if prior.prefix != candidate.prefix {
                        *entry.get_mut() = None;
                    } else if candidate.missing > prior.missing {
                        *prior = candidate;
                    }
                }
            }
        }
    }
    let newline = crate::md047::preferred_newline(source.text());
    // Valid configuration can request trillions of blanks. Keep the requested
    // diagnostic, but never allocate an unbounded replacement to satisfy it.
    let total = selected
        .values()
        .flatten()
        .try_fold(0usize, |total, candidate| {
            total.checked_add(
                (candidate.prefix.len() + newline.len()).checked_mul(candidate.missing)?,
            )
        });
    if total.is_none_or(|bytes| bytes > 1024 * 1024) {
        return vec![];
    }
    let edits: Vec<_> = selected
        .into_values()
        .flatten()
        .filter_map(|candidate| {
            let edit = Edit {
                span: candidate.at..candidate.at,
                replacement: format!("{}{newline}", candidate.prefix).repeat(candidate.missing),
                origin: candidate.rule.into(),
            };
            if !safe(source, parsed, std::slice::from_ref(&edit)) {
                return None;
            }
            Some(edit)
        })
        .collect();
    if safe(source, parsed, &edits) {
        edits
    } else {
        vec![]
    }
}

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    let mut diagnostics = md022::diagnose(source, parsed, document);
    diagnostics.extend(md031::diagnose(source, parsed, document));
    diagnostics.extend(md032::diagnose(source, parsed, document));
    diagnostics
}
