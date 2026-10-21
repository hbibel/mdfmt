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
    Container,
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
}

impl RegionKind {
    pub(crate) fn is_protected(self) -> bool {
        !matches!(
            self,
            Self::Root | Self::Container | Self::Paragraph | Self::Text
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
            Node::Blockquote(_) | Node::List(_) | Node::ListItem(_) => RegionKind::Container,
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
