use super::{ParsedDocument, RegionKind, diagnostic_list_parse, diagnostic_parse, parse};
use crate::Source;
use std::collections::HashMap;
use std::ops::Range;

struct Continuation {
    span: Range<usize>,
    prefix: String,
}

fn continuations(source: &Source, parsed: &ParsedDocument) -> Vec<Continuation> {
    let mut siblings = HashMap::new();
    let mut output = vec![];
    for (index, region) in parsed.regions.iter().enumerate() {
        let previous = region
            .parent
            .and_then(|parent| siblings.insert(parent, index));
        if region.kind != RegionKind::Html
            || !region.parent.is_some_and(|parent| {
                matches!(
                    parsed.regions[parent].kind,
                    RegionKind::Root | RegionKind::ListItem | RegionKind::Blockquote
                )
            })
        {
            continue;
        }
        let Some(container) = previous
            .map(|index| &parsed.regions[index])
            .filter(|r| matches!(r.kind, RegionKind::List | RegionKind::Blockquote))
        else {
            continue;
        };
        let last = container.span.start
            + source.text()[container.span.clone()]
                .trim_end_matches([' ', '\t', '\r', '\n'])
                .len();
        if source.position(last).unwrap().line + 1
            != source.position(region.span.start).unwrap().line
        {
            continue;
        }
        let Some(paragraph) = parsed.regions.iter().position(|r| {
            r.kind == RegionKind::Paragraph
                && r.span.start >= container.span.start
                && r.span.start < last
                && last <= r.span.end
        }) else {
            continue;
        };
        let probe = Source::new(format!("x\n{}", &source.text()[region.span.clone()]));
        let generic = parse(&probe).ok().is_some_and(|parsed| {
            parsed.regions.iter().any(|r| {
                matches!(r.kind, RegionKind::Paragraph | RegionKind::Heading)
                    && probe.position(r.span.start).unwrap().line == 1
                    && probe.position(r.span.end).unwrap().line > 1
            })
        });
        if generic {
            let raw = &source.text()[region.span.clone()];
            let start = region.span.start + raw.len() - raw.trim_start_matches([' ', '\t']).len();
            if source.text().as_bytes().get(start) != Some(&b'<') {
                continue;
            }
            let paragraph_line = source
                .position(parsed.regions[paragraph].span.start)
                .unwrap()
                .line;
            let paragraph_raw = &source.text()[source.line_range(paragraph_line).unwrap()];
            let path = super::ContainerPath::for_region(source, parsed, paragraph);
            let Some(end) = path.content_start_at(paragraph_raw, paragraph_line) else {
                continue;
            };
            let prefix = paragraph_raw[..end]
                .chars()
                .map(|ch| if ch == '>' { '>' } else { ' ' })
                .collect::<String>();
            output.push(Continuation {
                span: source
                    .line_range(source.position(start).unwrap().line)
                    .unwrap()
                    .start..start,
                prefix,
            });
        }
    }
    output
}

pub(crate) fn diagnostic_lists(source: &Source) -> Option<(Source, ParsedDocument, Vec<usize>)> {
    let (mut logical, mut parsed, mut offsets) = diagnostic_list_parse(source)?;
    if logical.text().contains('<') {
        loop {
            let continuations = continuations(&logical, &parsed);
            if continuations.is_empty() {
                break;
            }
            let mut text = String::new();
            let mut mapped = vec![offsets[0]];
            let mut cursor = 0;
            for continuation in continuations {
                text.push_str(&logical.text()[cursor..continuation.span.start]);
                mapped.extend_from_slice(&offsets[cursor + 1..=continuation.span.start]);
                // Upstream admits the first lazy HTML line as a block in the
                // active containers. It does not extend the old paragraph.
                let prefix = format!(
                    "{}\n{}",
                    continuation.prefix.trim_end(),
                    continuation.prefix
                );
                text.push_str(&prefix);
                mapped.extend(std::iter::repeat_n(
                    offsets[continuation.span.start],
                    prefix.len(),
                ));
                *mapped.last_mut().unwrap() = offsets[continuation.span.end];
                cursor = continuation.span.end;
            }
            text.push_str(&logical.text()[cursor..]);
            mapped.extend_from_slice(&offsets[cursor + 1..]);
            let view = source.mapped(text, &mapped);
            (logical, parsed, _) = diagnostic_list_parse(&view)?;
            offsets = mapped;
        }
    }
    let parsed = diagnostic_parse(&logical)
        .map(|tree| (*tree).clone())
        .unwrap_or(parsed);
    Some((logical, parsed, offsets))
}
