mod scanner;
mod styles;

use super::{ParsedDocument, RegionKind, label_end};
use crate::{Source, config::js_space};
use markdown::mdast::Node;
use std::ops::Range;

pub(crate) struct ReferenceDefinition {
    pub key: String,
    pub span: Range<usize>,
}
pub(crate) struct ReferenceUse {
    pub key: String,
    pub span: Range<usize>,
    pub shortcut: bool,
}
#[derive(Clone, Copy)]
pub(crate) enum LinkKind {
    Autolink,
    Inline,
    Full,
    Collapsed,
    Shortcut,
}
pub(crate) struct LinkStyle {
    pub span: Range<usize>,
    pub kind: LinkKind,
    pub label: Option<Range<usize>>,
    pub destination: Option<Range<usize>>,
    pub title: Option<Range<usize>>,
}
pub(crate) struct ReferenceFields {
    pub uses: bool,
    pub styles: bool,
}
#[derive(Default)]
pub(crate) struct ReferenceAnalysis {
    pub definitions: Vec<ReferenceDefinition>,
    pub uses: Vec<ReferenceUse>,
    pub styles: Vec<LinkStyle>,
}

fn key(text: &str) -> String {
    text.to_lowercase()
        .split(js_space)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

struct Scopes<'a> {
    source: &'a Source,
    definitions: String,
    logical: std::collections::HashMap<usize, String>,
    output: ReferenceAnalysis,
    fields: ReferenceFields,
}

impl Scopes<'_> {
    fn resolved(&mut self, node: &Node, span: Range<usize>, quotes: usize) {
        let raw = &self.source.text()[span.clone()];
        let image = usize::from(raw.starts_with('!'));
        let raw = &raw[image..];
        let Some(end) = label_end(raw, true) else {
            return;
        };
        let label = &raw[1..end];
        let suffix = &raw[end + 1..];
        let shortcut = !suffix.starts_with('[');
        let reference = if let Some(end) = suffix
            .strip_prefix('[')
            .and_then(|_| label_end(suffix, true))
        {
            let reference = &suffix[1..end];
            if reference.is_empty() {
                label
            } else {
                reference
            }
        } else {
            label
        };
        let reference = crate::md045::mask_quote_prefixes(reference, quotes);
        self.output.uses.push(ReferenceUse {
            key: key(&reference),
            span,
            shortcut: shortcut || matches!(node, Node::FootnoteReference(_)),
        });
    }

    fn children(
        &mut self,
        node: &Node,
        base: usize,
        quotes: usize,
        excluded: &mut Vec<Range<usize>>,
    ) {
        let Some(position) = node.position() else {
            return;
        };
        let span = base + position.start.offset..base + position.end.offset;
        if self.fields.styles
            && matches!(
                node,
                Node::Link(_) | Node::LinkReference(_) | Node::Image(_) | Node::ImageReference(_)
            )
        {
            let logical = if quotes == 0 {
                self.source.text()
            } else {
                self.logical.entry(quotes).or_insert_with(|| {
                    crate::md045::mask_quote_prefixes(self.source.text(), quotes)
                })
            };
            if let Some(style) = styles::record(node, self.source.text(), logical, span.clone()) {
                self.output.styles.push(style);
            }
        }
        match node {
            Node::InlineCode(_) | Node::InlineMath(_) | Node::Html(_) => excluded.push(span),
            Node::FootnoteReference(_) => {
                if self.fields.uses {
                    self.resolved(node, span.clone(), quotes);
                }
                excluded.push(span);
            }
            Node::Link(_) | Node::LinkReference(_) => {
                excluded.push(span.clone());
                if self.fields.uses && matches!(node, Node::LinkReference(_)) {
                    self.resolved(node, span.clone(), quotes);
                }
                let raw = &self.source.text()[span.clone()];
                if let Some(end) = label_end(raw, true) {
                    self.scope(node, base, span.start + 1..span.start + end, quotes);
                }
            }
            Node::Image(_) | Node::ImageReference(_) => {
                excluded.push(span.clone());
                if self.fields.uses && matches!(node, Node::ImageReference(_)) {
                    self.resolved(node, span.clone(), quotes);
                }
                let raw = &self.source.text()[span.clone()];
                if let Some(end) = raw.strip_prefix('!').and_then(|raw| label_end(raw, true)) {
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
                        super::mdast(&format!("x {label}\n\n{}", self.definitions), &options)
                        && let Some(first) = tree.children().and_then(|children| children.first())
                    {
                        self.scope(
                            first,
                            span.start,
                            span.start + 2..span.start + end + 1,
                            quotes,
                        );
                    }
                }
            }
            _ => {
                if let Some(children) = node.children() {
                    for child in children {
                        self.children(child, base, quotes, excluded);
                    }
                }
            }
        }
    }

    fn scope(&mut self, node: &Node, base: usize, span: Range<usize>, quotes: usize) {
        let mut excluded = vec![];
        if let Some(children) = node.children() {
            for child in children {
                self.children(child, base, quotes, &mut excluded);
            }
        }
        excluded.sort_by_key(|range| range.start);
        if !self.fields.uses {
            return;
        }
        let logical = if quotes == 0 {
            self.source.text()
        } else {
            self.logical
                .entry(quotes)
                .or_insert_with(|| crate::md045::mask_quote_prefixes(self.source.text(), quotes))
        };
        self.output
            .uses
            .extend(scanner::scan(self.source, logical, span, &excluded));
    }

    fn blocks(&mut self, node: &Node, base: usize, quotes: usize) {
        let quotes = quotes + usize::from(matches!(node, Node::Blockquote(_)));
        if matches!(node, Node::Definition(_) | Node::FootnoteDefinition(_))
            && let Some(position) = node.position()
        {
            let span = base + position.start.offset..base + position.end.offset;
            let raw = &self.source.text()[span.clone()];
            if let Some(end) = label_end(raw, true) {
                self.output.definitions.push(ReferenceDefinition {
                    key: key(&raw[1..end]),
                    span,
                });
            }
        }
        if matches!(
            node,
            Node::Paragraph(_) | Node::Heading(_) | Node::TableCell(_)
        ) && let Some(position) = node.position()
        {
            self.scope(
                node,
                base,
                base + position.start.offset..base + position.end.offset,
                quotes,
            );
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

pub(crate) fn reference_analysis(
    source: &Source,
    parsed: &ParsedDocument,
    fields: ReferenceFields,
) -> ReferenceAnalysis {
    let base = parsed
        .regions
        .iter()
        .find(|region| region.kind == RegionKind::Root && region.parent.is_some())
        .map_or(0, |region| region.span.start);
    let definitions = parsed
        .regions
        .iter()
        .filter(|region| region.kind == RegionKind::Definition)
        .map(|region| &source.text()[region.span.clone()])
        .collect::<Vec<_>>()
        .join("\n");
    let mut scopes = Scopes {
        source,
        definitions,
        logical: Default::default(),
        output: Default::default(),
        fields,
    };
    scopes.blocks(&parsed.tree, base, 0);
    scopes
        .output
        .definitions
        .sort_by_key(|definition| definition.span.start);
    scopes
        .output
        .uses
        .sort_by_key(|reference| reference.span.start);
    scopes.output.styles.sort_by_key(|style| style.span.start);
    scopes.output
}
