#[cfg(test)]
mod tests {
    use super::*;
    use crate::Source;

    #[test]
    fn source_regions_match_fixtures() {
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/parser/cases.json")).unwrap();
        for case in cases.as_array().unwrap() {
            let source = Source::new(case["input"].as_str().unwrap().into());
            let parsed = parse(&source).unwrap();
            for expected in case["expect"].as_array().unwrap() {
                assert!(
                    parsed
                        .regions
                        .iter()
                        .any(|r| r.kind.name() == expected[0].as_str().unwrap()
                            && &source.text()[r.span.clone()] == expected[1].as_str().unwrap()),
                    "{} {expected}",
                    case["id"]
                );
            }
            for (index, region) in parsed.regions.iter().enumerate() {
                assert!(source.text().get(region.span.clone()).is_some());
                if let Some(parent) = region.parent {
                    assert!(parent < index);
                    assert!(parsed.regions[parent].span.start <= region.span.start);
                    assert!(parsed.regions[parent].span.end >= region.span.end);
                }
            }
        }
    }

    #[test]
    fn front_matter_matches_upstream_runtime_observations() {
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/parser/cases.json")).unwrap();
        let observations: serde_json::Value =
            serde_json::from_str(include_str!("../tools/parser-oracle/observations.json")).unwrap();
        for (case, observed) in cases
            .as_array()
            .unwrap()
            .iter()
            .zip(observations.as_array().unwrap())
        {
            let source = Source::new(case["input"].as_str().unwrap().into());
            let parsed = parse(&source).unwrap();
            let front = parsed
                .regions
                .iter()
                .find(|r| r.kind == RegionKind::FrontMatter);
            let actual = front
                .map(|r| {
                    source.text()[r.span.clone()]
                        .replace("\r\n", "\n")
                        .replace('\r', "\n")
                })
                .unwrap_or_default();
            let expected: Vec<_> = observed["frontMatterLines"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap())
                .collect();
            // Compare content separately from trailing blank-line count, checked by exact fixture spans.
            assert_eq!(
                actual.trim_end(),
                expected.join("\n").trim_end(),
                "{}",
                case["id"]
            );
        }
    }
}
use crate::Source;
use markdown::mdast::Node;
use std::{ops::Range, sync::LazyLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RegionKind {
    Root,
    Blockquote,
    List,
    ListItem,
    Paragraph,
    Text,
    Link,
    Image,
    InlineCode,
    Table,
    Code,
    Html,
    Definition,
    FrontMatter,
    Protected,
    Heading,
    ThematicBreak,
    Break,
}

impl RegionKind {
    pub(crate) fn is_protected(self) -> bool {
        !matches!(
            self,
            Self::Root
                | Self::Blockquote
                | Self::List
                | Self::ListItem
                | Self::Paragraph
                | Self::Text
        )
    }

    #[cfg(test)]
    fn name(self) -> &'static str {
        match self {
            Self::Paragraph => "paragraph",
            Self::Link => "link",
            Self::Image => "image",
            Self::InlineCode => "inlineCode",
            Self::Table => "table",
            Self::Code => "code",
            Self::Html => "html",
            Self::Definition => "definition",
            Self::FrontMatter => "frontMatter",
            _ => "other",
        }
    }
}

#[derive(Debug)]
pub(crate) struct Region {
    pub span: Range<usize>,
    pub kind: RegionKind,
    pub parent: Option<usize>,
}

#[derive(Debug)]
pub(crate) struct ParsedDocument {
    pub regions: Vec<Region>,
    pub tree: Node,
}

pub(crate) fn expanded_tabs(source: &Source) -> (Source, Vec<usize>) {
    let mut expanded = String::new();
    let mut offsets = vec![0];
    let mut width = 0;
    for (index, ch) in source.text().char_indices() {
        match ch {
            '\t' => {
                let spaces = 4 - width % 4;
                expanded.extend(std::iter::repeat_n(' ', spaces));
                offsets.extend(std::iter::repeat_n(index, spaces - 1));
                offsets.push(index + 1);
                width += spaces;
            }
            _ => {
                expanded.push(ch);
                offsets.extend(index + 1..=index + ch.len_utf8());
                if matches!(ch, '\r' | '\n') {
                    width = 0;
                } else if index != 0 || ch != '\u{feff}' {
                    width += ch.len_utf16();
                }
            }
        }
    }
    (Source::new(expanded), offsets)
}

pub(crate) fn has_unmodeled_syntax(source: &Source, parsed: &ParsedDocument) -> bool {
    has_uncertain_container_prefixes(source, parsed)
        || parsed.regions.iter().any(|region| {
            let literal = &source.text()[region.span.clone()];
            (region.kind == RegionKind::Paragraph
                && (literal.contains('$')
                    || literal
                        .as_bytes()
                        .windows(2)
                        .any(|pair| pair[0] == b':' && pair[1].is_ascii_alphabetic())))
                || (region.kind == RegionKind::Html && !literal.starts_with("<!--"))
        })
}

pub(crate) fn has_uncertain_container_prefixes(source: &Source, parsed: &ParsedDocument) -> bool {
    parsed.regions.iter().any(|region| {
        if !matches!(
            region.kind,
            RegionKind::List
                | RegionKind::ListItem
                | RegionKind::Blockquote
                | RegionKind::Paragraph
                | RegionKind::Heading
                | RegionKind::Code
                | RegionKind::ThematicBreak
        ) {
            return false;
        }
        let mut container = matches!(region.kind, RegionKind::ListItem | RegionKind::Blockquote);
        let mut parent = region.parent;
        while let Some(index) = parent {
            let ancestor = &parsed.regions[index];
            container |= matches!(ancestor.kind, RegionKind::ListItem | RegionKind::Blockquote);
            parent = ancestor.parent;
        }
        if !container {
            return false;
        }
        let line_start = source
            .line_range(source.position(region.span.start).unwrap().line)
            .unwrap()
            .start;
        if source.text()[line_start..region.span.start].contains('\t') {
            return true;
        }
        if region.kind == RegionKind::ListItem {
            let raw = source.text()[region.span.clone()].trim_start_matches([' ', '\t']);
            let marker = region.span.end - raw.len();
            let digits = raw.bytes().take_while(u8::is_ascii_digit).count();
            let width = if digits > 0 { digits + 1 } else { 1 };
            let separator = raw[width..]
                .bytes()
                .take_while(|byte| matches!(byte, b' ' | b'\t'))
                .count();
            return source.text()[line_start..marker + width + separator].contains('\t');
        }
        false
    })
}

pub(crate) fn equivalent(
    source: &Source,
    parsed: &ParsedDocument,
    candidate: &Source,
    reparsed: &ParsedDocument,
) -> bool {
    equivalent_with_style_changes(source, parsed, candidate, reparsed, false)
}

pub(crate) fn equivalent_with_thematic_styles(
    source: &Source,
    parsed: &ParsedDocument,
    candidate: &Source,
    reparsed: &ParsedDocument,
) -> bool {
    equivalent_with_style_changes(source, parsed, candidate, reparsed, true)
}

fn equivalent_with_style_changes(
    source: &Source,
    parsed: &ParsedDocument,
    candidate: &Source,
    reparsed: &ParsedDocument,
    thematic_styles: bool,
) -> bool {
    fn normalize(node: &mut Node) {
        node.position_set(None);
        if let Node::Text(text) = node {
            text.value = text
                .value
                .split_ascii_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
        }
        if let Some(children) = node.children_mut() {
            for child in children {
                normalize(child);
            }
        }
    }
    let mut before = parsed.tree.clone();
    let mut after = reparsed.tree.clone();
    normalize(&mut before);
    normalize(&mut after);
    let protected = |source: &Source, parsed: &ParsedDocument| {
        parsed
            .regions
            .iter()
            .filter(|r| {
                r.kind.is_protected() && !(thematic_styles && r.kind == RegionKind::ThematicBreak)
            })
            .map(|r| (r.kind, source.text()[r.span.clone()].to_owned()))
            .collect::<Vec<_>>()
    };
    before == after && protected(source, parsed) == protected(candidate, reparsed)
}

// Match upstream's built-in pattern after newline normalization, not the parser's
// metadata extension (which accepts a different set of closing delimiters).
static FRONT_MATTER: LazyLock<regex::Regex> = LazyLock::new(|| {
    // Upstream executes the translated pattern with RE2JS's ASCII whitespace,
    // not ECMAScript's Unicode whitespace (or Rust regex's Unicode \s).
    let space = r"[\t\n\x0c\r ]";
    let horizontal = r"[\t\x0c ]";
    regex::Regex::new(&format!(r"(?m)((^---{horizontal}*$[\s\S]+?^---{space}*)|(^\+\+\+{horizontal}*$[\s\S]+?^(\+\+\+|\.\.\.){space}*)|(^\{{{horizontal}*$[\s\S]+?^\}}{space}*))(\n|$)")).unwrap()
});

pub(crate) fn parse(source: &Source) -> Result<ParsedDocument, String> {
    let text = source.text();
    let bom = usize::from(text.starts_with('\u{feff}')) * 3;
    let mut normalized = String::new();
    let mut offsets = vec![bom];
    let mut chars = text[bom..].char_indices().peekable();
    while let Some((index, ch)) = chars.next() {
        let mut end = bom + index + ch.len_utf8();
        if ch == '\r' {
            if chars.peek().is_some_and(|(_, c)| *c == '\n') {
                chars.next();
                end += 1;
            }
            normalized.push('\n');
            offsets.push(end);
        } else {
            normalized.push(ch);
            for offset in (bom + index + 1)..=end {
                offsets.push(offset);
            }
        }
    }
    let front_end = FRONT_MATTER
        .find(&normalized)
        .filter(|m| m.start() == 0)
        .map_or(bom, |m| offsets[m.end()]);
    let root = markdown::to_mdast(&text[front_end..], &markdown::ParseOptions::gfm())
        .map_err(|error| error.to_string())?;
    let mut parsed = ParsedDocument {
        tree: root.clone(),
        regions: vec![Region {
            span: 0..text.len(),
            kind: RegionKind::Root,
            parent: None,
        }],
    };
    if front_end > bom {
        parsed.regions.push(Region {
            span: bom..front_end,
            kind: RegionKind::FrontMatter,
            parent: Some(0),
        });
    }
    fn visit(node: &Node, base: usize, parent: usize, parsed: &mut ParsedDocument) {
        let kind = match node {
            Node::Root(_) => RegionKind::Root,
            Node::Paragraph(_) => RegionKind::Paragraph,
            Node::Text(_) => RegionKind::Text,
            Node::Blockquote(_) => RegionKind::Blockquote,
            Node::List(_) => RegionKind::List,
            Node::ListItem(_) => RegionKind::ListItem,
            Node::Heading(_) => RegionKind::Heading,
            Node::ThematicBreak(_) => RegionKind::ThematicBreak,
            Node::Break(_) => RegionKind::Break,
            Node::Link(_) | Node::LinkReference(_) => RegionKind::Link,
            Node::Image(_) | Node::ImageReference(_) => RegionKind::Image,
            Node::InlineCode(_) => RegionKind::InlineCode,
            Node::Table(_) => RegionKind::Table,
            Node::Code(_) => RegionKind::Code,
            Node::Html(_) => RegionKind::Html,
            Node::Definition(_) => RegionKind::Definition,
            _ => RegionKind::Protected,
        };
        let current = if let Some(pos) = node.position() {
            let index = parsed.regions.len();
            parsed.regions.push(Region {
                span: (base + pos.start.offset)..(base + pos.end.offset),
                kind,
                parent: Some(parent),
            });
            index
        } else {
            parent
        };
        if let Some(children) = node.children() {
            for child in children {
                visit(child, base, current, parsed);
            }
        }
    }
    visit(&root, front_end, 0, &mut parsed);
    if text.contains(['\u{2028}', '\u{2029}']) {
        parsed.regions.push(Region {
            span: 0..text.len(),
            kind: RegionKind::Protected,
            parent: Some(0),
        });
    }
    Ok(parsed)
}
