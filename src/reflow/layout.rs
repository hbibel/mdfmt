use crate::{
    Source,
    edit::Edit,
    parse::{ParsedDocument, RegionKind},
};
use std::ops::Range;

pub(super) struct Gap {
    pub span: Range<usize>,
    pub flat: String,
    pub wrapped: String,
}

pub(super) struct Paragraph {
    pub span: Range<usize>,
    pub continuation: String,
    pub gaps: Vec<Gap>,
    pub eligible: bool,
}

pub(crate) struct Layout {
    pub(super) paragraphs: Vec<Paragraph>,
}

impl Layout {
    pub(crate) fn new(source: &Source, parsed: &ParsedDocument) -> Self {
        let mut paragraphs: Vec<Paragraph> = vec![];
        let mut owners = vec![None; parsed.regions.len()];
        let mut blocked = vec![false; parsed.regions.len()];
        let mut tables: Vec<_> = parsed
            .regions
            .iter()
            .filter(|region| region.kind == RegionKind::Table)
            .map(|region| region.span.clone())
            .collect();
        tables.sort_by_key(|span| span.start);
        let mut protected: Vec<std::ops::Range<usize>> = vec![];
        for span in tables {
            if let Some(last) = protected.last_mut().filter(|last| span.start <= last.end) {
                last.end = last.end.max(span.end);
            } else {
                protected.push(span);
            }
        }
        for (index, region) in parsed.regions.iter().enumerate() {
            let parent_owner = region.parent.and_then(|parent| owners[parent]);
            blocked[index] =
                region.kind.is_protected() || region.parent.is_some_and(|parent| blocked[parent]);
            if region.kind == RegionKind::Paragraph {
                owners[index] = Some(paragraphs.len());
                paragraphs.push(Paragraph {
                    span: region.span.clone(),
                    continuation: continuation(source, parsed, index),
                    gaps: vec![],
                    eligible: !source.text()[region.span.clone()].contains('\t') && {
                        let prior = protected.partition_point(|span| span.start < region.span.end);
                        prior == 0 || protected[prior - 1].end <= region.span.start
                    },
                });
            } else {
                owners[index] = parent_owner;
            }
            let Some(owner) = owners[index] else {
                continue;
            };
            let paragraph = &mut paragraphs[owner];
            if region.kind.is_protected()
                && region.kind != RegionKind::Break
                && source.text()[region.span.clone()].contains(['\r', '\n'])
            {
                paragraph.eligible = false;
            }
            if region.kind != RegionKind::Text || blocked[index] {
                continue;
            }
            let mut cursor = region.span.start;
            let bytes = source.text().as_bytes();
            while cursor < region.span.end {
                if !matches!(bytes[cursor], b' ' | b'\r' | b'\n') {
                    cursor += 1;
                    continue;
                }
                let start = cursor;
                while cursor < region.span.end && matches!(bytes[cursor], b' ' | b'\r' | b'\n') {
                    let newline = matches!(bytes[cursor], b'\r' | b'\n');
                    if bytes[cursor] == b'\r' && bytes.get(cursor + 1) == Some(&b'\n') {
                        cursor += 1;
                    }
                    cursor += 1;
                    if newline {
                        while cursor < region.span.end && matches!(bytes[cursor], b' ' | b'>') {
                            cursor += 1;
                        }
                    }
                }
                if start <= paragraph.span.start || cursor >= paragraph.span.end {
                    continue;
                }
                let literal = &source.text()[start..cursor];
                let ending = if literal.contains("\r\n") {
                    "\r\n"
                } else if literal.contains('\r') {
                    "\r"
                } else if literal.contains('\n') {
                    "\n"
                } else {
                    super::newline(source.text(), paragraph.span.end)
                };
                paragraph.gaps.push(Gap {
                    span: start..cursor,
                    flat: if literal.contains(['\r', '\n']) {
                        " ".into()
                    } else {
                        literal.into()
                    },
                    wrapped: format!("{ending}{}", paragraph.continuation),
                });
            }
        }
        Self { paragraphs }
    }

    pub(crate) fn allows(&self, edit: &Edit) -> bool {
        if edit.origin != "MD013" {
            return false;
        }
        let index = self
            .paragraphs
            .partition_point(|p| p.span.start <= edit.span.start);
        let Some(paragraph) = index.checked_sub(1).and_then(|i| self.paragraphs.get(i)) else {
            return false;
        };
        if !paragraph.eligible {
            return false;
        }
        let index = paragraph
            .gaps
            .partition_point(|gap| gap.span.start < edit.span.start);
        paragraph.gaps.get(index).is_some_and(|gap| {
            gap.span == edit.span
                && (edit.replacement == gap.flat || edit.replacement == gap.wrapped)
        })
    }
}

fn continuation(source: &Source, parsed: &ParsedDocument, index: usize) -> String {
    let paragraph = &parsed.regions[index];
    let position = source.position(paragraph.span.start).unwrap();
    let line = source.line_range(position.line).unwrap();
    let start =
        line.start + usize::from(position.line == 1 && source.text().starts_with('\u{feff}')) * 3;
    let mut prefix = source.text().as_bytes()[start..paragraph.span.start].to_vec();
    let mut current = paragraph.parent;
    let mut container = false;
    while let Some(index) = current {
        let region = &parsed.regions[index];
        container |= matches!(region.kind, RegionKind::ListItem | RegionKind::Blockquote);
        if region.kind == RegionKind::ListItem
            && start <= region.span.start
            && region.span.start < paragraph.span.start
        {
            let marker = region.span.start - start;
            let mut end = marker;
            if prefix[marker].is_ascii_digit() {
                while end < prefix.len() && prefix[end].is_ascii_digit() {
                    end += 1;
                }
            }
            if end < prefix.len() {
                end += 1;
            }
            prefix[marker..end].fill(b' ');
        }
        current = region.parent;
    }
    if prefix.ends_with(b"[x] ") || prefix.ends_with(b"[X] ") || prefix.ends_with(b"[ ] ") {
        prefix.truncate(prefix.len() - 4);
    }
    if container {
        String::from_utf8(prefix).unwrap()
    } else {
        String::new()
    }
}
