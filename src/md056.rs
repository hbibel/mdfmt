use crate::{Diagnostic, Source, config::DocumentConfig, parse::ParsedDocument};

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    if !(1..=source.line_count()).any(|line| {
        document
            .state("MD056", line)
            .is_some_and(|state| state.enabled)
    }) {
        return vec![];
    }
    let mut result = vec![];
    for table in crate::heading_spaces::diagnostic_tables(source, parsed) {
        let Some(first) = table.rows.first() else {
            continue;
        };
        let expected = first.cells.len();
        for row in table.rows {
            let actual = row.cells.len();
            if actual == expected {
                continue;
            }
            let line = source.position(row.span.end).unwrap().line;
            let Some(state) = document.state("MD056", line).filter(|state| state.enabled) else {
                continue;
            };
            let (span, detail) = if actual < expected {
                let last = source.text()[row.span.clone()].chars().next_back().unwrap();
                (
                    row.span.end - last.len_utf8()..row.span.end,
                    "Too few cells, row will be missing data",
                )
            } else {
                (
                    row.cells[expected].start..row.span.end,
                    "Too many cells, extra data will be missing",
                )
            };
            result.push(Diagnostic {
                code: "MD056".into(),
                rule_id: Some("MD056".into()),
                severity: state.severity,
                message: format!(
                    "Table column count [Expected: {expected}; Actual: {actual}; {detail}]"
                ),
                span: Some(span),
            });
        }
    }
    result.sort_by_key(|finding| finding.span.as_ref().unwrap().start);
    result
}
