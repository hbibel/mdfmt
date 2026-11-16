use crate::{
    Diagnostic, Severity, Source,
    config::DocumentConfig,
    edit::Edit,
    parse::{ParsedDocument, RegionKind, equivalent, parse},
};
use markdown::mdast::Node;
use std::ops::Range;

struct Finding {
    span: Range<usize>,
    padding: Range<usize>,
    actual: usize,
    expected: u64,
    severity: Severity,
    reflow_sensitive: bool,
}

fn findings(source: &Source, document: &DocumentConfig) -> Vec<Finding> {
    let Some((expanded, parsed, offsets)) = crate::parse::diagnostic_list_parse(source) else {
        return vec![];
    };
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
        document: &DocumentConfig,
        output: &mut Vec<Finding>,
    ) {
        if let Node::List(list) = node
            && let Some(position) = &list.position
        {
            let raw = &expanded.text()[base + position.start.offset..base + position.end.offset];
            let end =
                base + position.start.offset + raw.trim_end_matches([' ', '\t', '\r', '\n']).len();
            let end_line = expanded.position(end).unwrap().line;
            let single = end_line - position.start.line + 1 == list.children.len();
            let first_line = source
                .position(offsets[base + position.start.offset])
                .unwrap()
                .line;
            if let Some(settings) = document.state("MD030", first_line) {
                let option = match (list.ordered, single) {
                    (false, true) => "ul_single",
                    (true, true) => "ol_single",
                    (false, false) => "ul_multi",
                    (true, false) => "ol_multi",
                };
                let expected = settings
                    .options
                    .get(option)
                    .and_then(|value| value.as_u64())
                    .unwrap_or(1);
                let single_option = if list.ordered {
                    "ol_single"
                } else {
                    "ul_single"
                };
                let multi_option = if list.ordered { "ol_multi" } else { "ul_multi" };
                let differing = settings
                    .options
                    .get(single_option)
                    .and_then(|value| value.as_u64())
                    .unwrap_or(1)
                    != settings
                        .options
                        .get(multi_option)
                        .and_then(|value| value.as_u64())
                        .unwrap_or(1);
                let end_line = source
                    .position(offsets[base + position.end.offset])
                    .unwrap()
                    .line;
                let reflow_sensitive = differing
                    && (first_line..=end_line).any(|line| {
                        document
                            .state("MD013", line)
                            .is_some_and(|state| state.enabled)
                    });
                for item in &list.children {
                    let Some(position) = item.position() else {
                        continue;
                    };
                    let raw =
                        &expanded.text()[base + position.start.offset..base + position.end.offset];
                    let trimmed = raw.trim_start_matches(' ');
                    let start = base + position.start.offset + raw.len() - trimmed.len();
                    let digits = trimmed.bytes().take_while(u8::is_ascii_digit).count();
                    let marker = if digits > 0
                        && matches!(trimmed.as_bytes().get(digits), Some(b'.' | b')'))
                    {
                        digits + 1
                    } else if matches!(trimmed.as_bytes().first(), Some(b'-' | b'+' | b'*')) {
                        1
                    } else {
                        continue;
                    };
                    let spaces = trimmed[marker..]
                        .bytes()
                        .take_while(|byte| *byte == b' ')
                        .count();
                    let remainder = &trimmed[marker + spaces..];
                    if spaces == 0 || remainder.is_empty() || remainder.starts_with(['\r', '\n']) {
                        continue;
                    }
                    let width = if spaces <= 4 { spaces } else { 1 };
                    let end = start + marker + width;
                    let end =
                        offsets[end] + usize::from(end > 0 && offsets[end] == offsets[end - 1]);
                    let padding = offsets[start + marker]..end;
                    let actual = crate::utf16_len(&source.text()[padding.clone()]);
                    let line = source.position(offsets[start]).unwrap().line;
                    if actual as u64 != expected
                        && let Some(state) =
                            document.state("MD030", line).filter(|state| state.enabled)
                    {
                        output.push(Finding {
                            span: offsets[start]..end,
                            padding,
                            actual,
                            expected,
                            severity: state.severity,
                            reflow_sensitive,
                        });
                    }
                }
            }
        }
        if let Some(children) = node.children() {
            for child in children {
                visit(child, base, source, expanded, offsets, document, output);
            }
        }
    }
    let mut output = vec![];
    visit(
        &parsed.tree,
        base,
        source,
        &expanded,
        &offsets,
        document,
        &mut output,
    );
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
            !finding.reflow_sensitive
                && finding.expected <= 4
                && document.allows_source("MD030", finding.padding.clone(), source)
                && !parsed.regions.iter().any(|region| {
                    region.kind.is_protected()
                        && region.span.start < finding.padding.end
                        && finding.padding.start < region.span.end
                })
        })
        .map(|finding| Edit {
            span: finding.padding,
            replacement: " ".repeat(finding.expected as usize),
            origin: "MD030".into(),
        })
        .collect();
    if edits.is_empty() {
        return edits;
    }
    let safe = crate::edit::apply_authorized_edits(source, parsed, edits.clone(), &edits)
        .ok()
        .and_then(|output| {
            let candidate = source.derive(output);
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
            code: "MD030".into(),
            rule_id: Some("MD030".into()),
            severity: finding.severity,
            message: format!(
                "Spaces after list markers [Expected: {}; Actual: {}]",
                finding.expected, finding.actual
            ),
            span: Some(finding.span),
        })
        .collect()
}
