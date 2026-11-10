use crate::{
    Diagnostic, Source,
    config::DocumentConfig,
    parse::{ParsedDocument, RegionKind, expanded_tabs, parse_flow_diagnostics},
};

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    if !(1..=source.line_count()).any(|line| {
        document
            .state("MD046", line)
            .is_some_and(|state| state.enabled)
    }) {
        return vec![];
    }
    let expanded = source
        .text()
        .contains('\t')
        .then(|| expanded_tabs(source).0);
    let recovered = match expanded.as_ref() {
        Some(view) => match parse_flow_diagnostics(view) {
            Ok(parsed) => Some(parsed),
            Err(_) => return vec![],
        },
        None => None,
    };
    let view = expanded.as_ref().unwrap_or(source);
    let parsed = recovered.as_ref().unwrap_or(parsed);
    let mut expected = None;
    let mut output = vec![];
    let owners: std::collections::HashMap<_, _> = parsed
        .regions
        .iter()
        .filter(|region| region.kind == RegionKind::Code)
        .map(|region| (region.span.clone(), region.parent))
        .collect();
    let mut blocks = crate::heading_spaces::literal_exclusions(view, parsed, true).code;
    blocks.sort_by_key(|block| block.span.start);
    let mut previous = None;
    for block in blocks {
        let raw = &view.text()[block.span.clone()];
        let trimmed = raw.trim_start_matches(' ');
        let marker = trimmed.as_bytes().first().copied();
        let fenced = raw.len() - trimmed.len() <= 3
            && matches!(marker, Some(b'`' | b'~'))
            && trimmed
                .bytes()
                .take_while(|byte| Some(*byte) == marker)
                .count()
                >= 3;
        let actual = if fenced { "fenced" } else { "indented" };
        let owner = owners.get(&block.span).copied();
        let continuation = !fenced
            && previous.is_some_and(|(end, previous_owner)| {
                owner.is_some()
                    && owner == previous_owner
                    && view.text()[end..block.span.start]
                        .bytes()
                        .all(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n' | b'>'))
            });
        previous = (!fenced).then_some((block.span.end, owner));
        if continuation {
            continue;
        }
        let line = view.position(block.span.start).unwrap().line;
        let Some(state) = document.state("MD046", line) else {
            continue;
        };
        let expected = *expected.get_or_insert_with(|| {
            match state
                .options
                .get("style")
                .and_then(|option| option.as_str())
            {
                Some("fenced") => "fenced",
                Some("indented") => "indented",
                _ => actual,
            }
        });
        if state.enabled && actual != expected {
            let start = source.line_range(line).unwrap().start
                + usize::from(line == 1 && source.text().starts_with('\u{feff}')) * 3;
            output.push(Diagnostic {
                code: "MD046".into(),
                rule_id: Some("MD046".into()),
                severity: state.severity,
                message: format!("Code block style [Expected: {expected}; Actual: {actual}]"),
                span: Some(start..start),
            });
        }
    }
    output
}
