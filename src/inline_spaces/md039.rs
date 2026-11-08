use crate::{
    Diagnostic, Source,
    config::{DocumentConfig, js_space},
    parse::InlineAnalysis,
};

fn horizontal(ch: char) -> bool {
    js_space(ch) && !matches!(ch, '\r' | '\n')
}

fn collapse(text: &str) -> String {
    let mut result = String::new();
    let mut whitespace = false;
    for ch in text.chars() {
        if js_space(ch) {
            if !whitespace {
                result.push(' ');
            }
            whitespace = true;
        } else {
            result.push(ch);
            whitespace = false;
        }
    }
    result
}

pub(super) fn diagnose(
    source: &Source,
    document: &DocumentConfig,
    analysis: &InlineAnalysis,
) -> Vec<Diagnostic> {
    let mut output = vec![];
    for label in &analysis.labels {
        let body = label.span.start + 1..label.span.end - 1;
        let text = &source.text()[body.clone()];
        let context = collapse(&source.text()[label.span.clone()]);
        for start in [true, false] {
            let trimmed = if start {
                text.trim_start_matches(js_space)
            } else {
                text.trim_end_matches(js_space)
            };
            if text.len() == trimmed.len() {
                continue;
            }
            let horizontal_text = if start {
                text.trim_start_matches(horizontal)
            } else {
                text.trim_end_matches(horizontal)
            };
            let count = text.len() - horizontal_text.len();
            let span = if count > 0 {
                if start {
                    body.start..body.start + count
                } else {
                    body.end - count..body.end
                }
            } else {
                let at = if start { body.start } else { body.end };
                let line = source.position(at).unwrap().line;
                let line = if start { line + 1 } else { line - 1 };
                let Some(range) = source.line_range(line) else {
                    continue;
                };
                let at = range.start
                    + usize::from(line == 1 && source.text().starts_with('\u{feff}')) * 3;
                at..at
            };
            let line = source.position(span.start).unwrap().line;
            let Some(state) = document.state("MD039", line).filter(|state| state.enabled) else {
                continue;
            };
            let context = super::context(&context, start);
            output.push(Diagnostic {
                code: "MD039".into(),
                rule_id: Some("MD039".into()),
                severity: state.severity,
                message: format!("Spaces inside link text [Context: \"{context}\"]"),
                span: Some(span),
            });
        }
    }
    output.sort_by_key(|finding| {
        source
            .position(finding.span.as_ref().unwrap().start)
            .unwrap()
            .line
    });
    output
}
