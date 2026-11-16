use crate::{
    Diagnostic, ProcessingError, Source,
    config::{DocumentConfig, RuleState, js_space},
    md024::{Heading, headings_in},
    parse::{ParsedDocument, RegionKind},
};
use markdown::mdast::Node;

pub(crate) fn context(text: &str) -> String {
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    if crate::utf16_len(&text) > 30 {
        format!(
            "{}...",
            String::from_utf16_lossy(&text.encode_utf16().take(30).collect::<Vec<_>>())
        )
    } else {
        text
    }
}

fn description(rule: &str) -> &'static str {
    match rule {
        "MD001" => "Heading levels should only increment by one level at a time",
        "MD025" => "Multiple top-level headings in the same document",
        "MD041" => "First line in a file should be a top-level heading",
        "MD043" => "Required heading structure",
        _ => unreachable!(),
    }
}

fn emit(
    output: &mut Vec<Diagnostic>,
    source: &Source,
    document: &DocumentConfig,
    rule: &str,
    line: usize,
    suffix: String,
) {
    if let Some(state) = document.state(rule, line).filter(|state| state.enabled) {
        let start = source.line_range(line).unwrap().start
            + usize::from(line == 1 && source.text().starts_with('\u{feff}')) * 3;
        output.push(Diagnostic {
            code: rule.into(),
            rule_id: Some(rule.into()),
            severity: state.severity,
            message: format!("{}{suffix}", description(rule)),
            span: Some(start..start),
        });
    }
}

fn title(
    source: &Source,
    parsed: &ParsedDocument,
    state: &RuleState,
    document: &DocumentConfig,
    rule: &str,
) -> Result<bool, ProcessingError> {
    let pattern = state
        .options
        .get("front_matter_title")
        .and_then(|v| v.as_str());
    if pattern == Some("") || !document.enabled_anywhere(rule) {
        return Ok(false);
    }
    let pattern = document.pattern(
        pattern.unwrap_or(r#"^\s*"?title"?\s*[:=]"#),
        crate::config::pattern::PatternFlags {
            unicode: false,
            ignore_case: true,
        },
        &format!("{rule}.front_matter_title"),
    )?;
    for region in parsed
        .regions
        .iter()
        .filter(|r| r.kind == RegionKind::FrontMatter)
    {
        for line in source.text()[region.span.clone()]
            .replace("\r\n", "\n")
            .replace('\r', "\n")
            .split_terminator('\n')
        {
            if pattern.is_match(line)? {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn html_comment(raw: &str) -> bool {
    if matches!(raw, "<!-->" | "<!--->") {
        return true;
    }
    raw.strip_prefix("<!--")
        .and_then(|s| s.strip_suffix("-->"))
        .is_some_and(|s| !s.starts_with('>') && !s.starts_with("->") && !s.ends_with('-'))
}

fn content_nodes(parsed: &ParsedDocument) -> Vec<&Node> {
    parsed
        .tree
        .children()
        .into_iter()
        .flatten()
        .filter(|node| !matches!(node, Node::Html(html) if html_comment(&html.value)))
        .collect()
}

fn required(
    source: &Source,
    document: &DocumentConfig,
    headings: &[Heading],
    state: &RuleState,
    output: &mut Vec<Diagnostic>,
) -> Result<(), ProcessingError> {
    if !document.enabled_anywhere("MD043") {
        return Ok(());
    }
    let Some(requirements) = state.options.get("headings").and_then(|v| v.as_array()) else {
        return Ok(());
    };
    let requirements: Vec<_> = requirements.iter().filter_map(|v| v.as_str()).collect();
    let match_case = state
        .options
        .get("match_case")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let matches = |actual: &str, expected: &str| -> Result<bool, ProcessingError> {
        if expected.starts_with('%') {
            let count = expected
                .bytes()
                .take_while(|byte| matches!(byte, b'%' | b'#'))
                .count();
            let level = actual.bytes().take_while(|byte| *byte == b'#').count();
            if count != level {
                return Ok(false);
            }
            let pattern = expected[count..].trim_start_matches(js_space);
            return document
                .pattern(
                    pattern,
                    crate::config::pattern::PatternFlags {
                        unicode: true,
                        ignore_case: !match_case,
                    },
                    "MD043.headings",
                )?
                .is_match(actual[level..].trim_start_matches(js_space));
        }
        if match_case {
            Ok(actual == expected)
        } else {
            Ok(actual.to_lowercase() == expected.to_lowercase())
        }
    };
    let is_loop = |s: &str| !s.is_empty() && s.bytes().all(|b| b == b'^');
    let get_expected = |index: &mut usize, skip: bool| {
        loop {
            let result = requirements.get(*index).copied().unwrap_or("[No heading]");
            *index += 1;
            if !skip || !is_loop(result) {
                break result;
            }
        }
    };
    let mut index = 0;
    let mut loops = 0;
    let mut fallback = false;
    for heading in headings {
        let actual = format!("{} {}", "#".repeat(heading.level), heading.text);
        let mut expected = get_expected(&mut index, false);
        if is_loop(expected) {
            let current = index;
            let next = get_expected(&mut index, true);
            if matches(&actual, next)? {
                expected = next;
            } else {
                loops += 1;
                if let Some(previous) = requirements[..current.saturating_sub(1)]
                    .iter()
                    .rposition(|s| *s == expected)
                {
                    index = previous + 1;
                    expected = get_expected(&mut index, false);
                } else {
                    expected = next;
                }
            }
        }
        let line = source.position(heading.start).unwrap().line;
        if loops > 1000 {
            emit(
                output,
                source,
                document,
                "MD043",
                line,
                " [Too many loops] [Context: \"Limit: 1000\"]".into(),
            );
            return Ok(());
        }
        match expected {
            "*" => {
                let next = get_expected(&mut index, true);
                if !matches(&actual, next)? {
                    fallback = true;
                    index -= 1;
                }
            }
            "+" => fallback = true,
            "?" => {}
            _ if matches(&actual, expected)? => fallback = false,
            _ if fallback => index -= 1,
            _ => {
                emit(
                    output,
                    source,
                    document,
                    "MD043",
                    line,
                    format!(" [Expected: {expected}; Actual: {actual}]"),
                );
                return Ok(());
            }
        }
    }
    if let Some(expected) = requirements
        .iter()
        .skip(index)
        .find(|s| **s != "*" && !is_loop(s))
    {
        emit(
            output,
            source,
            document,
            "MD043",
            source.line_count(),
            format!(" [Expected: {expected}; Actual: [End of file]]"),
        );
    }
    Ok(())
}

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Result<Vec<Diagnostic>, ProcessingError> {
    let recovered = crate::parse::diagnostic_parse(source);
    let parsed = recovered.as_deref().unwrap_or(parsed);
    let headings = headings_in(source, parsed, None);
    let nodes = content_nodes(parsed);
    let base = parsed
        .regions
        .iter()
        .find(|r| r.kind == RegionKind::Root && r.parent.is_some())
        .map_or(0, |r| r.span.start);
    let mut output = vec![];
    if let Some(state) = document.state("MD001", 1) {
        let mut previous = if title(source, parsed, &state, document, "MD001")? {
            1
        } else {
            usize::MAX
        };
        for heading in &headings {
            if heading.level > previous.saturating_add(1) {
                emit(
                    &mut output,
                    source,
                    document,
                    "MD001",
                    source.position(heading.start).unwrap().line,
                    format!(" [Expected: h{}; Actual: h{}]", previous + 1, heading.level),
                );
            }
            previous = heading.level;
        }
    }
    if let Some(state) = document.state("MD025", 1) {
        let level = state
            .options
            .get("level")
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as usize;
        let matching: Vec<_> = headings
            .iter()
            .filter(|h| h.level == level && !h.docfx_tab)
            .collect();
        let has_title = title(source, parsed, &state, document, "MD025")?;
        let first_is_title = matching.first().is_some_and(|h| {
            nodes.first().is_some_and(|n| {
                matches!(n, Node::Heading(_))
                    && n.position()
                        .is_some_and(|p| base + p.start.offset == h.start)
            })
        });
        if has_title || first_is_title {
            for heading in matching.iter().skip(usize::from(!has_title)) {
                let suffix = if heading.text.is_empty() {
                    String::new()
                } else {
                    format!(" [Context: \"{}\"]", context(&heading.text))
                };
                emit(
                    &mut output,
                    source,
                    document,
                    "MD025",
                    source.position(heading.start).unwrap().line,
                    suffix,
                );
            }
        }
    }
    if let Some(state) = document.state("MD041", 1)
        && !title(source, parsed, &state, document, "MD041")?
    {
        let level = state
            .options
            .get("level")
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as usize;
        let preamble = state
            .options
            .get("allow_preamble")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        for node in nodes {
            let heading_level = match node {
                Node::Heading(h) => Some(usize::from(h.depth)),
                Node::Html(h) => {
                    let tag = h
                        .value
                        .trim_start_matches([' ', '\t'])
                        .strip_prefix('<')
                        .unwrap_or("")
                        .trim_start_matches('/');
                    let name = tag
                        .split(|ch: char| js_space(ch) || ch == '>' || ch == '/')
                        .next()
                        .unwrap_or("");
                    (name.len() == 2
                        && name.as_bytes()[0].eq_ignore_ascii_case(&b'h')
                        && matches!(name.as_bytes()[1], b'1'..=b'6'))
                    .then(|| usize::from(name.as_bytes()[1] - b'0'))
                }
                _ => None,
            };
            if heading_level.is_some() || !preamble {
                if heading_level != Some(level) {
                    let line = source
                        .position(base + node.position().unwrap().start.offset)
                        .unwrap()
                        .line;
                    let span = source.line_range(line).unwrap();
                    let raw = source.text()[span].trim_start_matches('\u{feff}');
                    let mut sanitized = String::new();
                    let mut rest = raw;
                    while let Some(start) = rest.find("<!--") {
                        sanitized.push_str(&rest[..start + 4]);
                        let Some(end) = rest[start + 4..].find("-->") else {
                            sanitized.push_str(&rest[start + 4..]);
                            rest = "";
                            break;
                        };
                        for ch in rest[start + 4..start + 4 + end].chars() {
                            if ch == ' ' {
                                sanitized.push(ch)
                            } else {
                                sanitized.extend(std::iter::repeat_n('.', ch.len_utf16()));
                            }
                        }
                        rest = &rest[start + 4 + end..];
                    }
                    sanitized.push_str(rest);
                    emit(
                        &mut output,
                        source,
                        document,
                        "MD041",
                        line,
                        format!(" [Context: \"{}\"]", context(&sanitized)),
                    );
                }
                break;
            }
        }
    }
    if let Some(state) = document.state("MD043", 1) {
        required(source, document, &headings, &state, &mut output)?;
    }
    Ok(output)
}
