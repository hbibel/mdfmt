use crate::{Diagnostic, Source, config::DocumentConfig, parse::ParsedDocument};

fn style((leading, trailing): (bool, bool)) -> &'static str {
    match (leading, trailing) {
        (true, true) => "leading_and_trailing",
        (true, false) => "leading_only",
        (false, true) => "trailing_only",
        (false, false) => "no_leading_or_trailing",
    }
}

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    if !(1..=source.line_count()).any(|line| {
        document
            .state("MD055", line)
            .is_some_and(|state| state.enabled)
    }) {
        return vec![];
    }
    let mut tables = crate::heading_spaces::literal_exclusions(source, parsed, true).tables;
    tables.sort_by_key(|table| table.span.start);
    let mut expected = None;
    let mut result = vec![];
    for table in tables {
        let Some(header) = table.rows.first() else {
            continue;
        };
        let line = source.position(header.span.start).unwrap().line;
        let Some(settings) = document.state("MD055", line) else {
            continue;
        };
        let mode = settings
            .options
            .get("style")
            .and_then(|value| value.as_str())
            .unwrap_or("consistent");
        if mode == "per_table" {
            expected = None;
        }
        if !matches!(mode, "consistent" | "per_table") {
            expected = Some((
                !matches!(mode, "no_leading_or_trailing" | "trailing_only"),
                !matches!(mode, "no_leading_or_trailing" | "leading_only"),
            ));
        }
        let rows = std::iter::once((header.span.clone(), false))
            .chain(std::iter::once((table.delimiter, true)))
            .chain(table.rows.into_iter().skip(1).map(|row| (row.span, false)));
        for (row, delimiter) in rows {
            let text = &source.text()[row.clone()];
            let leading_start = if delimiter {
                0
            } else {
                text.len() - text.trim_start_matches([' ', '\t']).len()
            };
            let Some(first) = text[leading_start..].chars().next() else {
                continue;
            };
            let last = text.chars().next_back().unwrap();
            let trimmed = text.trim_matches([' ', '\t']);
            let leading = trimmed.starts_with('|');
            let trailing = trimmed.ends_with('|')
                && trimmed[..trimmed.len() - 1]
                    .bytes()
                    .rev()
                    .take_while(|byte| *byte == b'\\')
                    .count()
                    % 2
                    == 0;
            let actual = (leading, trailing);
            let expected = *expected.get_or_insert(actual);
            let line = source.position(row.start).unwrap().line;
            let Some(state) = document.state("MD055", line).filter(|state| state.enabled) else {
                continue;
            };
            for (leading_side, want, have, span) in [
                (
                    true,
                    expected.0,
                    actual.0,
                    row.start + leading_start..row.start + leading_start + first.len_utf8(),
                ),
                (
                    false,
                    expected.1,
                    actual.1,
                    row.end - last.len_utf8()..row.end,
                ),
            ] {
                if want != have {
                    result.push(Diagnostic {
                        code: "MD055".into(),
                        rule_id: Some("MD055".into()),
                        severity: state.severity,
                        message: format!(
                            "Table pipe style [Expected: {}; Actual: {}; {} {} pipe]",
                            style(expected),
                            style(actual),
                            if want { "Missing" } else { "Unexpected" },
                            if leading_side { "leading" } else { "trailing" }
                        ),
                        span: Some(span),
                    });
                }
            }
        }
    }
    result.sort_by_key(|finding| finding.span.as_ref().unwrap().start);
    result
}
