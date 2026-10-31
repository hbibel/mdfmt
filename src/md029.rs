use crate::{
    Diagnostic, Severity, Source,
    config::DocumentConfig,
    edit::Edit,
    parse::{ParsedDocument, RegionKind, equivalent, parse},
};
use markdown::mdast::Node;
use std::ops::Range;

struct Prefix {
    span: Range<usize>,
    digits: Range<usize>,
    number: usize,
    end_column: usize,
}

fn lists(source: &Source) -> Vec<Vec<Prefix>> {
    let (mut expanded, offsets) = crate::parse::expanded_tabs(source);
    let Ok(mut parsed) = parse(&expanded) else {
        return vec![];
    };
    loop {
        let invalid = crate::parse::invalid_ordered_interruptions(&expanded, &parsed);
        if invalid.is_empty() {
            break;
        }
        let mut masked = expanded.text().to_owned();
        // Equal-width masks restore paragraph interruption without moving source spans.
        for marker in invalid {
            masked.replace_range(marker..marker + 1, "x");
        }
        expanded = Source::new(masked);
        let Ok(next) = parse(&expanded) else {
            return vec![];
        };
        parsed = next;
    }
    let base = parsed
        .regions
        .iter()
        .find(|region| region.kind == RegionKind::Root && region.parent.is_some())
        .map_or(0, |region| region.span.start);
    fn visit(
        node: &Node,
        base: usize,
        source: &Source,
        expanded: &Source,
        offsets: &[usize],
        output: &mut Vec<Vec<Prefix>>,
    ) {
        if let Node::List(list) = node
            && list.ordered
        {
            let mut prefixes = vec![];
            for item in &list.children {
                let Some(position) = item.position() else {
                    continue;
                };
                let raw =
                    &expanded.text()[base + position.start.offset..base + position.end.offset];
                let trimmed = raw.trim_start_matches(' ');
                let start = base + position.start.offset + raw.len() - trimmed.len();
                let digits = trimmed.bytes().take_while(u8::is_ascii_digit).count();
                if digits == 0 || !matches!(trimmed.as_bytes().get(digits), Some(b'.' | b')')) {
                    continue;
                }
                let width = digits + 1;
                let spaces = trimmed[width..]
                    .bytes()
                    .take_while(|byte| *byte == b' ')
                    .count();
                let empty = trimmed[width + spaces..].is_empty()
                    || trimmed[width + spaces..].starts_with(['\r', '\n']);
                let padding = if empty {
                    0
                } else if (1..=4).contains(&spaces) {
                    spaces
                } else {
                    1
                };
                let end = start + width + padding;
                let end = offsets[end] + usize::from(end > 0 && offsets[end] == offsets[end - 1]);
                let span = offsets[start]..end;
                let digits = offsets[start]..offsets[start + digits];
                let number = source.text()[digits.clone()].parse().unwrap();
                let position = source.position(end).unwrap();
                let end_column = position.column_utf16
                    - usize::from(position.line == 1 && source.text().starts_with('\u{feff}'));
                prefixes.push(Prefix {
                    span,
                    digits,
                    number,
                    end_column,
                });
            }
            if !prefixes.is_empty() {
                output.push(prefixes);
            }
        }
        if let Some(children) = node.children() {
            for child in children {
                visit(child, base, source, expanded, offsets, output);
            }
        }
    }
    let mut output = vec![];
    visit(&parsed.tree, base, source, &expanded, &offsets, &mut output);
    output
}

struct Finding {
    span: Range<usize>,
    digits: Range<usize>,
    expected: usize,
    actual: usize,
    style: &'static str,
    replacement: String,
    first: bool,
    severity: Severity,
}

fn findings(source: &Source, document: &DocumentConfig) -> Vec<Finding> {
    let mut output = vec![];
    for prefixes in lists(source) {
        let first = &prefixes[0];
        let line = source.position(first.span.start).unwrap().line;
        let Some(settings) = document.state("MD029", line) else {
            continue;
        };
        let incrementing = prefixes.len() > 1 && (prefixes[1].number != 1 || first.number == 0);
        let style = match settings
            .options
            .get("style")
            .and_then(|value| value.as_str())
            .unwrap_or("one_or_ordered")
        {
            "one" => "one",
            "zero" => "zero",
            "ordered" => "ordered",
            _ => {
                if incrementing {
                    "ordered"
                } else {
                    "one"
                }
            }
        };
        let mut expected =
            if style == "zero" || (style == "ordered" && prefixes.len() > 1 && first.number == 0) {
                0
            } else {
                1
            };
        let right_aligned = prefixes
            .iter()
            .all(|prefix| prefix.end_column == first.end_column);
        let zero_prefixed =
            prefixes.len() > 1 && source.text()[first.digits.clone()].starts_with('0');
        for (index, prefix) in prefixes.into_iter().enumerate() {
            let line = source.position(prefix.span.start).unwrap().line;
            if let Some(state) = document.state("MD029", line).filter(|state| state.enabled)
                && prefix.number != expected
            {
                let text = expected.to_string();
                let padding = if right_aligned {
                    prefix.digits.len().saturating_sub(text.len())
                } else {
                    0
                };
                let replacement = format!(
                    "{}{text}",
                    if zero_prefixed { "0" } else { " " }.repeat(padding)
                );
                output.push(Finding {
                    span: prefix.span,
                    digits: prefix.digits,
                    expected,
                    actual: prefix.number,
                    style: match style {
                        "one" => "1/1/1",
                        "zero" => "0/0/0",
                        _ => "1/2/3",
                    },
                    replacement,
                    first: index == 0,
                    severity: state.severity,
                });
            }
            if style == "ordered" {
                expected += 1;
            }
        }
    }
    output.sort_by_key(|finding| finding.span.start);
    output
}

pub(crate) fn propose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Edit> {
    if crate::parse::has_unmodeled_syntax(source, parsed) {
        return vec![];
    }
    let edits: Vec<_> = findings(source, document)
        .into_iter()
        .filter(|finding| {
            !finding.first
                && document.allows_source("MD029", finding.digits.clone(), source)
                && !parsed.regions.iter().any(|region| {
                    region.kind.is_protected()
                        && region.span.start < finding.digits.end
                        && finding.digits.start < region.span.end
                })
        })
        .map(|finding| Edit {
            span: finding.digits,
            replacement: finding.replacement,
            origin: "MD029".into(),
        })
        .collect();
    if edits.is_empty() {
        return edits;
    }
    let safe = crate::edit::apply_authorized_edits(source, parsed, edits.clone(), &edits)
        .ok()
        .and_then(|output| {
            let candidate = Source::new(output);
            let reparsed = parse(&candidate).ok()?;
            Some(equivalent(source, parsed, &candidate, &reparsed))
        })
        .unwrap_or(false);
    if safe { edits } else { vec![] }
}

pub(crate) fn diagnose(
    source: &Source,
    _parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    findings(source, document)
        .into_iter()
        .map(|finding| Diagnostic {
            code: "MD029".into(),
            rule_id: Some("MD029".into()),
            severity: finding.severity,
            message: format!(
                "Ordered list item prefix [Expected: {}; Actual: {}; Style: {}]",
                finding.expected, finding.actual, finding.style
            ),
            span: Some(finding.span),
        })
        .collect()
}
