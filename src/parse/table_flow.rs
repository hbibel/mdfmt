use super::{ContainerPath, ParsedDocument, RegionKind, parse_with_math};
use crate::Source;
use markdown::mdast::Node;
use std::collections::BTreeSet;

pub(super) fn candidates(source: &Source, parsed: &ParsedDocument) -> BTreeSet<usize> {
    let mut positions = BTreeSet::new();
    if !source.text().contains('|') {
        return positions;
    }
    for (index, region) in parsed.regions.iter().enumerate() {
        if region.kind != RegionKind::Paragraph {
            continue;
        }
        let path = ContainerPath::for_region(source, parsed, index);
        for line in crate::md013::covered_lines(source, region.span.clone()) {
            let line = line + 1;
            let range = source.line_range(line).unwrap();
            let text = &source.text()[range.clone()];
            let Some(prefix) = path.content_start_at(text, line) else {
                continue;
            };
            if matches!(
                text.as_bytes().get(prefix),
                Some(b'#' | b'$' | b'`' | b'~' | b'*' | b'_' | b'<' | b'e' | b'i' | b'{')
            ) {
                positions.insert(range.start + prefix);
            }
        }
    }
    positions
}

pub(super) fn recover(source: &Source, parsed: ParsedDocument) -> Result<ParsedDocument, String> {
    let positions = candidates(source, &parsed);
    if positions.is_empty() {
        return Ok(parsed);
    }
    let base = parsed
        .regions
        .iter()
        .find(|region| region.kind == RegionKind::Root && region.parent.is_some())
        .map_or(0, |region| region.span.start);
    let mut text = String::with_capacity(source.text().len() + positions.len());
    let mut offsets = vec![0];
    for (index, ch) in source.text().char_indices() {
        if positions.contains(&index) {
            // A synthetic prefix mapped to zero source width bypasses native flow fast paths,
            // while keeping the original inline delimiters available to parse.
            text.push('x');
            offsets.push(index);
        }
        text.push(ch);
        offsets.extend(index + 1..=index + ch.len_utf8());
    }
    let diagnostic = source.mapped(text, &offsets);
    let mut recovered = parse_with_math(&diagnostic, true)?;
    let diagnostic_base = recovered
        .regions
        .iter()
        .find(|region| region.kind == RegionKind::Root && region.parent.is_some())
        .map_or(0, |region| region.span.start);
    fn remap(
        node: &mut Node,
        base: usize,
        diagnostic_base: usize,
        offsets: &[usize],
        source: &Source,
        diagnostic: &Source,
    ) {
        if let Some(position) = node.position_mut() {
            for point in [&mut position.start, &mut position.end] {
                let old = diagnostic_base + point.offset;
                let mapped = offsets[old];
                let old_line = diagnostic
                    .line_range(diagnostic.position(old).unwrap().line)
                    .unwrap()
                    .start;
                let new_line = source
                    .line_range(source.position(mapped).unwrap().line)
                    .unwrap()
                    .start;
                point.column = point
                    .column
                    .saturating_sub((old - old_line) - (mapped - new_line));
                point.offset = mapped - base;
            }
        }
        if let Some(children) = node.children_mut() {
            for child in children {
                remap(child, base, diagnostic_base, offsets, source, diagnostic);
            }
        }
    }
    remap(
        &mut recovered.tree,
        base,
        diagnostic_base,
        &offsets,
        source,
        &diagnostic,
    );
    for region in &mut recovered.regions {
        region.span = offsets[region.span.start]..offsets[region.span.end];
    }
    Ok(recovered)
}
