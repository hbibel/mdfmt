use crate::{Diagnostic, Source, config::DocumentConfig, parse::ParsedDocument};
use std::sync::LazyLock;

static REVERSED: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(^|[^\\])\(([^()]+)\)\[([^\]^][^\]]*)\]").unwrap());

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    let mut result = vec![];
    for line in 0..source.line_count() {
        let Some(state) = document
            .state("MD011", line + 1)
            .filter(|state| state.enabled)
        else {
            continue;
        };
        let range = source.line_range(line + 1).unwrap();
        let base =
            range.start + usize::from(line == 0 && source.text().starts_with('\u{feff}')) * 3;
        let text = &source.text()[base..range.end];
        for captures in REVERSED.captures_iter(text) {
            let full = captures.get(0).unwrap();
            if text.as_bytes().get(full.end()) == Some(&b'(')
                || captures[2].ends_with('\\')
                || captures[3].ends_with('\\')
            {
                continue;
            }
            let start = full.start() + captures[1].len();
            let candidate = &text[start..full.end()];
            result.push(Diagnostic {
                code: "MD011".into(),
                rule_id: Some("MD011".into()),
                severity: state.severity,
                message: format!("Reversed link syntax [{candidate}]"),
                span: Some(base + start..base + full.end()),
            });
        }
    }
    if result.is_empty() {
        return result;
    }
    let excluded = crate::heading_spaces::literal_exclusions(source, parsed, true);
    result.retain(|finding| {
        let span = finding.span.as_ref().unwrap();
        let line = source.position(span.start).unwrap().line - 1;
        !excluded.flow[line]
            && !excluded
                .inline
                .iter()
                .any(|literal| literal.start < span.end && span.start < literal.end)
    });
    result
}
