use crate::Source;
use std::ops::Range;

pub(crate) fn content(source: &Source, span: Range<usize>, setext: bool) -> Range<usize> {
    if setext {
        let underline = source.position(span.end).unwrap().line;
        let end = source.line_range(underline - 1).unwrap().end;
        let raw = &source.text()[span.start..end];
        let text = raw.trim_start_matches([' ', '\t']);
        let start = end - text.len();
        return start..start + text.trim_end_matches([' ', '\t']).len();
    }
    let raw = source.text()[span.clone()].trim_start_matches([' ', '\t']);
    let markers = raw.bytes().take_while(|byte| *byte == b'#').count();
    let text = raw[markers..].trim_start_matches([' ', '\t']);
    let start = span.end - text.len();
    let mut text = text.trim_end_matches([' ', '\t']);
    let without_hashes = text.trim_end_matches('#');
    if without_hashes.len() < text.len()
        && (without_hashes.is_empty() || without_hashes.ends_with([' ', '\t']))
    {
        text = without_hashes.trim_end_matches([' ', '\t']);
    }
    start..start + text.len()
}
