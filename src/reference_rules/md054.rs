use crate::{
    Diagnostic, Source,
    config::DocumentConfig,
    parse::{LinkKind, ReferenceAnalysis},
};

pub(super) fn diagnose(
    source: &Source,
    document: &DocumentConfig,
    analysis: &ReferenceAnalysis,
) -> Vec<Diagnostic> {
    let mut output = vec![];
    for style in &analysis.styles {
        let first = source.position(style.span.start).unwrap();
        let Some(state) = document
            .state("MD054", first.line)
            .filter(|state| state.enabled)
        else {
            continue;
        };
        let allowed = |option| {
            state
                .options
                .get(option)
                .and_then(|value| value.as_bool())
                .unwrap_or(true)
        };
        let option = match style.kind {
            LinkKind::Autolink => "autolink",
            LinkKind::Inline => "inline",
            LinkKind::Full => "full",
            LinkKind::Collapsed => "collapsed",
            LinkKind::Shortcut => "shortcut",
        };
        let mut error = !allowed(option);
        if matches!(style.kind, LinkKind::Inline)
            && !allowed("url_inline")
            && allowed("autolink")
            && !source.text()[style.span.clone()].starts_with('!')
            && style.title.as_ref().is_none_or(|title| title.is_empty())
            && let Some((label, destination)) = style.label.as_ref().zip(style.destination.as_ref())
        {
            let label = &source.text()[label.clone()];
            let destination = &source.text()[destination.clone()];
            error |= label == destination
                && !destination.contains([' ', '<', '>'])
                && url::Url::parse(destination).is_ok();
        }
        if !error {
            continue;
        }
        let raw = &source.text()[style.span.clone()];
        let first_line = raw.split(['\r', '\n']).next().unwrap();
        let units: Vec<_> = first_line.encode_utf16().take(30).collect();
        let context = if crate::utf16_len(first_line) > 30 {
            format!("{}...", String::from_utf16_lossy(&units))
        } else {
            first_line.to_owned()
        };
        let end = source.position(style.span.end).unwrap();
        let span = if first.line == end.line {
            style.span.clone()
        } else {
            let start = source.line_range(first.line).unwrap().start
                + usize::from(first.line == 1 && source.text().starts_with('\u{feff}')) * 3;
            start..start
        };
        output.push(Diagnostic {
            code: "MD054".into(),
            rule_id: Some("MD054".into()),
            severity: state.severity,
            message: format!("Link and image style [Context: \"{context}\"]"),
            span: Some(span),
        });
    }
    output
}
