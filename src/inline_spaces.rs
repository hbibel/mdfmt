mod md037;
mod md038;

use crate::{
    Diagnostic, Source,
    config::DocumentConfig,
    parse::{ParsedDocument, comment_masked_source, diagnostic_parse, inline_analysis},
};

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    if !(1..=source.line_count()).any(|line| {
        ["MD037", "MD038"].iter().any(|rule| {
            document
                .state(rule, line)
                .is_some_and(|state| state.enabled)
        })
    }) {
        return vec![];
    }
    let recovered = diagnostic_parse(source);
    let analysis = inline_analysis(source, recovered.as_ref().unwrap_or(parsed));
    let display = comment_masked_source(source, parsed);
    let mut output = md037::diagnose(source, &display, document, &analysis);
    output.extend(md038::diagnose(source, document, &analysis));
    output
}

fn context(raw: &str, start: bool) -> String {
    let normalized = raw.replace("\r\n", "\n").replace('\r', "\n");
    let units: Vec<_> = normalized.encode_utf16().collect();
    let value = if units.len() <= 30 {
        normalized
    } else if start {
        format!("{}...", String::from_utf16_lossy(&units[..30]))
    } else {
        format!(
            "...{}",
            String::from_utf16_lossy(&units[units.len() - 30..])
        )
    };
    value.replace('\n', " ")
}
