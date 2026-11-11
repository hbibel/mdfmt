use super::{ParsedDocument, RegionKind, diagnostic_lists, equivalent, parse};
use crate::{Source, config::DocumentConfig, edit::Edit};
use markdown::mdast::Node;
use std::ops::Range;

pub(crate) struct ListIndentation {
    pub ordered: bool,
    pub span: Range<usize>,
    pub items: Vec<ListItemIndentation>,
    pub ul_depth: Option<usize>,
    pub footnote_base: usize,
}
pub(crate) struct ListItemIndentation {
    pub span: Range<usize>,
    pub marker: Range<usize>,
    pub padding_end: usize,
    pub indent: usize,
    pub marker_end_column: usize,
    pub quote_adjustment: usize,
}

#[derive(Clone, Copy)]
struct Scope {
    ul_depth: Option<usize>,
    footnote_base: usize,
    quotes: usize,
}

fn quote_adjustment(source: &Source, start: usize, quotes: usize) -> usize {
    if quotes == 0 {
        return 0;
    }
    let line = source.position(start).unwrap().line;
    let range = source.line_range(line).unwrap();
    let prefix = &source.text()[range.start..start];
    let mut cursor = 0;
    let mut found = 0;
    let mut end = 0;
    while cursor < prefix.len() && found < quotes {
        if prefix.as_bytes()[cursor] == b'['
            && let Some(close) = super::label_end(&prefix[cursor..], false)
        {
            cursor += close + 1;
            continue;
        }
        if prefix.as_bytes()[cursor] == b'>' {
            found += 1;
            end = cursor
                + 1
                + usize::from(matches!(
                    prefix.as_bytes().get(cursor + 1),
                    Some(b' ' | b'\t')
                ));
        }
        cursor += prefix[cursor..].chars().next().unwrap().len_utf8();
    }
    if end == 0 {
        0
    } else {
        crate::utf16_len(&prefix[..end]) - usize::from(line == 1 && prefix.starts_with('\u{feff}'))
    }
}

pub(crate) fn list_indentation(source: &Source) -> Vec<ListIndentation> {
    let Some((view, parsed, offsets)) = diagnostic_lists(source) else {
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
        view: &Source,
        offsets: &[usize],
        output: &mut Vec<ListIndentation>,
        mut scope: Scope,
    ) {
        if matches!(node, Node::Blockquote(_)) {
            scope.ul_depth = Some(0);
            scope.quotes += 1;
        }
        if matches!(node, Node::FootnoteDefinition(_)) {
            scope.footnote_base = 4;
        }
        if let Node::List(list) = node
            && let Some(position) = node.position()
        {
            let mut items = vec![];
            for item in &list.children {
                let Some(position) = item.position() else {
                    continue;
                };
                let raw = &view.text()[base + position.start.offset..base + position.end.offset];
                let trimmed = raw.trim_start_matches(' ');
                let start = base + position.start.offset + raw.len() - trimmed.len();
                let digits = trimmed.bytes().take_while(u8::is_ascii_digit).count();
                let width = if list.ordered
                    && digits > 0
                    && matches!(trimmed.as_bytes().get(digits), Some(b'.' | b')'))
                {
                    digits + 1
                } else if !list.ordered
                    && matches!(trimmed.as_bytes().first(), Some(b'-' | b'+' | b'*'))
                {
                    1
                } else {
                    continue;
                };
                let spaces = trimmed[width..]
                    .bytes()
                    .take_while(|byte| *byte == b' ')
                    .count();
                let remainder = &trimmed[width + spaces..];
                let padding = if remainder.is_empty() || remainder.starts_with(['\r', '\n']) {
                    0
                } else if spaces <= 4 {
                    spaces
                } else {
                    1
                };
                let end = start + width + padding;
                let padding_end =
                    offsets[end] + usize::from(end > 0 && offsets[end] == offsets[end - 1]);
                let marker = offsets[start]..offsets[start + width];
                let position_original = source.position(marker.start).unwrap();
                let indent = position_original.column_utf16
                    - 1
                    - usize::from(
                        position_original.line == 1 && source.text().starts_with('\u{feff}'),
                    );
                items.push(ListItemIndentation {
                    span: offsets[base + position.start.offset]
                        ..offsets[base + position.end.offset],
                    marker,
                    padding_end,
                    indent,
                    marker_end_column: indent + width,
                    quote_adjustment: quote_adjustment(source, offsets[start], scope.quotes),
                });
            }
            if !items.is_empty() {
                output.push(ListIndentation {
                    ordered: list.ordered,
                    span: offsets[base + position.start.offset]
                        ..offsets[base + position.end.offset],
                    items,
                    ul_depth: if list.ordered { None } else { scope.ul_depth },
                    footnote_base: scope.footnote_base,
                });
            }
        }
        if let Node::List(list) = node {
            scope.ul_depth = if list.ordered {
                None
            } else {
                scope.ul_depth.map(|depth| depth + 1)
            };
        }
        if let Some(children) = node.children() {
            for child in children {
                visit(child, base, source, view, offsets, output, scope);
            }
        }
    }
    let mut output = vec![];
    visit(
        &parsed.tree,
        base,
        source,
        &view,
        &offsets,
        &mut output,
        Scope {
            ul_depth: Some(0),
            footnote_base: 0,
            quotes: 0,
        },
    );
    output
}

pub(crate) struct IndentationRepair {
    span: Range<usize>,
    padding: usize,
    origin: &'static str,
}

pub(crate) fn indentation_repair(
    source: &Source,
    item: &ListItemIndentation,
    expected: usize,
    origin: &'static str,
    document: &DocumentConfig,
) -> Option<IndentationRepair> {
    let delta = expected.abs_diff(item.indent);
    if delta == 0 || delta > source.text().len().saturating_add(4) {
        return None;
    }
    let (span, padding) = if expected < item.indent {
        let start = item.marker.start.checked_sub(delta)?;
        let raw = source.text().get(start..item.marker.start)?;
        if !raw.bytes().all(|byte| byte == b' ') {
            return None;
        }
        (start..item.marker.start, 0)
    } else {
        (item.marker.start..item.marker.start, delta)
    };
    document
        .allows_source(origin, span.clone(), source)
        .then_some(IndentationRepair {
            span,
            padding,
            origin,
        })
}

pub(crate) fn validated_indentation_edits(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
    repairs: Vec<IndentationRepair>,
) -> Vec<Edit> {
    let repairs: Vec<_> = repairs
        .into_iter()
        .filter(|edit| {
            !parsed.regions.iter().any(|region| {
                region.kind.is_protected()
                    && region.span.start < edit.span.end
                    && edit.span.start < region.span.end
            })
        })
        .collect();
    if repairs.is_empty() {
        return vec![];
    }
    // Bound the complete batch before allocating any replacement strings.
    let padding = repairs
        .iter()
        .try_fold(0usize, |sum, repair| sum.checked_add(repair.padding));
    if padding.is_none_or(|padding| padding > source.text().len().saturating_add(4)) {
        return vec![];
    }
    let edits: Vec<_> = repairs
        .into_iter()
        .map(|repair| Edit {
            span: repair.span,
            replacement: " ".repeat(repair.padding),
            origin: repair.origin.into(),
        })
        .collect();
    let Ok(output) = crate::edit::apply_authorized_edits(source, parsed, edits.clone(), &edits)
    else {
        return vec![];
    };
    let candidate = Source::new(output);
    let Ok(reparsed) = parse(&candidate) else {
        return vec![];
    };
    if !equivalent(source, parsed, &candidate, &reparsed) {
        return vec![];
    }
    for rule in ["MD023", "MD027", "MD030"] {
        if !(1..=source.line_count()).any(|line| {
            document
                .state(rule, line)
                .is_some_and(|state| state.enabled)
        }) {
            continue;
        }
        // Only line permissions survive the candidate's shifted byte ranges.
        // Deletions can also create padding findings at the collapsed boundary.
        let opposing = match rule {
            "MD023" => crate::md023::diagnose(&candidate, &reparsed, document),
            "MD027" => crate::md027::diagnose(&candidate, &reparsed, document),
            _ => crate::md030::diagnose(&candidate, &reparsed, document),
        };
        let mut original_cursor = 0;
        let mut candidate_cursor = 0;
        for edit in &edits {
            candidate_cursor += edit.span.start - original_cursor;
            let inserted = candidate_cursor..candidate_cursor + edit.replacement.len();
            if opposing
                .iter()
                .filter_map(|finding| finding.span.as_ref())
                .any(|span| {
                    if inserted.is_empty() {
                        span.start <= inserted.start && inserted.start <= span.end
                    } else {
                        span.start < inserted.end && inserted.start < span.end
                    }
                })
            {
                return vec![];
            }
            candidate_cursor = inserted.end;
            original_cursor = edit.span.end;
        }
    }
    edits
}
