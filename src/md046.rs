use crate::{
    Diagnostic, Source,
    config::DocumentConfig,
    parse::{ParsedDocument, diagnostic_code_blocks},
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
    let mut expected = None;
    let mut output = vec![];
    for block in diagnostic_code_blocks(source, parsed) {
        let actual = if block.fenced { "fenced" } else { "indented" };
        let line = source.position(block.span.start).unwrap().line;
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
