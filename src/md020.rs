use crate::{Diagnostic, Source, config::DocumentConfig, parse::ParsedDocument};
use std::sync::LazyLock;

static CLOSED: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
    r"^(#+)([ \t]*)([^# \t\\]|[^# \t][^#]*?[^# \t\\])([ \t]*)((?:\\#)?)(#+)([\t\n\x0b\x0c\r \u00a0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000\uFEFF]*)$"
).unwrap()
});

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    let mut result = vec![];
    for line in 0..source.line_count() {
        let Some(state) = document
            .state("MD020", line + 1)
            .filter(|state| state.enabled)
        else {
            continue;
        };
        let span = source.line_range(line + 1).unwrap();
        let base = span.start + usize::from(line == 0 && source.text().starts_with('\u{feff}')) * 3;
        let text = &source.text()[base..span.end];
        let Some(captures) = CLOSED.captures(text) else {
            continue;
        };
        let left = captures[2].is_empty();
        let right = captures[4].is_empty() || !captures[5].is_empty();
        if !left && !right {
            continue;
        }
        let range = if left {
            let end = captures.get(1).unwrap().end();
            0..end + text[end..].chars().next().unwrap().len_utf8()
        } else {
            let end = captures.get(6).unwrap().end();
            let start = captures.get(6).unwrap().start();
            let preceding = text[..start].chars().next_back().unwrap();
            start - preceding.len_utf8()..end
        };
        let context = text
            .trim_matches(|ch: char| ch == '\u{feff}' || (ch.is_whitespace() && ch != '\u{85}'));
        let units: Vec<_> = context.encode_utf16().collect();
        let context = if units.len() <= 30 {
            context.into()
        } else if left && right {
            format!(
                "{}...{}",
                String::from_utf16_lossy(&units[..15]),
                String::from_utf16_lossy(&units[units.len() - 15..])
            )
        } else if right {
            format!(
                "...{}",
                String::from_utf16_lossy(&units[units.len() - 30..])
            )
        } else {
            format!("{}...", String::from_utf16_lossy(&units[..30]))
        };
        result.push(Diagnostic {
            code: "MD020".into(),
            rule_id: Some("MD020".into()),
            severity: state.severity,
            message: format!(
                "No space inside hashes on closed atx style heading [Context: \"{context}\"]"
            ),
            span: Some(base + range.start..base + range.end),
        });
    }
    if result.is_empty() {
        return result;
    }
    let ignored = crate::heading_spaces::excluded_flow_lines(source, parsed);
    result.retain(|finding| {
        !ignored[source
            .position(finding.span.as_ref().unwrap().start)
            .unwrap()
            .line
            - 1]
    });
    result
}
