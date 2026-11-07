use crate::{
    Diagnostic, Source,
    config::{DocumentConfig, js_space},
    edit::{Edit, apply_authorized_edits},
    parse::{ParsedDocument, RegionKind, equivalent, parse},
};

fn blank_line(mut text: String) -> bool {
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

struct Candidate {
    line: usize,
    insertion: usize,
    table: std::ops::Range<usize>,
}
fn candidates(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Candidate> {
    if !source.text().contains('|')
        || !(1..=source.line_count()).any(|line| {
            document
                .state("MD058", line)
                .is_some_and(|state| state.enabled)
        })
    {
        return vec![];
    }
    let mut candidates = vec![];
    for table in crate::heading_spaces::diagnostic_tables(source, parsed) {
        let lines = crate::md013::covered_lines(source, table.span.clone());
        let first = lines.start + 1;
        let last = lines.end;
        for (line, neighbor, insertion) in [
            (first, first.checked_sub(1), first),
            (last, Some(last + 1), last + 1),
        ] {
            if neighbor
                .and_then(|line| source.line_range(line))
                .is_some_and(|span| {
                    !parsed.regions.iter().any(|region| {
                        region.kind == RegionKind::FrontMatter
                            && region.span.start <= span.start
                            && span.end <= region.span.end
                    }) && !blank_line(source.text()[span].into())
                })
                && document
                    .state("MD058", line)
                    .is_some_and(|state| state.enabled)
            {
                candidates.push(Candidate {
                    line,
                    insertion,
                    table: table.span.clone(),
                });
            }
        }
    }
    candidates.sort_by_key(|candidate| candidate.line);
    candidates.dedup_by_key(|candidate| (candidate.line, candidate.insertion));
    candidates
}

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    candidates(source, parsed, document)
        .into_iter()
        .map(|candidate| {
            let range = source.line_range(candidate.line).unwrap();
            let raw = &source.text()[range.clone()];
            let text = raw.trim_matches(js_space);
            let context = if crate::utf16_len(text) > 30 {
                format!(
                    "{}...",
                    String::from_utf16_lossy(&text.encode_utf16().take(30).collect::<Vec<_>>())
                )
            } else {
                text.into()
            };
            let start =
                range.start + usize::from(candidate.line == 1 && raw.starts_with('\u{feff}')) * 3;
            Diagnostic {
                code: "MD058".into(),
                rule_id: Some("MD058".into()),
                severity: document.state("MD058", candidate.line).unwrap().severity,
                message: format!(
                    "Tables should be surrounded by blank lines [Context: \"{context}\"]"
                ),
                span: Some(start..start),
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
    let newline = crate::md047::preferred_newline(source.text());
    let mut edits = vec![];
    for candidate in candidates(source, parsed, document) {
        // Recovered tables lack a native ownership tree suitable for edit validation.
        let Some(index) = parsed
            .regions
            .iter()
            .position(|region| region.kind == RegionKind::Table && region.span == candidate.table)
        else {
            continue;
        };
        let mut parent = parsed.regions[index].parent;
        let mut list = false;
        while let Some(index) = parent {
            list |= parsed.regions[index].kind == RegionKind::ListItem;
            parent = parsed.regions[index].parent;
        }
        if list {
            continue;
        }
        let Some(range) = source.line_range(candidate.insertion) else {
            continue;
        };
        let offset = range.start;
        // Inserting before a line targeted by disable-next-line changes the
        // configuration of that original line, even for unrelated rules.
        if document.has_any_line_override(candidate.insertion) {
            continue;
        }
        let row = source.line_range(candidate.line).unwrap();
        let raw = &source.text()[row.clone()];
        let prefix = crate::parse::ContainerPath::for_region(source, parsed, index)
            .content_start(raw)
            .unwrap_or(0);
        let prefix = &raw[..prefix];
        if !prefix.chars().all(|ch| matches!(ch, ' ' | '\t' | '>')) {
            continue;
        }
        if !document.allows_source("MD058", offset..offset, source) {
            continue;
        }
        edits.push(Edit {
            span: offset..offset,
            replacement: format!("{prefix}{newline}"),
            origin: "MD058".into(),
        });
    }
    edits.sort_by_key(|edit| edit.span.start);
    edits.dedup_by_key(|edit| edit.span.start);
    let Ok(output) = apply_authorized_edits(source, parsed, edits.clone(), &edits) else {
        return vec![];
    };
    let output = Source::new(output);
    if parse(&output).is_ok_and(|reparsed| equivalent(source, parsed, &output, &reparsed)) {
        edits
    } else {
        vec![]
    }
}
