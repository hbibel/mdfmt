use super::{ParsedDocument, RegionKind, expanded_tabs, parse_flow_diagnostics};
use crate::Source;
use markdown::mdast::Node;
use std::collections::BTreeSet;

pub(crate) fn diagnostic_parse(source: &Source) -> Option<ParsedDocument> {
    let (expanded, expanded_offsets) = expanded_tabs(source);
    let parsed = parse_flow_diagnostics(&expanded).ok()?;
    let mut boundaries = BTreeSet::new();
    if expanded.text().contains('<') {
        let excluded = crate::heading_spaces::literal_exclusions(&expanded, &parsed, true);
        for region in &parsed.regions {
            if region.kind == RegionKind::Html
                && region.parent.is_some_and(|p| {
                    matches!(
                        parsed.regions[p].kind,
                        RegionKind::Root | RegionKind::ListItem | RegionKind::Blockquote
                    )
                })
            {
                for line in crate::md013::covered_lines(&expanded, region.span.clone()).skip(1) {
                    if !excluded.flow[line] && excluded.flow[line - 1] {
                        boundaries.insert(expanded.line_range(line + 1)?.start);
                    }
                }
            }
        }
    }
    let mut text = String::new();
    let mut offsets = vec![0];
    for (index, ch) in expanded.text().char_indices() {
        // Only diagnostic flow is restarted at previously recovered HTML boundaries.
        // Synthetic blank bytes map to zero original width; no edit uses this tree.
        if boundaries.contains(&index) {
            text.push('\n');
            offsets.push(expanded_offsets[index]);
        }
        text.push(ch);
        offsets.extend((index + 1..=index + ch.len_utf8()).map(|i| expanded_offsets[i]));
    }
    let diagnostic = source.mapped(text, &offsets);
    let mut parsed = if boundaries.is_empty() {
        parsed
    } else {
        parse_flow_diagnostics(&diagnostic).ok()?
    };
    let base = parsed
        .regions
        .iter()
        .find(|r| r.kind == RegionKind::Root && r.parent.is_some())
        .map_or(0, |r| r.span.start);
    let original_base = offsets[base];
    fn remap(
        node: &mut Node,
        base: usize,
        original_base: usize,
        offsets: &[usize],
        source: &Source,
    ) {
        if let Some(position) = node.position_mut() {
            for point in [&mut position.start, &mut position.end] {
                let byte = offsets[base + point.offset];
                let original = source.position(byte).unwrap();
                point.offset = byte - original_base;
                point.line = original.line;
                point.column = original.column_utf16;
            }
        }
        if let Some(children) = node.children_mut() {
            for child in children {
                remap(child, base, original_base, offsets, source);
            }
        }
    }
    remap(&mut parsed.tree, base, original_base, &offsets, source);
    for region in &mut parsed.regions {
        region.span = offsets[region.span.start]..offsets[region.span.end];
    }
    Some(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_boundary_recovery_keeps_multiline_setext_heading_history() {
        let source = Source::new("- item\n<span>\nA\nB\n===\n### C\n".into());
        let parsed = diagnostic_parse(&source).unwrap();
        let headings = crate::md024::headings_in(&source, &parsed, None);
        assert_eq!(
            headings
                .iter()
                .map(|h| (h.level, h.text.as_str()))
                .collect::<Vec<_>>(),
            vec![(1, "A B"), (3, "C")],
            "{:#?}",
            parsed.tree
        );
    }
}
