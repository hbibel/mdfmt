use crate::{
    Diagnostic, Source,
    config::{DocumentConfig, js_space},
    parse::{ParsedDocument, html_tokens},
};

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    html_tokens(source, parsed)
        .into_iter()
        .filter_map(|token| {
            let raw = &source.text()[token.span.clone()];
            let rest = raw.strip_prefix('<')?;
            if rest.starts_with(['!', '>', '/']) {
                return None;
            }
            let end = rest
                .char_indices()
                .find(|(_, ch)| matches!(ch, '/' | '>') || js_space(*ch))
                .map_or(rest.len(), |(i, _)| i);
            let name = &rest[..end];
            let line = source.position(token.span.start).ok()?.line;
            let state = document.state("MD033", line).filter(|s| s.enabled)?;
            let allowed = if token.in_table {
                state
                    .options
                    .get("table_allowed_elements")
                    .or_else(|| state.options.get("allowed_elements"))
            } else {
                state.options.get("allowed_elements")
            };
            if allowed.and_then(|v| v.as_array()).is_some_and(|values| {
                values.iter().any(|v| {
                    v.as_str()
                        .is_some_and(|s| s.to_lowercase() == name.to_lowercase())
                })
            }) {
                return None;
            }
            Some(Diagnostic {
                code: "MD033".into(),
                rule_id: Some("MD033".into()),
                severity: state.severity,
                message: format!("Inline HTML [Element: {name}]"),
                span: Some(token.span.start..token.span.end.min(source.line_range(line)?.end)),
            })
        })
        .collect()
}
