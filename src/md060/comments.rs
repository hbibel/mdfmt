use crate::{
    Source,
    config::js_space,
    parse::{ParsedDocument, RegionKind},
};

pub(super) fn display_source(source: &Source, parsed: &ParsedDocument) -> Source {
    let text = source.text();
    let mut cursor = parsed
        .regions
        .iter()
        .find(|r| r.kind == RegionKind::FrontMatter)
        .map_or(0, |r| r.span.end);
    let mut masks = vec![];
    while let Some(start) = text[cursor..].find("<!--").map(|i| cursor + i) {
        let Some(end) = text[start + 2..].find("-->").map(|i| start + 2 + i) else {
            break;
        };
        if end > start + 4 {
            let content = &text[start + 4..end];
            let line = text[..start].rfind(['\r', '\n']).map_or(0, |i| i + 1);
            let prefix = &text[line..start];
            let block = prefix.trim_matches(js_space).is_empty();
            let table =
                prefix.trim_start_matches(' ').starts_with('|') && content.contains(['\r', '\n']);
            if block
                || !(table
                    || content.starts_with('>')
                    || content.starts_with("->")
                    || content.ends_with('-')
                    || content.contains("--"))
            {
                masks.push(start + 4..end);
            }
        }
        cursor = end + 3;
    }
    let mut output = String::new();
    let mut masks = masks.into_iter().peekable();
    for (i, ch) in text.char_indices() {
        while masks.peek().is_some_and(|mask| mask.end <= i) {
            masks.next();
        }
        let masked = masks.peek().is_some_and(|mask| {
            mask.contains(&i)
                && (ch != ' '
                    || text[i..mask.end]
                        .trim_start_matches(' ')
                        .starts_with(['\r', '\n']))
        });
        if masked && !matches!(ch, '\r' | '\n') {
            output.extend(std::iter::repeat_n('.', ch.len_utf16()));
        } else {
            output.push(ch);
        }
    }
    Source::new(output)
}

pub(super) fn prefix<'a>(source: &Source, display: &'a Source, pipe: usize) -> &'a str {
    let position = source.position(pipe).unwrap();
    let line = &display.text()[display.line_range(position.line).unwrap()];
    let mut units = 0;
    let end = line
        .char_indices()
        .find_map(|(i, ch)| {
            if units == position.column_utf16 - 1 {
                Some(i)
            } else {
                units += ch.len_utf16();
                None
            }
        })
        .unwrap_or(line.len());
    &line[..end]
}
