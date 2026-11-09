mod md052;
mod md053;

use crate::{
    Diagnostic, Source,
    config::DocumentConfig,
    parse::{ParsedDocument, comment_masked_source, diagnostic_parse, reference_analysis},
};

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    if !(1..=source.line_count()).any(|line| {
        ["MD052", "MD053"].iter().any(|rule| {
            document
                .state(rule, line)
                .is_some_and(|state| state.enabled)
        })
    }) {
        return vec![];
    }
    let recovered = diagnostic_parse(source);
    let analysis = reference_analysis(source, recovered.as_ref().unwrap_or(parsed));
    let display = comment_masked_source(source, parsed);
    let mut output = md052::diagnose(source, &display, document, &analysis);
    output.extend(md053::diagnose(source, &display, document, &analysis));
    output
}
