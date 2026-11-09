use crate::{Diagnostic, Source, config::DocumentConfig, parse::ReferenceAnalysis};
use std::collections::HashSet;

pub(super) fn diagnose(
    source: &Source,
    display: &Source,
    document: &DocumentConfig,
    analysis: &ReferenceAnalysis,
) -> Vec<Diagnostic> {
    let definitions: HashSet<_> = analysis
        .definitions
        .iter()
        .map(|definition| definition.key.as_str())
        .collect();
    let mut output = vec![];
    for reference in &analysis.uses {
        if definitions.contains(reference.key.as_str()) {
            continue;
        }
        let position = source.position(reference.span.start).unwrap();
        let line = position.line;
        let Some(state) = document.state("MD052", line).filter(|state| state.enabled) else {
            continue;
        };
        if reference.shortcut
            && !state
                .options
                .get("shortcut_syntax")
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
        {
            continue;
        }
        let ignored = state
            .options
            .get("ignored_labels")
            .and_then(|value| value.as_array())
            .map_or_else(
                || reference.key == "x",
                |labels| {
                    labels
                        .iter()
                        .any(|label| label.as_str() == Some(&reference.key))
                },
            );
        if ignored {
            continue;
        }
        let end = reference.span.end.min(source.line_range(line).unwrap().end);
        let shown = &display.text()[display.line_range(line).unwrap()];
        let start = position.column_utf16 - 1;
        let length = crate::utf16_len(&source.text()[reference.span.start..end]);
        let context = String::from_utf16_lossy(
            &shown
                .encode_utf16()
                .skip(start)
                .take(length)
                .collect::<Vec<_>>(),
        );
        let detail = format!(
            "Missing link or image reference definition: \"{}\"",
            reference.key
        );
        output.push(Diagnostic{code:"MD052".into(),rule_id:Some("MD052".into()),severity:state.severity,message:format!("Reference links and images should use a label that is defined [{detail}] [Context: \"{context}\"]"),span:Some(reference.span.start..end)});
    }
    output
}
