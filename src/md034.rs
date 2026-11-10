use crate::{
    Diagnostic, Source,
    config::DocumentConfig,
    parse::{ContentFields, ParsedDocument, content_analysis},
};

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    if !(1..=source.line_count()).any(|line| {
        document
            .state("MD034", line)
            .is_some_and(|state| state.enabled)
    }) {
        return vec![];
    }
    content_analysis(source, parsed, ContentFields { bare_urls: true })
        .bare_urls
        .into_iter()
        .filter_map(|span| {
            let line = source.position(span.start).ok()?.line;
            let state = document
                .state("MD034", line)
                .filter(|state| state.enabled)?;
            let raw = &source.text()[span.clone()];
            let context = if crate::utf16_len(raw) > 30 {
                format!(
                    "{}...",
                    String::from_utf16_lossy(&raw.encode_utf16().take(30).collect::<Vec<_>>())
                )
            } else {
                raw.to_owned()
            };
            Some(Diagnostic {
                code: "MD034".into(),
                rule_id: Some("MD034".into()),
                severity: state.severity,
                message: format!("Bare URL used [Context: \"{context}\"]"),
                span: Some(span),
            })
        })
        .collect()
}
