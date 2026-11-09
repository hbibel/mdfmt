use crate::{
    Source,
    config::js_space,
    parse::{ContainerPath, ParsedDocument, RegionKind},
};
use markdown::mdast::Node;
use regex::Regex;
use std::{ops::Range, sync::LazyLock};

static PUNCTUATION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[\p{P}\p{S}]$").unwrap());
static ENTITY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^&(?:#[0-9]{1,7}|#[xX][0-9a-fA-F]{1,6}|[A-Za-z][A-Za-z0-9]{1,31});").unwrap()
});

fn reference(text: &str) -> bool {
    crate::parse::mdast(text, &markdown::ParseOptions::gfm())
        .ok()
        .is_some_and(|tree| {
            tree.children()
                .and_then(|children| children.first())
                .and_then(Node::children)
                .and_then(|children| children.first())
                .is_some_and(|node| matches!(node, Node::Text(value) if value.value != text))
        })
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct AttentionPair {
    pub opening: Range<usize>,
    pub closing: Range<usize>,
}

pub(crate) struct BareMarker {
    pub span: Range<usize>,
    pub parent: Range<usize>,
}

pub(crate) struct CodeSpan {
    pub span: Range<usize>,
    pub prefixes: Vec<Range<usize>>,
}

pub(crate) struct LinkLabel {
    pub span: Range<usize>,
}

#[derive(Default, Clone, Copy)]
pub(crate) struct InlineFields {
    pub bare: bool,
    pub code: bool,
    pub labels: bool,
}

#[derive(Default)]
pub(crate) struct InlineAnalysis {
    pub attention: Vec<AttentionPair>,
    pub bare: Vec<BareMarker>,
    pub code: Vec<CodeSpan>,
    pub labels: Vec<LinkLabel>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    Other,
    Punctuation,
    Whitespace,
}

fn classify(ch: Option<char>) -> Class {
    match ch {
        None => Class::Whitespace,
        Some(ch) if js_space(ch) => Class::Whitespace,
        // Pinned micromark receives one UTF-16 code unit at these boundaries.
        Some(ch)
            if ch.len_utf16() == 1
                && (ch.is_ascii_punctuation() || PUNCTUATION.is_match(&ch.to_string())) =>
        {
            Class::Punctuation
        }
        _ => Class::Other,
    }
}

struct Run {
    span: Range<usize>,
    marker: u8,
    open: bool,
    close: bool,
    active: bool,
}

fn resolve(
    text: &str,
    span: Range<usize>,
    excluded: &[Range<usize>],
    prefixes: &[Range<usize>],
    isolated: bool,
    bare: bool,
) -> InlineAnalysis {
    let mut runs = vec![];
    let mut cursor = span.start;
    let bytes = text.as_bytes();
    let mut exclusion = 0;
    let mut boundaries = if bare { excluded.to_vec() } else { vec![] };
    boundaries.extend_from_slice(prefixes);
    while cursor < span.end {
        while exclusion < excluded.len() && excluded[exclusion].end <= cursor {
            exclusion += 1;
        }
        if let Some(range) = excluded
            .get(exclusion)
            .filter(|range| range.start <= cursor && cursor < range.end)
        {
            cursor = range.end.min(span.end);
            continue;
        }
        if bytes[cursor] == b'\\' && bytes.get(cursor + 1).is_some_and(u8::is_ascii_punctuation) {
            if bare {
                boundaries.push(cursor..cursor + 2);
            }
            cursor += 2;
            continue;
        }
        if bare
            && bytes[cursor] == b'&'
            && let Some(entity) = ENTITY.find(&text[cursor..span.end])
            && reference(entity.as_str())
        {
            boundaries.push(cursor..cursor + entity.end());
            cursor += entity.end();
            continue;
        }
        let marker = bytes[cursor];
        if matches!(marker, b'*' | b'_') {
            let start = cursor;
            while cursor < span.end && bytes[cursor] == marker {
                cursor += 1;
            }
            let before = classify(if isolated && start == span.start {
                None
            } else {
                text[..start].chars().next_back()
            });
            let after = classify(if isolated && cursor == span.end {
                None
            } else {
                text[cursor..].chars().next()
            });
            let open =
                after == Class::Other || (after == Class::Punctuation && before != Class::Other);
            let close =
                before == Class::Other || (before == Class::Punctuation && after != Class::Other);
            runs.push(Run {
                span: start..cursor,
                marker,
                open: open && (marker == b'*' || before != Class::Other || !close),
                close: close && (marker == b'*' || after != Class::Other || !open),
                active: true,
            });
        } else {
            cursor += text[cursor..].chars().next().unwrap().len_utf8();
        }
    }
    let mut output = vec![];
    let mut index = 0;
    while index < runs.len() {
        if !runs[index].active || !runs[index].close {
            index += 1;
            continue;
        }
        let opening = (0..index).rev().find(|&open| {
            runs[open].active
                && runs[open].open
                && runs[open].marker == runs[index].marker
                && !((runs[open].close || runs[index].open)
                    && runs[index].span.len() % 3 != 0
                    && (runs[open].span.len() + runs[index].span.len()).is_multiple_of(3))
        });
        let Some(open) = opening else {
            index += 1;
            continue;
        };
        let width = if runs[open].span.len() > 1 && runs[index].span.len() > 1 {
            2
        } else {
            1
        };
        output.push(AttentionPair {
            opening: runs[open].span.end - width..runs[open].span.end,
            closing: runs[index].span.start..runs[index].span.start + width,
        });
        runs[open].span.end -= width;
        runs[index].span.start += width;
        for run in &mut runs[open + 1..index] {
            run.active = false;
        }
        if runs[open].span.is_empty() {
            runs[open].active = false;
        }
        if runs[index].span.is_empty() {
            runs[index].active = false;
            index += 1;
        }
    }
    output.sort_by_key(|pair| pair.opening.start);
    if !bare {
        return InlineAnalysis {
            attention: output,
            ..Default::default()
        };
    }
    boundaries.extend(
        output
            .iter()
            .map(|pair| pair.opening.start..pair.closing.end),
    );
    let starts: std::collections::BTreeSet<_> =
        boundaries.iter().map(|range| range.start).collect();
    let ends: std::collections::BTreeSet<_> = boundaries.iter().map(|range| range.end).collect();
    let mut bare = vec![];
    let mut next = 0;
    let mut parents: Vec<&AttentionPair> = vec![];
    for run in runs.into_iter().filter(|run| !run.span.is_empty()) {
        while next < output.len() && output[next].opening.end <= run.span.start {
            let pair = &output[next];
            while parents
                .last()
                .is_some_and(|parent| parent.closing.start <= pair.opening.start)
            {
                parents.pop();
            }
            parents.push(pair);
            next += 1;
        }
        while parents
            .last()
            .is_some_and(|parent| parent.closing.start < run.span.end)
        {
            parents.pop();
        }
        let parent = parents.last().map_or_else(
            || span.clone(),
            |parent| parent.opening.end..parent.closing.start,
        );
        // Inner attention and labels run data coalescing after delimiter resolution.
        if !isolated || parent != span {
            let before = run.span.start > parent.start
                && !ends.contains(&run.span.start)
                && !text[..run.span.start].ends_with(['\r', '\n']);
            let after = run.span.end < parent.end
                && !starts.contains(&run.span.end)
                && !text[run.span.end..].starts_with(['\r', '\n']);
            if before || after {
                continue;
            }
        }
        bare.push(BareMarker {
            span: run.span,
            parent,
        });
    }
    InlineAnalysis {
        attention: output,
        bare,
        ..Default::default()
    }
}

struct Scopes<'a> {
    source: &'a Source,
    parsed: &'a ParsedDocument,
    indices: std::collections::HashMap<(usize, usize), usize>,
    fields: InlineFields,
    paths: std::collections::HashMap<usize, ContainerPath>,
    prefix_ends: std::collections::HashMap<(Option<usize>, usize), usize>,
    definitions: String,
    logical: std::collections::HashMap<usize, String>,
    output: InlineAnalysis,
}

impl Scopes<'_> {
    fn scope(
        &mut self,
        node: &Node,
        base: usize,
        span: Range<usize>,
        quotes: usize,
        isolated: bool,
        inherited: Option<usize>,
    ) {
        let owner = node
            .position()
            .and_then(|position| {
                self.indices
                    .get(&(base + position.start.offset, base + position.end.offset))
            })
            .copied()
            .or(inherited);
        let mut excluded = vec![];
        if let Some(children) = node.children() {
            for child in children {
                self.children(child, base, quotes, owner, &mut excluded);
            }
        }
        excluded.sort_by_key(|range| range.start);
        let prefixes = if self.fields.bare {
            self.prefixes(span.clone(), owner, true)
        } else {
            vec![]
        };
        let logical = if quotes == 0 {
            self.source.text()
        } else {
            self.logical
                .entry(quotes)
                .or_insert_with(|| crate::md045::mask_quote_prefixes(self.source.text(), quotes))
        };
        let analysis = resolve(
            logical,
            span,
            &excluded,
            &prefixes,
            isolated,
            self.fields.bare,
        );
        self.output.attention.extend(analysis.attention);
        self.output.bare.extend(analysis.bare);
    }

    fn prefixes(
        &mut self,
        span: Range<usize>,
        owner: Option<usize>,
        trim_indent: bool,
    ) -> Vec<Range<usize>> {
        let mut output = vec![];
        let first = self.source.line_number(span.start).unwrap();
        let last = self
            .source
            .line_number(span.end.saturating_sub(1).max(span.start))
            .unwrap();
        for line in first..=last {
            let range = self.source.line_range(line).unwrap();
            let raw = &self.source.text()[range.clone()];
            let mut end = if let Some(&end) = self.prefix_ends.get(&(owner, line)) {
                end
            } else {
                let end = owner.map_or(0, |owner| {
                    let path = self.paths.entry(owner).or_insert_with(|| {
                        ContainerPath::for_region(self.source, self.parsed, owner)
                    });
                    if path.is_empty() {
                        0
                    } else {
                        path.content_start_at(raw, line).unwrap_or(0)
                    }
                });
                self.prefix_ends.insert((owner, line), end);
                end
            };
            while trim_indent && matches!(raw.as_bytes().get(end), Some(b' ' | b'\t')) {
                end += 1;
            }
            if end > 0 {
                output.push(range.start..range.start + end);
            }
        }
        output
    }

    fn children(
        &mut self,
        node: &Node,
        base: usize,
        quotes: usize,
        owner: Option<usize>,
        excluded: &mut Vec<Range<usize>>,
    ) {
        let Some(position) = node.position() else {
            return;
        };
        let span = base + position.start.offset..base + position.end.offset;
        match node {
            Node::InlineCode(_) => {
                if self.fields.code {
                    let prefixes = self.prefixes(span.clone(), owner, false);
                    self.output.code.push(CodeSpan {
                        span: span.clone(),
                        prefixes,
                    });
                }
                excluded.push(span);
            }
            Node::InlineMath(_) | Node::Html(_) | Node::FootnoteReference(_) => excluded.push(span),
            Node::Link(_) | Node::LinkReference(_) => {
                excluded.push(span.clone());
                let raw = &self.source.text()[span.clone()];
                if let Some(end) = crate::parse::label_end(raw, true) {
                    if self.fields.labels {
                        self.output.labels.push(LinkLabel {
                            span: span.start..span.start + end + 1,
                        });
                    }
                    self.scope(
                        node,
                        base,
                        span.start + 1..span.start + end,
                        quotes,
                        false,
                        owner,
                    );
                }
            }
            Node::Image(_) | Node::ImageReference(_) => {
                excluded.push(span.clone());
                let raw = &self.source.text()[span.clone()];
                if let Some(end) = raw
                    .strip_prefix('!')
                    .and_then(|label| crate::parse::label_end(label, true))
                {
                    let label = crate::md045::mask_quote_prefixes(&raw[2..end + 1], quotes);
                    let options = markdown::ParseOptions {
                        constructs: markdown::Constructs {
                            html_flow: false,
                            math_text: true,
                            ..markdown::Constructs::gfm()
                        },
                        ..markdown::ParseOptions::gfm()
                    };
                    if let Ok(tree) =
                        crate::parse::mdast(&format!("x {label}\n\n{}", self.definitions), &options)
                        && let Some(first) = tree.children().and_then(|children| children.first())
                    {
                        self.scope(
                            first,
                            span.start,
                            span.start + 2..span.start + end + 1,
                            quotes,
                            false,
                            owner,
                        );
                    }
                }
            }
            _ => {
                if let Some(children) = node.children() {
                    for child in children {
                        self.children(child, base, quotes, owner, excluded);
                    }
                }
            }
        }
    }

    fn blocks(&mut self, node: &Node, base: usize, quotes: usize) {
        let quotes = quotes + usize::from(matches!(node, Node::Blockquote(_)));
        if matches!(
            node,
            Node::Paragraph(_) | Node::Heading(_) | Node::TableCell(_)
        ) && let Some(position) = node.position()
        {
            let span = if matches!(node, Node::TableCell(_)) {
                // Native cell positions include edge pipes; inline chunks do not.
                let Some(children) = node.children() else {
                    return;
                };
                let Some((first, last)) = children
                    .first()
                    .and_then(Node::position)
                    .zip(children.last().and_then(Node::position))
                else {
                    return;
                };
                base + first.start.offset..base + last.end.offset
            } else {
                base + position.start.offset..base + position.end.offset
            };
            self.scope(node, base, span, quotes, true, None);
        } else if !matches!(
            node,
            Node::Code(_) | Node::Math(_) | Node::Html(_) | Node::Definition(_)
        ) && let Some(children) = node.children()
        {
            for child in children {
                self.blocks(child, base, quotes);
            }
        }
    }
}

pub(crate) fn inline_analysis(
    source: &Source,
    parsed: &ParsedDocument,
    fields: InlineFields,
) -> InlineAnalysis {
    let base = parsed
        .regions
        .iter()
        .find(|r| r.kind == RegionKind::Root && r.parent.is_some())
        .map_or(0, |r| r.span.start);
    let definitions = parsed
        .regions
        .iter()
        .filter(|r| r.kind == RegionKind::Definition)
        .map(|r| &source.text()[r.span.clone()])
        .collect::<Vec<_>>()
        .join("\n");
    let mut scopes = Scopes {
        source,
        parsed,
        indices: if fields.bare || fields.code {
            parsed
                .regions
                .iter()
                .enumerate()
                .map(|(index, region)| ((region.span.start, region.span.end), index))
                .collect()
        } else {
            Default::default()
        },
        fields,
        paths: Default::default(),
        prefix_ends: Default::default(),
        definitions,
        logical: std::collections::HashMap::new(),
        output: InlineAnalysis::default(),
    };
    scopes.blocks(&parsed.tree, base, 0);
    scopes
        .output
        .attention
        .sort_by_key(|pair| pair.opening.start);
    scopes.output.bare.sort_by_key(|marker| marker.span.start);
    scopes.output.code.sort_by_key(|code| code.span.start);
    scopes.output.labels.sort_by_key(|label| label.span.start);
    scopes.output
}
