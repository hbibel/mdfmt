mod md037;

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
        document
            .state("MD037", line)
            .is_some_and(|state| state.enabled)
    }) {
        return vec![];
    }
    let recovered = diagnostic_parse(source);
    let analysis = inline_analysis(source, recovered.as_ref().unwrap_or(parsed));
    let display = comment_masked_source(source, parsed);
    md037::diagnose(source, &display, document, &analysis)
}
