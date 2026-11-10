use super::{ParsedDocument, RegionKind, expanded_tabs, parse_flow_diagnostics};
use crate::{Source, heading_spaces::literal_exclusions};
use std::{collections::HashMap, ops::Range};

pub(crate) struct DiagnosticCodeBlock {
    pub span: Range<usize>,
    pub value: String,
    pub fenced: bool,
}

pub(crate) fn diagnostic_code_blocks(
    source: &Source,
    parsed: &ParsedDocument,
) -> Vec<DiagnosticCodeBlock> {
    let expanded = source.text().contains('\t').then(|| expanded_tabs(source));
    let recovered = match expanded.as_ref() {
        Some((view, _)) => match parse_flow_diagnostics(view) {
            Ok(parsed) => Some(parsed),
            Err(_) => return vec![],
        },
        None => None,
    };
    let originals: HashMap<_, _> = if expanded.is_some() {
        literal_exclusions(source, parsed, true)
            .code
            .into_iter()
            .map(|block| (block.span, block.value))
            .collect()
    } else {
        HashMap::new()
    };
    let view = expanded.as_ref().map_or(source, |(view, _)| view);
    let parsed = recovered.as_ref().unwrap_or(parsed);
    let mut siblings = HashMap::new();
    let mut owners = HashMap::new();
    for (index, region) in parsed.regions.iter().enumerate() {
        let previous = siblings.insert(region.parent, index);
        if region.kind == RegionKind::Code {
            owners.insert(region.span.clone(), (region.parent, index, previous));
        }
    }
    let mut blocks = literal_exclusions(view, parsed, true).code;
    blocks.sort_by_key(|block| block.span.start);
    let mut previous = None;
    let mut output: Vec<DiagnosticCodeBlock> = vec![];
    for block in blocks {
        let raw = &view.text()[block.span.clone()];
        let trimmed = raw.trim_start_matches(' ');
        let marker = trimmed.as_bytes().first().copied();
        let fenced = raw.len() - trimmed.len() <= 3
            && matches!(marker, Some(b'`' | b'~'))
            && trimmed
                .bytes()
                .take_while(|byte| Some(*byte) == marker)
                .count()
                >= 3;
        let owner = owners.get(&block.span).copied();
        let continuation = !fenced
            && previous.is_some_and(|(end, previous_owner)| {
                owner.zip(previous_owner).is_some_and(
                    |((parent, _, sibling), (previous_parent, index, _))| {
                        parent == previous_parent && sibling == Some(index)
                    },
                ) && end <= block.span.start
                    && view.position(end).unwrap().line + 1
                        < view.position(block.span.start).unwrap().line
                    && view.text()[end..block.span.start]
                        .bytes()
                        .all(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n' | b'>'))
            });
        previous = (!fenced).then_some((block.span.end, owner));
        let span = expanded
            .as_ref()
            .map_or(block.span.clone(), |(_, offsets)| {
                offsets[block.span.start]..offsets[block.span.end]
            });
        let value = originals.get(&span).cloned().unwrap_or(block.value);
        if continuation && let Some(last) = output.last_mut() {
            let lines = source.position(span.start).unwrap().line
                - source.position(last.span.end).unwrap().line;
            last.value.extend(std::iter::repeat_n('\n', lines));
            last.value.push_str(&value);
            last.span.end = span.end;
        } else {
            output.push(DiagnosticCodeBlock {
                span,
                value,
                fenced,
            });
        }
    }
    output
}
