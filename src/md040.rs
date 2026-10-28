use crate::{
    Diagnostic, Source,
    config::DocumentConfig,
    parse::{ParsedDocument, RegionKind, parse},
};

pub(crate) fn diagnose(
    source: &Source,
    _parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    let (expanded, offsets) = crate::parse::expanded_tabs(source);
    let Ok(parsed) = parse(&expanded) else {
        return vec![];
    };
    let mut result = vec![];
    for region in &parsed.regions {
        if region.kind != RegionKind::Code {
            continue;
        }
        let raw = &expanded.text()[region.span.clone()];
        let trimmed = raw.trim_start_matches(' ');
        if raw.len() - trimmed.len() > 3 {
            continue;
        }
        let Some(marker @ (b'`' | b'~')) = trimmed.as_bytes().first().copied() else {
            continue;
        };
        if trimmed.bytes().take_while(|byte| *byte == marker).count() < 3 {
            continue;
        }
        let line = expanded.position(region.span.start).unwrap().line;
        let Some(state) = document.state("MD040", line).filter(|state| state.enabled) else {
            continue;
        };
        let start = offsets[region.span.start + raw.len() - trimmed.len()];
        let end = source.line_range(line).unwrap().end;
        let text = &source.text()[start..end];
        let count = text.bytes().take_while(|byte| *byte == marker).count();
        let info = text[count..].trim_start_matches([' ', '\t']);
        let length = info
            .bytes()
            .take_while(|byte| !matches!(byte, b' ' | b'\t'))
            .count();
        let language = &info[..length];
        let allowed = state
            .options
            .get("allowed_languages")
            .and_then(|value| value.as_array());
        let mut messages = vec![];
        if language.is_empty() {
            let context = if crate::utf16_len(text) > 30 {
                format!(
                    "{}...",
                    String::from_utf16_lossy(&text.encode_utf16().take(30).collect::<Vec<_>>())
                )
            } else {
                text.into()
            };
            messages.push(format!("Context: \"{context}\""));
        } else if allowed.is_some_and(|allowed| {
            !allowed.is_empty() && !allowed.iter().any(|value| value.as_str() == Some(language))
        }) {
            messages.push(format!("\"{language}\" is not allowed"));
        }
        if state
            .options
            .get("language_only")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
            && !info[length..].trim_matches([' ', '\t']).is_empty()
        {
            messages.push(format!(
                "Info string contains more than language: \"{text}\""
            ));
        }
        let base = source.line_range(line).unwrap().start
            + usize::from(line == 1 && source.text().starts_with('\u{feff}')) * 3;
        result.extend(messages.into_iter().map(|message| Diagnostic {
            code: "MD040".into(),
            rule_id: Some("MD040".into()),
            severity: state.severity,
            message: format!("Fenced code blocks should have a language specified [{message}]"),
            span: Some(base..base),
        }));
    }
    result
}
