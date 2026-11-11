use super::{ParsedDocument, RegionKind, diagnostic_lists, equivalent, parse};
use crate::{Source, config::DocumentConfig, edit::Edit};
use markdown::mdast::Node;
use std::ops::Range;

pub(crate) struct ListIndentation {
    pub ordered: bool,
    pub span: Range<usize>,
    pub items: Vec<ListItemIndentation>,
}
pub(crate) struct ListItemIndentation {
    pub span: Range<usize>,
    pub marker: Range<usize>,
    pub padding_end: usize,
    pub indent: usize,
    pub marker_end_column: usize,
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
    ) {
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
                });
            }
            if !items.is_empty() {
                output.push(ListIndentation {
                    ordered: list.ordered,
                    span: offsets[base + position.start.offset]
                        ..offsets[base + position.end.offset],
                    items,
                });
            }
        }
        if let Some(children) = node.children() {
            for child in children {
                visit(child, base, source, view, offsets, output);
            }
        }
    }
    let mut output = vec![];
    visit(&parsed.tree, base, source, &view, &offsets, &mut output);
    output
}

pub(crate) fn indentation_edit(
    source: &Source,
    item: &ListItemIndentation,
    expected: usize,
    origin: &str,
    document: &DocumentConfig,
) -> Option<Edit> {
    let delta = expected.abs_diff(item.indent);
    if delta == 0 || delta > source.text().len().saturating_add(4) {
        return None;
    }
    let (span, replacement) = if expected < item.indent {
        let start = item.marker.start.checked_sub(delta)?;
        let raw = source.text().get(start..item.marker.start)?;
        if !raw.bytes().all(|byte| byte == b' ') {
            return None;
        }
        (start..item.marker.start, String::new())
    } else {
        (item.marker.start..item.marker.start, " ".repeat(delta))
    };
    document
        .allows_source(origin, span.clone(), source)
        .then(|| Edit {
            span,
            replacement,
            origin: origin.into(),
        })
}

pub(crate) fn validated_indentation_edits(
    source: &Source,
    parsed: &ParsedDocument,
    edits: Vec<Edit>,
) -> Vec<Edit> {
    let edits: Vec<_> = edits
        .into_iter()
        .filter(|edit| {
            !parsed.regions.iter().any(|region| {
                region.kind.is_protected()
                    && region.span.start < edit.span.end
                    && edit.span.start < region.span.end
            })
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
