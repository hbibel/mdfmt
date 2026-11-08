use crate::Source;

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
