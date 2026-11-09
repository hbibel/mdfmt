use crate::{Diagnostic, Source, config::DocumentConfig, parse::ReferenceAnalysis};
use std::collections::HashSet;

pub(super) fn diagnose(
    source: &Source,
    display: &Source,
    document: &DocumentConfig,
    analysis: &ReferenceAnalysis,
) -> Vec<Diagnostic> {
    let used: HashSet<_> = analysis
        .uses
        .iter()
        .map(|reference| reference.key.as_str())
        .collect();
    let mut seen = HashSet::new();
    let mut output = vec![];
    for definition in &analysis.definitions {
        let duplicate = !seen.insert(definition.key.as_str());
        if !duplicate && used.contains(definition.key.as_str()) {
            continue;
        }
        let line = source.position(definition.span.start).unwrap().line;
        let Some(state) = document.state("MD053", line).filter(|state| state.enabled) else {
            continue;
        };
        let ignored = state
            .options
            .get("ignored_definitions")
            .and_then(|value| value.as_array())
            .map_or_else(
                || definition.key == "//",
                |labels| {
                    labels
                        .iter()
                        .any(|label| label.as_str() == Some(&definition.key))
                },
            );
        if ignored {
            continue;
        }
        let mut span = source.line_range(line).unwrap();
        if line == 1 && source.text().starts_with('\u{feff}') {
            span.start += 3;
        }
        let text = &display.text()[display.line_range(line).unwrap()];
        let text = if line == 1 {
            text.strip_prefix('\u{feff}').unwrap_or(text)
        } else {
            text
        };
        let units: Vec<_> = text.encode_utf16().take(30).collect();
        let context = if crate::utf16_len(text) > 30 {
            format!("{}...", String::from_utf16_lossy(&units))
        } else {
            text.to_owned()
        };
        let kind = if duplicate { "Duplicate" } else { "Unused" };
        let detail = format!(
            "{kind} link or image reference definition: \"{}\"",
            definition.key
        );
        output.push(Diagnostic{code:"MD053".into(),rule_id:Some("MD053".into()),severity:state.severity,message:format!("Link and image reference definitions should be needed [{detail}] [Context: \"{context}\"]"),span:Some(span)});
    }
    output
}
