use crate::{
    Diagnostic, Source,
    config::DocumentConfig,
    parse::{ParsedDocument, RegionKind, parse},
};

fn js_space(ch: char) -> bool {
    ch == '\u{feff}' || (ch.is_whitespace() && ch != '\u{85}')
}

fn flow_lines(source: &Source, parsed: &ParsedDocument) -> Vec<bool> {
    let mut ignored = vec![false; source.line_count()];
    for region in &parsed.regions {
        let html_flow = region.kind == RegionKind::Html
            && region.parent.is_some_and(|parent| {
                matches!(
                    parsed.regions[parent].kind,
                    RegionKind::Root | RegionKind::Blockquote | RegionKind::ListItem
                )
            });
        if matches!(region.kind, RegionKind::Code | RegionKind::FrontMatter) || html_flow {
            for line in crate::md013::covered_lines(source, region.span.clone()) {
                ignored[line] = true;
            }
        }
    }
    ignored
}

fn ignored_lines(source: &Source, parsed: &ParsedDocument) -> Vec<bool> {
    let mut ignored = flow_lines(source, parsed);
    let mut math = vec![false; source.line_count()];
    let mut masked = source.text().to_owned();
    let mut open = None;
    for line in 0..source.line_count() {
        let span = source.line_range(line + 1).unwrap();
        let raw = &source.text()[span.clone()];
        let text = if line == 0 {
            raw.strip_prefix('\u{feff}').unwrap_or(raw)
        } else {
            raw
        };
        let trimmed = text.trim_start_matches(' ');
        let indent = text.len() - trimmed.len();
        let dollars = trimmed.bytes().take_while(|byte| *byte == b'$').count();
        if let Some((first, width)) = open {
            math[line] = true;
            let closing = indent <= 3
                && dollars >= width
                && trimmed[dollars..].trim_matches([' ', '\t']).is_empty();
            if closing || line + 1 == source.line_count() {
                for number in first..=line {
                    let span = source.line_range(number + 1).unwrap();
                    masked.replace_range(span.clone(), &" ".repeat(span.len()));
                }
                // Native code/HTML inside math must not leak beyond the math fence.
                let neutral = Source::new(masked.clone());
                if let Ok(parsed) = parse(&neutral) {
                    ignored = flow_lines(&neutral, &parsed);
                }
                open = None;
            }
        } else if !ignored[line] && indent <= 3 && dollars >= 2 && !trimmed[dollars..].contains('$')
        {
            open = Some((line, dollars));
            math[line] = true;
        }
    }
    ignored
        .into_iter()
        .zip(math)
        .map(|(flow, math)| flow || math)
        .collect()
}

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    let ignored = ignored_lines(source, parsed);
    let mut result = vec![];
    for (line, ignored) in ignored.into_iter().enumerate() {
        if ignored {
            continue;
        }
        let span = source.line_range(line + 1).unwrap();
        let base = span.start + usize::from(line == 0 && source.text().starts_with('\u{feff}')) * 3;
        let text = &source.text()[base..span.end];
        let hashes = text.bytes().take_while(|byte| *byte == b'#').count();
        let Some(ch) = text[hashes..].chars().next() else {
            continue;
        };
        if hashes == 0
            || matches!(ch, '#' | ' ' | '\t')
            || text.trim_end_matches(js_space).ends_with('#')
            || text.starts_with("#️⃣")
        {
            continue;
        }
        let Some(state) = document
            .state("MD018", line + 1)
            .filter(|state| state.enabled)
        else {
            continue;
        };
        let text = text.trim_matches(js_space);
        let context = if crate::utf16_len(text) > 30 {
            format!(
                "{}...",
                String::from_utf16_lossy(&text.encode_utf16().take(30).collect::<Vec<_>>())
            )
        } else {
            text.into()
        };
        result.push(Diagnostic {
            code: "MD018".into(),
            rule_id: Some("MD018".into()),
            severity: state.severity,
            message: format!("No space after hash on atx style heading [Context: \"{context}\"]"),
            span: Some(base..base + hashes + ch.len_utf8()),
        });
    }
    result
}
