use super::{
    LinkKind, ParsedDocument, ReferenceFields, RegionKind, diagnostic_parse, html_tokens,
    reference_analysis,
};
use crate::{Source, config::js_space};
use markdown::mdast::Node;
use regex::Regex;
use std::{
    collections::{HashMap, HashSet},
    ops::Range,
    sync::LazyLock,
};

pub(crate) struct FragmentSnapshot {
    pub targets: Vec<String>,
    pub destinations: Vec<FragmentDestination>,
}
pub(crate) struct FragmentDestination {
    pub span: Range<usize>,
    pub destination: Range<usize>,
    pub value: String,
    pub inline: bool,
    pub docfx_tab: bool,
}

static ENTITY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^&(?:#[0-9]{1,7}|#[xX][0-9a-fA-F]{1,6}|[A-Za-z][A-Za-z0-9]{1,31});").unwrap()
});
static SLUG_DENY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[^\p{L}\p{M}\p{N}\p{Pc}\- ]").unwrap());
static ANCHOR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{(#[a-z0-9]+(?:[-_]+[a-z0-9]+)*)\}").unwrap());
static ID: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r#"(?iu){}id{}*={}*['"]?([^'"{}>]*)"#,
        crate::config::JS_SPACE,
        crate::config::JS_SPACE,
        crate::config::JS_SPACE,
        crate::config::JS_SPACE
    ))
    .unwrap()
});
static NAME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r#"(?iu){}name{}*={}*['"]?([^'"{}>]*)"#,
        crate::config::JS_SPACE,
        crate::config::JS_SPACE,
        crate::config::JS_SPACE,
        crate::config::JS_SPACE
    ))
    .unwrap()
});

pub(crate) fn encode_component(text: &str) -> String {
    let mut output = String::new();
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte) {
            output.push(byte as char);
        } else {
            use std::fmt::Write;
            write!(&mut output, "%{byte:02X}").unwrap();
        }
    }
    output
}

struct Projection {
    entities: HashMap<String, bool>,
}
impl Projection {
    fn string(&mut self, raw: &str, quotes: usize, entities: bool) -> String {
        let mut output = String::new();
        let mut cursor = 0;
        let bytes = raw.as_bytes();
        while cursor < raw.len() {
            if matches!(bytes[cursor], b'\r' | b'\n') {
                if bytes[cursor] == b'\r' && bytes.get(cursor + 1) == Some(&b'\n') {
                    cursor += 1;
                }
                cursor += 1;
                for _ in 0..quotes {
                    while matches!(bytes.get(cursor), Some(b' ' | b'\t')) {
                        cursor += 1;
                    }
                    if bytes.get(cursor) == Some(&b'>') {
                        cursor += 1;
                    } else {
                        break;
                    }
                }
                while matches!(bytes.get(cursor), Some(b' ' | b'\t')) {
                    cursor += 1;
                }
                continue;
            }
            if entities
                && bytes[cursor] == b'\\'
                && bytes.get(cursor + 1).is_some_and(u8::is_ascii_punctuation)
            {
                output.push(bytes[cursor + 1] as char);
                cursor += 2;
                continue;
            }
            if entities
                && bytes[cursor] == b'&'
                && let Some(entity) = ENTITY.find(&raw[cursor..])
            {
                let candidate = entity.as_str();
                let valid = *self.entities.entry(candidate.into()).or_insert_with(|| {
                    super::mdast(candidate, &markdown::ParseOptions::gfm())
                        .ok()
                        .is_some_and(|node| {
                            node.children()
                                .and_then(|children| children.first())
                                .and_then(Node::children)
                                .and_then(|children| children.first())
                                .is_some_and(
                                    |node| matches!(node,Node::Text(text)if text.value!=candidate),
                                )
                        })
                });
                if valid {
                    cursor += entity.end();
                    continue;
                }
            }
            let ch = raw[cursor..].chars().next().unwrap();
            output.push(ch);
            cursor += ch.len_utf8();
        }
        output
    }
    fn heading(
        &mut self,
        node: &Node,
        source: &Source,
        base: usize,
        quotes: usize,
        output: &mut String,
    ) {
        let Some(position) = node.position() else {
            return;
        };
        let raw = &source.text()[base + position.start.offset..base + position.end.offset];
        match node {
            Node::Image(_) | Node::ImageReference(_) | Node::Html(_) => {}
            Node::Text(_) | Node::FootnoteReference(_) => {
                output.push_str(&self.string(raw, quotes, true))
            }
            Node::InlineCode(_) => {
                let ticks = raw.bytes().take_while(|byte| *byte == b'`').count();
                if ticks * 2 <= raw.len() {
                    let mut value = &raw[ticks..raw.len() - ticks];
                    if value.starts_with([' ', '\r', '\n'])
                        && value.ends_with([' ', '\r', '\n'])
                        && value.chars().any(|ch| !matches!(ch, ' ' | '\r' | '\n'))
                    {
                        value = &value[1..value.len() - 1];
                    }
                    output.push_str(&self.string(value, quotes, false));
                }
            }
            Node::InlineMath(math) => output.push_str(&self.string(&math.value, 0, false)),
            _ => {
                if let Some(children) = node.children() {
                    for child in children {
                        self.heading(child, source, base, quotes, output);
                    }
                }
            }
        }
    }
}

struct Targets {
    ordered: Vec<String>,
    counts: HashMap<String, usize>,
}
impl Targets {
    fn put(&mut self, target: String, count: usize) {
        if !self.counts.contains_key(&target) {
            self.ordered.push(target.clone());
        }
        self.counts.insert(target, count);
    }
}

pub(crate) fn fragment_snapshot(source: &Source, parsed: &ParsedDocument) -> FragmentSnapshot {
    let (view, offsets) = super::html::diagnostic_view(source, parsed);
    let Some(recovered) = diagnostic_parse(&view) else {
        return FragmentSnapshot {
            targets: vec!["#top".into()],
            destinations: vec![],
        };
    };
    let base = recovered
        .regions
        .iter()
        .find(|region| region.kind == RegionKind::Root && region.parent.is_some())
        .map_or(0, |region| region.span.start);
    let docfx: HashSet<_> = crate::md024::headings_in(&view, &recovered, None)
        .into_iter()
        .filter(|heading| heading.docfx_tab)
        .map(|heading| heading.start)
        .collect();
    let mut projection = Projection {
        entities: HashMap::new(),
    };
    let mut targets = Targets {
        ordered: vec!["#top".into()],
        counts: HashMap::from([("#top".into(), 0)]),
    };
    let mut docfx_ranges = vec![];
    fn headings(
        node: &Node,
        source: &Source,
        base: usize,
        quotes: usize,
        projection: &mut Projection,
        targets: &mut Targets,
        docfx: (&HashSet<usize>, &mut Vec<Range<usize>>),
    ) {
        let quotes = quotes + usize::from(matches!(node, Node::Blockquote(_)));
        if matches!(node, Node::Heading(_))
            && let Some(position) = node.position()
        {
            let span = base + position.start.offset..base + position.end.offset;
            if docfx.0.contains(&span.start) {
                docfx.1.push(span.clone());
            }
            let mut text = String::new();
            projection.heading(node, source, base, quotes, &mut text);
            let lower = text.to_lowercase();
            let slug = SLUG_DENY.replace_all(&lower, "").replace(' ', "-");
            let fragment = format!("#{}", encode_component(&slug));
            if fragment != "#" {
                let count = targets.counts.get(&fragment).copied().unwrap_or(0);
                if count > 0 {
                    targets.put(format!("{fragment}-{count}"), 0);
                }
                targets.put(fragment, count + 1);
                for anchor in ANCHOR.captures_iter(&source.text()[span]) {
                    if !targets.counts.contains_key(&anchor[1]) {
                        targets.put(anchor[1].into(), 1);
                    }
                }
            }
        }
        if let Some(children) = node.children() {
            for child in children {
                headings(
                    child,
                    source,
                    base,
                    quotes,
                    projection,
                    targets,
                    (docfx.0, docfx.1),
                );
            }
        }
    }
    headings(
        &recovered.tree,
        &view,
        base,
        0,
        &mut projection,
        &mut targets,
        (&docfx, &mut docfx_ranges),
    );
    for token in html_tokens(source, parsed) {
        let raw = &source.text()[token.span];
        let Some(rest) = raw.strip_prefix('<') else {
            continue;
        };
        if rest.starts_with(['!', '>', '/']) {
            continue;
        }
        let end = rest
            .find(|ch| matches!(ch, '/' | '>') || js_space(ch))
            .unwrap_or(rest.len());
        let anchor = ID.captures(raw).or_else(|| {
            rest[..end]
                .eq_ignore_ascii_case("a")
                .then(|| NAME.captures(raw))
                .flatten()
        });
        if let Some(anchor) = anchor {
            targets.put(format!("#{}", &anchor[1]), 0);
        }
    }
    let references = reference_analysis(
        &view,
        &recovered,
        ReferenceFields {
            uses: false,
            styles: true,
            destinations: true,
        },
    );
    let mut destinations = vec![];
    docfx_ranges.sort_by_key(|heading| heading.start);
    for style in references.styles {
        if !matches!(style.kind, LinkKind::Inline)
            || view.text()[style.span.clone()].starts_with('!')
        {
            continue;
        }
        let Some(destination) = style.destination else {
            continue;
        };
        let prior = docfx_ranges.partition_point(|heading| heading.start <= style.span.start);
        let docfx_tab = prior > 0 && style.span.end <= docfx_ranges[prior - 1].end;
        let value = projection.string(&view.text()[destination.clone()], 0, true);
        destinations.push(FragmentDestination {
            span: offsets[style.span.start]..offsets[style.span.end],
            destination: offsets[destination.start]..offsets[destination.end],
            value,
            inline: true,
            docfx_tab,
        });
    }
    for definition in references.definitions {
        let Some(destination) = definition.destination else {
            continue;
        };
        let value = projection.string(&view.text()[destination.clone()], 0, true);
        destinations.push(FragmentDestination {
            span: offsets[definition.span.start]..offsets[definition.span.end],
            destination: offsets[destination.start]..offsets[destination.end],
            value,
            inline: false,
            docfx_tab: false,
        });
    }
    destinations.sort_by_key(|link| link.span.start);
    FragmentSnapshot {
        targets: targets.ordered,
        destinations,
    }
}
