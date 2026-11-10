use crate::{
    Diagnostic, Source,
    config::DocumentConfig,
    parse::{ContentFields, ContentKind, ParsedDocument, content_analysis},
};
use markdown::mdast::Node;
use std::{
    collections::{BTreeMap, HashSet},
    ops::Range,
    sync::LazyLock,
};

// Equal-length name priority follows ICU collation, not Unicode scalar order.
static COLLATOR: LazyLock<icu_collator::CollatorBorrowed<'static>> = LazyLock::new(|| {
    icu_collator::Collator::try_new(Default::default(), Default::default())
        .expect("compiled root collation data")
});

fn overlaps(ranges: &BTreeMap<usize, usize>, span: &Range<usize>) -> bool {
    ranges
        .range(..span.end)
        .next_back()
        .is_some_and(|(_, end)| *end > span.start)
}
fn word(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}
fn pattern(name: &str) -> regress::Regex {
    let escaped = name
        .chars()
        .flat_map(|ch| {
            if "\\^$.*+?()[]{}|/".contains(ch) {
                vec!['\\', ch]
            } else {
                vec![ch]
            }
        })
        .collect::<String>();
    let left = if name.chars().next().is_some_and(word) {
        "\\b_*"
    } else {
        ""
    };
    let right = if name.chars().next_back().is_some_and(word) {
        "_*\\b"
    } else {
        ""
    };
    regress::Regex::from_unicode(
        format!("({left})({escaped}){right}")
            .encode_utf16()
            .map(u32::from),
        regress::Flags {
            icase: true,
            ..Default::default()
        },
    )
    .expect("escaped nonempty literal name")
}
fn url_ranges(text: &str) -> Vec<Range<usize>> {
    fn visit(node: &Node, text: &str, output: &mut Vec<Range<usize>>) {
        if let Node::Link(_) = node {
            if let Some(position) = node.position() {
                let span = position.start.offset..position.end.offset;
                if !text[span.clone()].starts_with(['[', '<']) {
                    output.push(span);
                }
            }
            return;
        }
        if matches!(
            node,
            Node::Html(_)
                | Node::Code(_)
                | Node::InlineCode(_)
                | Node::Image(_)
                | Node::ImageReference(_)
        ) {
            return;
        }
        if let Some(children) = node.children() {
            for child in children {
                visit(child, text, output);
            }
        }
    }
    let mut output = vec![];
    if (text.contains([':', '@']) || text.to_ascii_lowercase().contains("www."))
        && let Ok(tree) = crate::parse::mdast(text, &markdown::ParseOptions::gfm())
    {
        visit(&tree, text, &mut output);
    }
    output
}

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    let Some(state) = (1..=source.line_count())
        .find_map(|line| document.state("MD044", line).filter(|state| state.enabled))
    else {
        return vec![];
    };
    let Some(configured) = state
        .options
        .get("names")
        .and_then(|value| value.as_array())
    else {
        return vec![];
    };
    let mut names: Vec<_> = configured
        .iter()
        .filter_map(|value| value.as_str())
        .filter(|name| !name.is_empty())
        .collect();
    if names.is_empty() {
        return vec![];
    }
    names.sort_by(|a, b| {
        crate::utf16_len(b)
            .cmp(&crate::utf16_len(a))
            .then_with(|| COLLATOR.compare(a, b))
    });
    let accepted: HashSet<_> = names.iter().copied().collect();
    let patterns: Vec<_> = names.iter().map(|name| (*name, pattern(name))).collect();
    let analysis = content_analysis(
        source,
        parsed,
        ContentFields {
            bare_urls: false,
            segments: true,
        },
    );
    let segments: Vec<_> = analysis
        .segments
        .into_iter()
        .map(|segment| {
            let text = &source.text()[segment.span.clone()];
            let units: Vec<_> = text.encode_utf16().collect();
            let mut offsets = vec![0];
            for (index, ch) in text.char_indices() {
                if ch.len_utf16() == 2 {
                    offsets.push(index);
                }
                offsets.push(index + ch.len_utf8());
            }
            let urls = url_ranges(text);
            (segment, units, offsets, urls)
        })
        .collect();
    let mut used = BTreeMap::new();
    let mut output = vec![];
    for (name, pattern) in patterns {
        for (segment, units, offsets, urls) in &segments {
            let line = source.position(segment.span.start).unwrap().line;
            let Some(state) = document.state("MD044", line).filter(|state| state.enabled) else {
                continue;
            };
            let allowed = |key| {
                state
                    .options
                    .get(key)
                    .and_then(|value| value.as_bool())
                    .unwrap_or(true)
            };
            if matches!(segment.kind, ContentKind::Code) && !allowed("code_blocks")
                || matches!(segment.kind, ContentKind::Html) && !allowed("html_elements")
            {
                continue;
            }
            for matched in pattern.find_from_ucs2(units, 0) {
                let Some(found) = matched.group(2) else {
                    continue;
                };
                let relative = offsets[found.start]..offsets[found.end];
                let span = segment.span.start + relative.start..segment.span.start + relative.end;
                if span.is_empty() || overlaps(&used, &span) {
                    continue;
                }
                let actual = &source.text()[span.clone()];
                let url = urls
                    .iter()
                    .any(|url| url.start < relative.end && relative.start < url.end);
                used.insert(span.start, span.end);
                if !accepted.contains(actual) && !url {
                    output.push(Diagnostic{code:"MD044".into(),rule_id:Some("MD044".into()),severity:state.severity,message:format!("Proper names should have the correct capitalization [Expected: {name}; Actual: {actual}]"),span:Some(span)});
                }
            }
        }
    }
    output.sort_by_key(|diagnostic| diagnostic.span.as_ref().unwrap().start);
    output
}
