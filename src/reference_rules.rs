mod md052;
mod md053;
mod md054;

use crate::{
    Diagnostic, Source,
    config::DocumentConfig,
    parse::{
        ParsedDocument, ReferenceFields, comment_masked_source, diagnostic_parse,
        reference_analysis,
    },
};

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    let enabled = |rule| {
        (1..=source.line_count()).any(|line| {
            document
                .state(rule, line)
                .is_some_and(|state| state.enabled)
        })
    };
    let uses = enabled("MD052") || enabled("MD053");
    let styles = (1..=source.line_count()).any(|line| {
        document.state("MD054", line).is_some_and(|state| {
            state.enabled
                && [
                    "autolink",
                    "inline",
                    "full",
                    "collapsed",
                    "shortcut",
                    "url_inline",
                ]
                .iter()
                .any(|option| {
                    state.options.get(*option).and_then(|value| value.as_bool()) == Some(false)
                })
        })
    });
    if !(uses || styles) {
        return vec![];
    }
    let recovered = diagnostic_parse(source);
    let analysis = reference_analysis(
        source,
        recovered.as_ref().unwrap_or(parsed),
        ReferenceFields {
            uses,
            styles,
            destinations: false,
        },
    );
    let mut output = vec![];
    if uses {
        let display = comment_masked_source(source, parsed);
        output.extend(md052::diagnose(source, &display, document, &analysis));
        output.extend(md053::diagnose(source, &display, document, &analysis));
    }
    if styles {
        output.extend(md054::diagnose(source, document, &analysis));
    }
    output
}
