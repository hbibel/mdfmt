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

#[derive(Clone)]
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

fn budget_outcome(
    source: &Source,
    document: &DocumentConfig,
    insertion: &Insertion,
) -> (usize, Option<crate::conflicts::RuleConflict>) {
    let count = budget(source, document, insertion);
    let conflict = (count < insertion.missing).then(|| crate::conflicts::RuleConflict {
        span: insertion.at..insertion.at,
        rules: vec!["MD012".into(), insertion.rule.into()],
        retained: Some("MD012".into()),
    });
    (count, conflict)
}

fn validated_budget_conflicts(
    source: &Source,
    parsed: &ParsedDocument,
    rejected: Vec<(Insertion, crate::conflicts::RuleConflict)>,
) -> Vec<crate::conflicts::RuleConflict> {
    let newline = crate::md047::preferred_newline(source.text());
    let total = rejected
        .iter()
        .try_fold((0usize, 0usize), |(bytes, lines), (insertion, _)| {
            Some((
                bytes.checked_add(
                    (insertion.prefix.len() + newline.len()).checked_mul(insertion.missing)?,
                )?,
                lines.checked_add(insertion.missing)?,
            ))
        });
    if total.is_none_or(|(bytes, lines)| bytes > 1024 * 1024 || lines > 4096) {
        return vec![];
    }
    let candidates: Vec<_> = rejected
        .into_iter()
        .map(|(insertion, conflict)| {
            (
                Edit {
                    span: insertion.at..insertion.at,
                    replacement: format!("{}{newline}", insertion.prefix).repeat(insertion.missing),
                    origin: insertion.rule.into(),
                },
                conflict,
            )
        })
        .collect();
    let mut groups: BTreeMap<usize, Vec<Edit>> = BTreeMap::new();
    for (edit, _) in &candidates {
        let group = groups.entry(edit.span.start).or_default();
        if !group.contains(edit) {
            group.push(edit.clone());
        }
    }
    let choices: Vec<_> = groups.into_values().collect();
    let mut accepted = vec![];
    if !select_batches(source, parsed, &choices, &mut 32, &mut accepted) {
        return vec![];
    }
    candidates
        .into_iter()
        .filter_map(|(edit, conflict)| accepted.contains(&edit).then_some(conflict))
        .collect()
}

fn safe(source: &Source, parsed: &ParsedDocument, edits: &[Edit]) -> bool {
    apply_authorized_edits(source, parsed, edits.to_vec(), edits)
        .ok()
        .is_some_and(|output| {
            let output = source.derive(output);
            parse(&output).is_ok_and(|reparsed| equivalent(source, parsed, &output, &reparsed))
        })
}

fn select_batches(
    source: &Source,
    parsed: &ParsedDocument,
    choices: &[Vec<Edit>],
    remaining: &mut usize,
    output: &mut Vec<Edit>,
) -> bool {
    if choices.is_empty() {
        return true;
    }
    if *remaining == 0 {
        return false;
    }
    *remaining -= 1;
    let batch: Vec<_> = choices.iter().map(|options| options[0].clone()).collect();
    if safe(source, parsed, &batch) {
        output.extend(batch);
    } else if choices.len() == 1 {
        for alternative in &choices[0][1..] {
            if *remaining == 0 {
                return false;
            }
            *remaining -= 1;
            if safe(source, parsed, std::slice::from_ref(alternative)) {
                output.push(alternative.clone());
                break;
            }
        }
    } else {
        let middle = choices.len() / 2;
        return select_batches(source, parsed, &choices[..middle], remaining, output)
            && select_batches(source, parsed, &choices[middle..], remaining, output);
    }
    true
}

pub(crate) fn propose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Edit> {
    evaluate(source, parsed, document).edits
}

pub(crate) fn evaluate(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> crate::conflicts::RuleProposals {
    if crate::parse::has_unmodeled_syntax(source, parsed) {
        return Default::default();
    }
    let mut rejected = vec![];
    let mut selected: BTreeMap<usize, Vec<Insertion>> = BTreeMap::new();
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
        let (count, conflict) = budget_outcome(source, document, &candidate);
        if let Some(conflict) = conflict {
            rejected.push((candidate.clone(), conflict));
        }
        candidate.missing = count;
        if candidate.missing == 0 {
            continue;
        }
        selected.entry(candidate.at).or_default().push(candidate);
    }
    let newline = crate::md047::preferred_newline(source.text());
    // Valid configuration can request trillions of blanks. Keep the requested
    // diagnostic, but never allocate an unbounded replacement to satisfy it.
    let total = selected
        .values()
        .try_fold((0usize, 0usize), |(bytes, lines), candidates| {
            let count = candidates.iter().map(|c| c.missing).max()?;
            let width = candidates.iter().map(|c| c.prefix.len()).max()?;
            Some((
                bytes.checked_add((width + newline.len()).checked_mul(count)?)?,
                lines.checked_add(count)?,
            ))
        });
    // Newlines also allocate parser and per-line configuration state downstream.
    if total.is_none_or(|(bytes, lines)| bytes > 1024 * 1024 || lines > 4096) {
        return Default::default();
    }
    let choices: Vec<_> = selected
        .into_values()
        .filter_map(|mut candidates| {
            let count = candidates.iter().map(|c| c.missing).max()?;
            candidates.sort_by(|a, b| {
                a.prefix
                    .len()
                    .cmp(&b.prefix.len())
                    .then(a.prefix.cmp(&b.prefix))
            });
            candidates.dedup_by(|a, b| a.prefix == b.prefix);
            let options: Vec<_> = candidates
                .into_iter()
                .filter_map(|mut candidate| {
                    candidate.missing = count;
                    // A smaller plain-blank budget must not partially satisfy a
                    // quoted requirement and leave another edit for the next call.
                    if budget(source, document, &candidate) < count {
                        return None;
                    }
                    Some(Edit {
                        span: candidate.at..candidate.at,
                        replacement: format!("{}{newline}", candidate.prefix).repeat(count),
                        origin: candidate.rule.into(),
                    })
                })
                .collect();
            (!options.is_empty()).then_some(options)
        })
        .collect();
    let mut edits = vec![];
    // Most documents need one batch validation. Limit fallback work on mixed
    // safe/unsafe boundaries instead of reparsing once for every separator.
    let conflicts = validated_budget_conflicts(source, parsed, rejected);
    if select_batches(source, parsed, &choices, &mut 32, &mut edits) && safe(source, parsed, &edits)
    {
        crate::conflicts::RuleProposals { edits, conflicts }
    } else {
        crate::conflicts::RuleProposals {
            edits: vec![],
            conflicts,
        }
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
