use super::{ParsedDocument, RegionKind, expanded_tabs};
use crate::Source;

struct Frame {
    index: usize,
    kind: RegionKind,
    line: usize,
    width: usize,
}
pub(crate) struct ContainerPath {
    frames: Vec<Frame>,
}

fn expanded(text: &str) -> (String, Vec<usize>) {
    let bom = usize::from(text.starts_with('\u{feff}')) * 3;
    let (source, mut offsets) = expanded_tabs(&Source::new(text[bom..].into()));
    for offset in &mut offsets {
        *offset += bom;
    }
    (source.text().into(), offsets)
}
fn spaces(text: &str, cursor: usize) -> usize {
    text[cursor..]
        .bytes()
        .take_while(|byte| *byte == b' ')
        .count()
}
fn quote_end(text: &str, cursor: usize) -> Option<usize> {
    let gap = spaces(text, cursor);
    let marker = cursor + gap;
    (gap <= 3 && text.as_bytes().get(marker) == Some(&b'>'))
        .then(|| marker + 1 + usize::from(text.as_bytes().get(marker + 1) == Some(&b' ')))
}
fn marker_end(text: &str, cursor: usize) -> Option<usize> {
    let gap = spaces(text, cursor);
    if gap > 3 {
        return None;
    }
    let start = cursor + gap;
    let digits = text[start..].bytes().take_while(u8::is_ascii_digit).count();
    let width = if digits > 0 && matches!(text.as_bytes().get(start + digits), Some(b'.' | b')')) {
        digits + 1
    } else if matches!(text.as_bytes().get(start), Some(b'-' | b'+' | b'*')) {
        1
    } else {
        return None;
    };
    let end = start + width;
    let padding = spaces(text, end);
    Some(
        end + if (1..=4).contains(&padding) {
            padding
        } else {
            1
        },
    )
}

impl ContainerPath {
    pub(crate) fn new(source: &Source, parsed: &ParsedDocument, indices: Vec<usize>) -> Self {
        let mut frames: Vec<Frame> = vec![];
        for index in indices {
            let region = &parsed.regions[index];
            let line = source.position(region.span.start).unwrap().line;
            let mut width = 0;
            if region.kind == RegionKind::ListItem {
                let text = expanded(&source.text()[source.line_range(line).unwrap()]).0;
                let mut cursor = 0;
                for parent in &frames {
                    cursor = if parent.kind == RegionKind::Blockquote {
                        quote_end(&text, cursor).unwrap_or(cursor)
                    } else if parent.line == line {
                        marker_end(&text, cursor).unwrap_or(cursor)
                    } else {
                        let end = cursor + parent.width;
                        if end <= text.len() && text.is_char_boundary(end) {
                            end
                        } else {
                            cursor
                        }
                    };
                }
                width = marker_end(&text, cursor).unwrap_or(cursor + 2) - cursor;
            }
            frames.push(Frame {
                index,
                kind: region.kind,
                line,
                width,
            });
        }
        Self { frames }
    }

    pub(crate) fn for_region(source: &Source, parsed: &ParsedDocument, index: usize) -> Self {
        let mut indices = vec![];
        let mut parent = parsed.regions[index].parent;
        while let Some(index) = parent {
            if matches!(
                parsed.regions[index].kind,
                RegionKind::ListItem | RegionKind::Blockquote
            ) {
                indices.push(index);
            }
            parent = parsed.regions[index].parent;
        }
        indices.reverse();
        Self::new(source, parsed, indices)
    }

    fn walk(&self, text: &str) -> Result<usize, usize> {
        let mut cursor = 0;
        for frame in &self.frames {
            let next = if frame.kind == RegionKind::Blockquote {
                quote_end(text, cursor)
            } else {
                (spaces(text, cursor) >= frame.width).then_some(cursor + frame.width)
            };
            let Some(next) = next else {
                return Err(frame.index);
            };
            cursor = next;
        }
        Ok(cursor)
    }

    pub(crate) fn first_missing(&self, text: &str) -> Option<usize> {
        self.walk(&expanded(text).0).err()
    }

    pub(crate) fn content_start(&self, text: &str) -> Option<usize> {
        let (expanded, offsets) = expanded(text);
        let cursor = self.walk(&expanded).ok()?;
        Some(offsets[cursor] + usize::from(cursor > 0 && offsets[cursor] == offsets[cursor - 1]))
    }
}
