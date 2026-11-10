mod scanner;
mod styles;

use super::{ParsedDocument, RegionKind, label_end, label_end_math};
use crate::{Source, config::js_space};
use markdown::mdast::Node;
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
    sync::Arc,
};

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

fn native_key(text: &str) -> String {
    text.split([' ', '\t', '\r', '\n'])
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
        .to_uppercase()
}

fn recovery_definitions(
    source: &Source,
    node: &Node,
    base: usize,
    quotes: usize,
    output: &mut BTreeMap<String, String>,
) {
    let quotes = quotes + usize::from(matches!(node, Node::Blockquote(_)));
    if matches!(node, Node::Definition(_) | Node::FootnoteDefinition(_))
        && let Some(position) = node.position()
    {
        let raw = &source.text()[base + position.start.offset..base + position.end.offset];
        if let Some(end) = label_end(raw, false) {
            let logical = crate::md045::mask_quote_prefixes(&raw[1..end], quotes);
            let label = logical
                .split([' ', '\t', '\r', '\n'])
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            output
                .entry(native_key(&label))
                .or_insert_with(|| format!("[{label}]: /\n"));
        }
    }
    if let Some(children) = node.children() {
        for child in children {
            recovery_definitions(source, child, base, quotes, output);
        }
    }
}

fn recovery_context(label: &str, definitions: &BTreeMap<String, String>) -> String {
    let bytes = label.as_bytes();
    let mut cursor = 0;
    let mut start = None;
    let mut selected = BTreeSet::new();
    while cursor < bytes.len() {
        if bytes[cursor] == b'\\' && bytes.get(cursor + 1).is_some_and(u8::is_ascii_punctuation) {
            cursor += 2;
            continue;
        }
        match bytes[cursor] {
            b'[' => start = Some(cursor + 1),
            b']' => {
                if let Some(start) = start.take() {
                    selected.insert(native_key(&label[start..cursor]));
                }
            }
            _ => {}
        }
        cursor += 1;
    }
    selected
        .into_iter()
        .filter_map(|key| definitions.get(&key).map(String::as_str))
        .collect()
}

struct Scopes<'a> {
    source: &'a Source,
    definitions: BTreeMap<String, String>,
    images: std::collections::HashMap<String, Option<Arc<Node>>>,
    logical: std::collections::HashMap<usize, String>,
    output: ReferenceAnalysis,
    fields: ReferenceFields,
}

impl Scopes<'_> {
    fn resolved(&mut self, node: &Node, span: Range<usize>, quotes: usize) {
        let raw = &self.source.text()[span.clone()];
        let image = usize::from(raw.starts_with('!'));
        let raw = &raw[image..];
        let Some(end) = label_end_math(raw) else {
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
        excluded: &mut Vec<scanner::Exclusion>,
        projection: &mut scanner::Projection,
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
            Node::InlineCode(_) | Node::InlineMath(_) | Node::Html(_) => {
                projection.skipped.push(span.clone());
                excluded.push(scanner::Exclusion { span, link: false });
            }
            Node::FootnoteReference(_) => {
                if self.fields.uses {
                    self.resolved(node, span.clone(), quotes);
                }
                projection.skipped.push(span.clone());
                excluded.push(scanner::Exclusion { span, link: false });
            }
            Node::Link(_) | Node::LinkReference(_) => {
                excluded.push(scanner::Exclusion {
                    span: span.clone(),
                    link: self.source.text()[span.clone()].starts_with('['),
                });
                if self.fields.uses && matches!(node, Node::LinkReference(_)) {
                    self.resolved(node, span.clone(), quotes);
                }
                let raw = &self.source.text()[span.clone()];
                if let Some(end) = label_end_math(raw) {
                    projection.skipped.push(span.start..span.start + 1);
                    projection.skipped.push(span.start + end..span.end);
                    projection.extend(self.scope(
                        node,
                        base,
                        span.start + 1..span.start + end,
                        quotes,
                    ));
                } else if raw.starts_with('<') {
                    projection.skipped.push(span);
                } else {
                    projection.literal.push(span);
                }
            }
            Node::Image(_) | Node::ImageReference(_) => {
                excluded.push(scanner::Exclusion {
                    span: span.clone(),
                    link: false,
                });
                if self.fields.uses && matches!(node, Node::ImageReference(_)) {
                    self.resolved(node, span.clone(), quotes);
                }
                let raw = &self.source.text()[span.clone()];
                if let Some(end) = raw.strip_prefix('!').and_then(label_end_math) {
                    projection.skipped.push(span.start..span.start + 2);
                    projection.skipped.push(span.start + end + 1..span.end);
                    let label = crate::md045::mask_quote_prefixes(&raw[2..end + 1], quotes);
                    let possible = label.contains('[')
                        || label.contains('<')
                        || (self.fields.uses
                            && (label.contains(['`', '$', '@'])
                                || label.contains("://")
                                || label.contains("www.")));
                    if !possible {
                        return;
                    }
                    let options = markdown::ParseOptions {
                        constructs: markdown::Constructs {
                            html_flow: false,
                            math_text: true,
                            ..markdown::Constructs::gfm()
                        },
                        ..markdown::ParseOptions::gfm()
                    };
                    let tree = if let Some(tree) = self.images.get(&label) {
                        tree.clone()
                    } else {
                        let definitions = recovery_context(&label, &self.definitions);
                        let candidate = format!("x {label}\n\n{definitions}");
                        #[cfg(test)]
                        tests::record_recovery(candidate.len());
                        let tree = super::mdast(&candidate, &options)
                            .ok()
                            .and_then(|tree| match tree {
                                Node::Root(root) => root.children.into_iter().next(),
                                _ => None,
                            })
                            .map(Arc::new);
                        self.images.insert(label, tree.clone());
                        tree
                    };
                    if let Some(tree) = tree {
                        projection.extend(self.scope(
                            &tree,
                            span.start,
                            span.start + 2..span.start + end + 1,
                            quotes,
                        ));
                    }
                } else {
                    projection.skipped.push(span);
                }
            }
            _ => {
                if let Some(children) = node.children() {
                    for child in children {
                        self.children(child, base, quotes, excluded, projection);
                    }
                }
            }
        }
    }

    fn scope(
        &mut self,
        node: &Node,
        base: usize,
        span: Range<usize>,
        quotes: usize,
    ) -> scanner::Projection {
        let mut excluded = vec![];
        let mut projection = scanner::Projection::default();
        if let Some(children) = node.children() {
            for child in children {
                self.children(child, base, quotes, &mut excluded, &mut projection);
            }
        }
        excluded.sort_by_key(|range| range.span.start);
        projection.skipped.sort_by_key(|range| range.start);
        projection.literal.sort_by_key(|range| range.start);
        if !self.fields.uses {
            return projection;
        }
        let logical = if quotes == 0 {
            self.source.text()
        } else {
            self.logical
                .entry(quotes)
                .or_insert_with(|| crate::md045::mask_quote_prefixes(self.source.text(), quotes))
        };
        self.output.uses.extend(scanner::scan(
            self.source,
            logical,
            span,
            &excluded,
            &projection,
        ));
        projection
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
    let mut definitions = BTreeMap::new();
    recovery_definitions(source, &parsed.tree, base, 0, &mut definitions);
    let mut scopes = Scopes {
        source,
        definitions,
        images: Default::default(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    thread_local! {static WORK:Cell<(usize,usize)>=const{Cell::new((0,0))};}
    pub(super) fn record_recovery(bytes: usize) {
        WORK.with(|work| {
            let (calls, total) = work.get();
            work.set((calls + 1, total + bytes));
        });
    }
    fn take_work() -> (usize, usize) {
        WORK.with(|work| work.replace((0, 0)))
    }

    #[test]
    fn image_recovery_work_is_independent_of_unrelated_definition_payloads() {
        for images in [4, 8] {
            for definitions in [4, 8] {
                for payload in [1024, 4096] {
                    let rest = (0..definitions)
                        .map(|index| format!("[d{index}]: /url \"{}\"\n", "a".repeat(payload)))
                        .collect::<String>();
                    for nested in [false, true] {
                        let image = if nested {
                            "![[text][id]](img)\n\n"
                        } else {
                            "![alt](img)\n\n"
                        };
                        let input = format!(
                            "{}[id]: /url \"{}\"\n{rest}",
                            image.repeat(images),
                            "a".repeat(payload)
                        );
                        let source = Source::new(input);
                        let parsed = super::super::parse(&source).unwrap();
                        take_work();
                        let analysis = reference_analysis(
                            &source,
                            &parsed,
                            ReferenceFields {
                                uses: true,
                                styles: true,
                            },
                        );
                        let (calls, bytes) = take_work();
                        assert_eq!(analysis.definitions.len(), definitions + 1);
                        assert_eq!(analysis.uses.len(), if nested { images } else { 0 });
                        assert_eq!(
                            analysis.styles.len(),
                            if nested { images * 2 } else { images }
                        );
                        if nested {
                            assert!(
                                calls <= 1,
                                "{calls} repeated parses for {images} equal labels"
                            );
                            assert!(
                                bytes < 4096,
                                "{bytes} recovery bytes include unrelated payload"
                            );
                        } else {
                            assert_eq!(calls, 0, "plain alt text needs no nested recovery");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn distinct_image_labels_only_parse_referenced_definition_headers() {
        for definitions in [64, 512] {
            let mut input = (0..8)
                .map(|index| format!("![[text {index}][id]](img)\n\n"))
                .collect::<String>();
            input.push_str("[id]: /url\n");
            for index in 0..definitions {
                input.push_str(&format!(
                    "[unrelated-{index}]: /url \"{}\"\n",
                    "a".repeat(1024)
                ));
            }
            let source = Source::new(input);
            let parsed = super::super::parse(&source).unwrap();
            take_work();
            let analysis = reference_analysis(
                &source,
                &parsed,
                ReferenceFields {
                    uses: true,
                    styles: true,
                },
            );
            let (calls, bytes) = take_work();
            assert_eq!(analysis.uses.len(), 8);
            assert_eq!(analysis.styles.len(), 16);
            assert_eq!(calls, 8);
            assert!(
                bytes < 2048,
                "{bytes} recovery bytes include unrelated headers"
            );
        }
    }
}
