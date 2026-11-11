use super::{ContentKind, ContentSegment};
use crate::{
    Source,
    parse::{label_end_math, references::ImageRecovery},
};
use markdown::mdast::Node;
use regex::Regex;
use std::{ops::Range, sync::LazyLock};

static ENTITY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^&(?:#[0-9]{1,7}|#[xX][0-9a-fA-F]{1,6}|[A-Za-z][A-Za-z0-9]{1,31});").unwrap()
});

struct Collector<'a> {
    source: &'a Source,
    recovery: ImageRecovery,
    output: Vec<ContentSegment>,
}

impl Collector<'_> {
    fn emit(&mut self, span: Range<usize>, kind: ContentKind, clip: &Range<usize>) {
        let start = span.start.max(clip.start);
        let end = span.end.min(clip.end);
        if start < end {
            self.output.push(ContentSegment {
                span: start..end,
                kind,
            });
        }
    }
    fn lines(&mut self, span: Range<usize>, kind: ContentKind, clip: &Range<usize>, quotes: usize) {
        let text = self.source.text();
        let mut start = span.start;
        for line in text[span.clone()].split_inclusive('\n') {
            let end = start + line.trim_end_matches('\n').len();
            let mut content = start;
            if start > span.start {
                for _ in 0..quotes {
                    while matches!(text.as_bytes().get(content), Some(b' ' | b'\t')) {
                        content += 1;
                    }
                    if text.as_bytes().get(content) == Some(&b'>') {
                        content += 1;
                    } else {
                        break;
                    }
                }
            }
            self.emit(content..end, kind, clip);
            start += line.len();
        }
    }
    fn data(&mut self, span: Range<usize>, clip: &Range<usize>, quotes: usize) {
        let text = self.source.text();
        let bytes = text.as_bytes();
        let mut cursor = span.start;
        let mut start = cursor;
        while cursor < span.end {
            let mut skip = 0;
            if bytes[cursor] == b'\\' && bytes.get(cursor + 1).is_some_and(u8::is_ascii_punctuation)
            {
                skip = 2;
            } else if matches!(bytes[cursor], b'*' | b'_' | b'\r' | b'\n')
                || (cursor == span.start
                    && bytes[cursor].is_ascii_punctuation()
                    && bytes[..cursor]
                        .iter()
                        .rev()
                        .take_while(|byte| **byte == b'\\')
                        .count()
                        % 2
                        == 1)
            {
                skip = 1;
            } else if bytes[cursor] == b'&'
                && let Some(entity) = ENTITY.find(&text[cursor..span.end])
            {
                let candidate = entity.as_str();
                let decoded = crate::parse::mdast(candidate, &markdown::ParseOptions::gfm())
                    .ok()
                    .is_some_and(|node| {
                        node.children()
                            .and_then(|children| children.first())
                            .and_then(Node::children)
                            .and_then(|children| children.first())
                            .is_some_and(
                                |node| matches!(node,Node::Text(value) if value.value!=candidate),
                            )
                    });
                if decoded {
                    skip = entity.end();
                }
            }
            if skip > 0 {
                self.emit(start..cursor, ContentKind::Data, clip);
                let newline = matches!(bytes[cursor], b'\r' | b'\n');
                cursor += skip;
                if newline {
                    for _ in 0..quotes {
                        while cursor < span.end && matches!(bytes[cursor], b' ' | b'\t') {
                            cursor += 1;
                        }
                        if bytes.get(cursor) == Some(&b'>') && cursor < span.end {
                            cursor += 1;
                        } else {
                            break;
                        }
                    }
                    while cursor < span.end && matches!(bytes[cursor], b' ' | b'\t') {
                        cursor += 1;
                    }
                }
                start = cursor;
            } else {
                cursor += text[cursor..].chars().next().unwrap().len_utf8();
            }
        }
        self.emit(start..span.end, ContentKind::Data, clip);
    }
    fn code_lines(&mut self, span: Range<usize>, value: &str, clip: &Range<usize>, quotes: usize) {
        let text = self.source.text();
        let mut start = span.start;
        for (raw, value) in text[span.clone()]
            .split_inclusive('\n')
            .zip(value.split('\n'))
        {
            let line = raw.trim_end_matches('\n');
            if !value.is_empty() {
                if let Some(offset) = line.rfind(value) {
                    self.emit(
                        start + offset..start + offset + value.len(),
                        ContentKind::Code,
                        clip,
                    );
                } else {
                    self.lines(start..start + line.len(), ContentKind::Code, clip, quotes);
                }
            }
            start += raw.len();
        }
    }
    fn visit(&mut self, node: &Node, base: usize, quotes: usize, clip: Range<usize>, flow: bool) {
        let quotes = quotes + usize::from(matches!(node, Node::Blockquote(_)));
        let Some(position) = node.position() else {
            return;
        };
        let span = base + position.start.offset..base + position.end.offset;
        let raw = &self.source.text()[span.clone()];
        match node {
            Node::Text(_) => self.data(span, &clip, quotes),
            Node::InlineMath(_)
            | Node::Math(_)
            | Node::Definition(_)
            | Node::FootnoteReference(_) => {}
            Node::InlineCode(_) => {
                let count = raw.bytes().take_while(|byte| *byte == b'`').count();
                if raw.len() >= 2 * count {
                    self.lines(
                        span.start + count..span.end - count,
                        ContentKind::Code,
                        &clip,
                        quotes,
                    );
                }
            }
            Node::Code(code) => {
                let trimmed = raw.trim_start_matches(' ');
                let fenced = raw.len() - trimmed.len() <= 3
                    && (trimmed.starts_with("```") || trimmed.starts_with("~~~"));
                let first = self.source.position(span.start).unwrap().line;
                let last = self.source.position(span.end).unwrap().line;
                let mut start = span.start;
                let mut end = span.end;
                if fenced {
                    start = self
                        .source
                        .line_range(first + 1)
                        .map_or(end, |range| range.start);
                    if last > first {
                        let line = self.source.line_range(last).unwrap();
                        let tail = &self.source.text()[line.clone()];
                        let marker = trimmed.as_bytes()[0] as char;
                        if tail.trim_matches([' ', '\t', '>']).starts_with(
                            &marker.to_string().repeat(
                                trimmed
                                    .bytes()
                                    .take_while(|byte| *byte == marker as u8)
                                    .count(),
                            ),
                        ) {
                            end = line.start;
                        }
                    }
                }
                if start < end {
                    self.code_lines(start..end, &code.value, &clip, quotes);
                }
            }
            Node::Html(_) => {
                if !flow || crate::parse::html::complete_comment(raw) {
                    self.lines(span, ContentKind::Html, &clip, quotes);
                } else {
                    let options = markdown::ParseOptions {
                        constructs: markdown::Constructs {
                            html_flow: false,
                            code_indented: false,
                            math_text: true,
                            math_flow: true,
                            ..markdown::Constructs::gfm()
                        },
                        ..markdown::ParseOptions::gfm()
                    };
                    if let Ok(tree) = crate::parse::mdast(raw, &options) {
                        self.visit(&tree, span.start, quotes, clip, false);
                    } else {
                        self.lines(span, ContentKind::Html, &clip, quotes);
                    }
                }
            }
            Node::Image(_) | Node::ImageReference(_) => {
                if let Some(end) = raw.strip_prefix('!').and_then(label_end_math) {
                    let label = crate::md045::mask_quote_prefixes(&raw[2..end + 1], quotes);
                    if let Some(tree) = self.recovery.paragraph(&label) {
                        self.visit(
                            &tree,
                            span.start,
                            quotes,
                            span.start + 2..span.start + end + 1,
                            false,
                        );
                    }
                }
            }
            Node::Link(_) => {
                if raw.starts_with('[')
                    && let Some(children) = node.children()
                {
                    for child in children {
                        self.visit(child, base, quotes, clip.clone(), false);
                    }
                }
            }
            _ => {
                if let Some(children) = node.children() {
                    for child in children {
                        self.visit(
                            child,
                            base,
                            quotes,
                            clip.clone(),
                            matches!(
                                node,
                                Node::Root(_) | Node::Blockquote(_) | Node::ListItem(_)
                            ),
                        );
                    }
                }
            }
        }
    }
}

pub(super) fn collect(source: &Source, tree: &Node, base: usize) -> Vec<ContentSegment> {
    let mut collector = Collector {
        source,
        recovery: ImageRecovery::new(source, tree, base),
        output: vec![],
    };
    collector.visit(tree, base, 0, 0..source.text().len(), false);
    collector.output.sort_by_key(|segment| segment.span.start);
    collector.output
}
