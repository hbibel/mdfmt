use super::{FormatterFailure, adapters::Adapter};
use crate::{
    Source,
    config::DocumentConfig,
    edit::Edit,
    parse::{self, ContainerPath, ParsedDocument, RegionKind},
};
use markdown::mdast::{Code, Node};
use std::{
    collections::{HashMap, HashSet},
    ops::Range,
};

pub(super) struct FenceBody {
    pub ordinal: usize,
    pub span: Range<usize>,
    pub language: String,
    pub adapter: Adapter,
    pub logical: String,
    pub prefix: String,
    pub width: usize,
    pub newline: &'static str,
    pub skip: Option<FormatterFailure>,
}

fn codes(parsed: &ParsedDocument) -> Vec<(usize, &Code, Range<usize>)> {
    let base = parsed
        .regions
        .iter()
        .find(|region| region.kind == RegionKind::Root && region.parent.is_some())
        .map_or(0, |region| region.span.start);
    fn visit<'a>(node: &'a Node, base: usize, output: &mut Vec<(usize, &'a Code, Range<usize>)>) {
        if let Node::Code(code) = node
            && let Some(position) = node.position()
        {
            output.push((
                output.len(),
                code,
                base + position.start.offset..base + position.end.offset,
            ));
        }
        if let Some(children) = node.children() {
            for child in children {
                visit(child, base, output);
            }
        }
    }
    let mut output = vec![];
    visit(&parsed.tree, base, &mut output);
    output
}

pub(super) fn anchors(parsed: &ParsedDocument) -> Vec<usize> {
    codes(parsed)
        .into_iter()
        .map(|(_, _, span)| span.start)
        .collect()
}

pub(super) fn bodies(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<FenceBody> {
    if parse::has_unmodeled_syntax(source, parsed) {
        return vec![];
    }
    let regions: HashMap<_, _> = parsed
        .regions
        .iter()
        .enumerate()
        .filter(|(_, region)| region.kind == RegionKind::Code)
        .map(|(index, region)| ((region.span.start, region.span.end), index))
        .collect();
    let mut result = vec![];
    for (ordinal, code, span) in codes(parsed) {
        let Some(language) = code.lang.as_deref().filter(|_| code.meta.is_none()) else {
            continue;
        };
        let Some(adapter) = Adapter::for_language(language) else {
            continue;
        };
        let Some(index) = regions.get(&(span.start, span.end)).copied() else {
            continue;
        };
        let path = ContainerPath::for_region(source, parsed, index);
        let first = source.position(span.start).unwrap().line;
        let last = crate::md013::covered_lines(source, span.clone()).end;
        let Some(body) = extract(source, &path, first, last) else {
            continue;
        };
        let Some(state) = document.state("MD013", first).filter(|state| state.enabled) else {
            continue;
        };
        if state
            .options
            .get("code_blocks")
            .and_then(|value| value.as_bool())
            == Some(false)
            || !document.allows_source("MD013", span.clone(), source)
        {
            continue;
        }
        if body.logical.strip_suffix('\n').unwrap_or(&body.logical)
            != code.value.replace("\r\n", "\n").replace('\r', "\n")
            || body.logical.trim().is_empty()
        {
            continue;
        }
        let base = state
            .options
            .get("line_length")
            .and_then(|value| value.as_u64())
            .unwrap_or(80);
        let width = state
            .options
            .get("code_block_line_length")
            .and_then(|value| value.as_u64())
            .unwrap_or(base);
        let width = usize::try_from(width)
            .unwrap_or(usize::MAX)
            .saturating_sub(crate::utf16_len(&body.prefix));
        let skip = (body.logical.len() > 1024 * 1024)
            .then(|| FormatterFailure::Skipped("code body exceeds 1 MiB".into()));
        result.push(FenceBody {
            ordinal,
            span: body.span,
            language: language.into(),
            adapter,
            logical: body.logical,
            prefix: body.prefix,
            width,
            newline: crate::md047::preferred_newline(source.text()),
            skip,
        });
    }
    result
}

struct Extracted {
    span: Range<usize>,
    logical: String,
    prefix: String,
}
fn extract(source: &Source, path: &ContainerPath, first: usize, last: usize) -> Option<Extracted> {
    if last <= first {
        return None;
    }
    let opening = source.line_range(first)?;
    let raw = &source.text()[opening.clone()];
    let start = path.content_start_at(raw, first)?;
    if raw[..start].contains('\t') {
        return None;
    }
    let indent = raw[start..]
        .bytes()
        .take_while(|byte| *byte == b' ')
        .count();
    if indent > 3 {
        return None;
    }
    let fence = &raw[start + indent..];
    let marker = *fence.as_bytes().first()?;
    if !matches!(marker, b'`' | b'~') {
        return None;
    }
    let length = fence.bytes().take_while(|byte| *byte == marker).count();
    if length < 3 {
        return None;
    }
    let closing = source.line_range(last)?;
    let raw = &source.text()[closing.clone()];
    let end = path.content_start_at(raw, last)?;
    if raw[..end].contains('\t') {
        return None;
    }
    let tail = raw[end..].trim_start_matches(' ');
    if raw[end..].len() - tail.len() > 3 {
        return None;
    }
    let count = tail.bytes().take_while(|byte| *byte == marker).count();
    if count < length || !tail[count..].trim_matches([' ', '\t']).is_empty() {
        return None;
    }
    let prefix = format!("{}{}", &raw[..end], " ".repeat(indent));
    let mut logical = String::new();
    for line in first + 1..last {
        let raw = &source.text()[source.line_range(line)?];
        let end = path.content_start_at(raw, line)?;
        if raw[..end].contains('\t') {
            return None;
        }
        let padding = raw[end..]
            .bytes()
            .take_while(|byte| *byte == b' ')
            .count()
            .min(indent);
        logical.push_str(&raw[end + padding..]);
        logical.push('\n');
    }
    Some(Extracted {
        span: source.line_range(first + 1)?.start..closing.start,
        logical,
        prefix,
    })
}

pub(super) fn validated_output(
    source: &Source,
    parsed: &ParsedDocument,
    edits: &[Edit],
    selected: &HashSet<usize>,
) -> Option<String> {
    crate::edit::validate_ranges(source, edits).ok()?;
    let mut output = String::new();
    let mut cursor = 0;
    for edit in edits {
        if edit.span.start < cursor {
            return None;
        }
        output.push_str(source.text().get(cursor..edit.span.start)?);
        output.push_str(&edit.replacement);
        cursor = edit.span.end;
    }
    output.push_str(source.text().get(cursor..)?);
    let candidate = Source::new(output.clone());
    let reparsed = parse::parse(&candidate).ok()?;
    fn normalize(node: &mut Node, selected: &HashSet<usize>, ordinal: &mut usize) {
        node.position_set(None);
        if let Node::Code(code) = node {
            if selected.contains(ordinal) {
                code.value.clear();
            }
            *ordinal += 1;
        }
        if let Some(children) = node.children_mut() {
            for child in children {
                normalize(child, selected, ordinal);
            }
        }
    }
    let mut before = parsed.tree.clone();
    let mut after = reparsed.tree;
    normalize(&mut before, selected, &mut 0);
    normalize(&mut after, selected, &mut 0);
    (before == after).then_some(output)
}
