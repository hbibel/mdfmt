use crate::{
    Diagnostic, Source,
    config::{DocumentConfig, js_space},
    edit::Edit,
    parse::{ParsedDocument, Region, RegionKind},
};

fn blank_line(mut text: String) -> bool {
    loop {
        match (text.find("<!--"), text.find("-->")) {
            (start, Some(end)) if start.is_none_or(|start| end < start) => {
                text = text[end + 3..].into();
            }
            (Some(start), Some(end)) => text.replace_range(start..end + 3, ""),
            (Some(start), None) => text.truncate(start),
            (None, None) => return text.chars().all(|ch| ch == '>' || js_space(ch)),
            _ => unreachable!(),
        }
    }
}

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    let line = source.line_count();
    let Some(state) = document.state("MD047", line).filter(|state| state.enabled) else {
        return vec![];
    };
    let range = source.line_range(line).unwrap();
    let text = &source.text()[range.clone()];
    if blank_line(text.into())
        || parsed.regions.iter().any(|region| {
            region.kind == RegionKind::FrontMatter && region.span.end == source.text().len()
        })
    {
        return vec![];
    }
    let last = text.char_indices().next_back().unwrap().0;
    vec![Diagnostic {
        code: "MD047".into(),
        rule_id: Some("MD047".into()),
        severity: state.severity,
        message: "Files should end with a single newline character".into(),
        span: Some(range.start + last..range.end),
    }]
}

fn closed_fence(text: &str, parsed: &ParsedDocument, region: &Region) -> bool {
    fn indentation(text: &str) -> Option<&str> {
        let trimmed = text.trim_start_matches(' ');
        (text.len() - trimmed.len() <= 3 && !trimmed.starts_with('\t')).then_some(trimmed)
    }
    let Some(opening) = indentation(text) else {
        return false;
    };
    let Some(marker @ (b'`' | b'~')) = opening.bytes().next() else {
        return false;
    };
    let count = opening.bytes().take_while(|&byte| byte == marker).count();
    let Some(last_break) = text.rfind(['\r', '\n']) else {
        return false;
    };
    let mut closing = &text[last_break + 1..];
    let mut parent = region.parent;
    while let Some(index) = parent {
        let ancestor = &parsed.regions[index];
        if ancestor.kind == RegionKind::Blockquote {
            let Some(rest) = indentation(closing).and_then(|text| text.strip_prefix('>')) else {
                return false;
            };
            closing = rest.strip_prefix(' ').unwrap_or(rest);
        }
        parent = ancestor.parent;
    }
    let Some(closing) = indentation(closing) else {
        return false;
    };
    let closing_count = closing.bytes().take_while(|&byte| byte == marker).count();
    count >= 3
        && closing_count >= count
        && closing[closing_count..]
            .trim_matches([' ', '\t'])
            .is_empty()
}

pub(crate) fn propose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
    newline: &str,
) -> Vec<Edit> {
    let end = source.text().len();
    if crate::parse::has_uncertain_container_prefixes(source, parsed)
        || diagnose(source, parsed, document).is_empty()
        || !document.allows_source("MD047", end..end, source)
        || parsed.regions.iter().any(|region| {
            region.span.end == end
                && match region.kind {
                    RegionKind::Code => {
                        !closed_fence(&source.text()[region.span.clone()], parsed, region)
                    }
                    RegionKind::Html => {
                        let text = &source.text()[region.span.clone()];
                        !(text.starts_with("<!--") && text.ends_with("-->"))
                    }
                    RegionKind::FrontMatter | RegionKind::Table | RegionKind::Protected => true,
                    _ => false,
                }
        })
    {
        return vec![];
    }
    vec![Edit {
        span: end..end,
        replacement: newline.into(),
        origin: "MD047".into(),
    }]
}

pub(crate) fn preferred_newline(text: &str) -> &'static str {
    text.rfind(['\r', '\n']).map_or("\n", |index| {
        if text.as_bytes()[index] == b'\r' {
            "\r"
        } else if index > 0 && text.as_bytes()[index - 1] == b'\r' {
            "\r\n"
        } else {
            "\n"
        }
    })
}
