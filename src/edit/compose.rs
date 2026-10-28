use super::{Edit, validate_ranges};
use crate::{ProcessingError, Source};
use std::{collections::BTreeSet, ops::Range};

#[derive(Clone)]
enum Piece {
    Original(Range<usize>),
    Inserted { text: String, anchor: Range<usize> },
}

impl Piece {
    fn len(&self) -> usize {
        match self {
            Self::Original(span) => span.len(),
            Self::Inserted { text, .. } => text.len(),
        }
    }

    fn anchor(&self) -> Range<usize> {
        match self {
            Self::Original(span) => span.clone(),
            Self::Inserted { anchor, .. } => anchor.clone(),
        }
    }

    fn part(&self, span: Range<usize>) -> Result<Self, ProcessingError> {
        Ok(match self {
            Self::Original(original) => {
                Self::Original(original.start + span.start..original.start + span.end)
            }
            Self::Inserted { text, anchor } => Self::Inserted {
                text: text
                    .get(span.clone())
                    .ok_or(ProcessingError::InvalidRange(span))?
                    .into(),
                anchor: anchor.clone(),
            },
        })
    }
}

struct Event {
    anchor: Range<usize>,
    origin: String,
}

struct Reader<'a> {
    pieces: &'a [Piece],
    index: usize,
    within: usize,
}

impl Reader<'_> {
    fn take(&mut self, mut count: usize) -> Result<Vec<Piece>, ProcessingError> {
        let mut result = vec![];
        while count > 0 {
            let piece = self
                .pieces
                .get(self.index)
                .ok_or(ProcessingError::EditConflict)?;
            let amount = count.min(piece.len() - self.within);
            result.push(piece.part(self.within..self.within + amount)?);
            self.within += amount;
            count -= amount;
            if self.within == piece.len() {
                self.index += 1;
                self.within = 0;
            }
        }
        Ok(result)
    }

    fn boundary(&self, original_len: usize) -> Range<usize> {
        match self.pieces.get(self.index) {
            Some(Piece::Original(span)) => span.start + self.within..span.start + self.within,
            Some(piece) => piece.anchor(),
            None => original_len..original_len,
        }
    }
}

fn render(original: &str, pieces: &[Piece]) -> Result<String, ProcessingError> {
    let mut result = String::new();
    for piece in pieces {
        match piece {
            Piece::Original(span) => result.push_str(
                original
                    .get(span.clone())
                    .ok_or_else(|| ProcessingError::InvalidRange(span.clone()))?,
            ),
            Piece::Inserted { text, .. } => result.push_str(text),
        }
    }
    Ok(result)
}

fn stage(
    original: &str,
    pieces: &mut Vec<Piece>,
    events: &mut Vec<Event>,
    edits: &[Edit],
) -> Result<(), ProcessingError> {
    let source = Source::new(render(original, pieces)?);
    let mut edits = edits.to_vec();
    edits.sort_by_key(|edit| (edit.span.start, edit.span.end));
    validate_ranges(&source, &edits)?;
    if edits.windows(2).any(|pair| {
        pair[0].span.end > pair[1].span.start
            || (pair[0].span.is_empty()
                && pair[1].span.is_empty()
                && pair[0].span.start == pair[1].span.start)
    }) {
        return Err(ProcessingError::EditConflict);
    }
    let old = std::mem::take(pieces);
    let mut reader = Reader {
        pieces: &old,
        index: 0,
        within: 0,
    };
    let mut cursor = 0;
    let mut result = vec![];
    for edit in edits {
        result.extend(reader.take(edit.span.start - cursor)?);
        let removed = reader.take(edit.span.len())?;
        let anchor = removed
            .iter()
            .map(Piece::anchor)
            .reduce(|left, right| left.start.min(right.start)..left.end.max(right.end))
            .unwrap_or_else(|| reader.boundary(original.len()));
        events.push(Event {
            anchor: anchor.clone(),
            origin: edit.origin,
        });
        if !edit.replacement.is_empty() {
            result.push(Piece::Inserted {
                text: edit.replacement,
                anchor,
            });
        }
        cursor = edit.span.end;
    }
    result.extend(reader.take(source.text().len() - cursor)?);
    *pieces = result;
    Ok(())
}

fn flush(
    original: &str,
    span: Range<usize>,
    replacement: String,
    events: &[Event],
    cursor: &mut usize,
) -> Result<Option<Edit>, ProcessingError> {
    let end = events.partition_point(|event| event.anchor.start <= span.end);
    let mut origins = BTreeSet::new();
    for event in &events[*cursor..end] {
        if event.anchor.end >= span.start {
            origins.extend(event.origin.split('+'));
        }
    }
    *cursor = end;
    if original
        .get(span.clone())
        .ok_or_else(|| ProcessingError::InvalidRange(span.clone()))?
        == replacement
    {
        return Ok(None);
    }
    if origins.is_empty() {
        return Err(ProcessingError::EditConflict);
    }
    Ok(Some(Edit {
        span,
        replacement,
        origin: origins.into_iter().collect::<Vec<_>>().join("+"),
    }))
}

pub(crate) fn compose_edits(
    original: &str,
    prior: &[Edit],
    next: &[Edit],
) -> Result<Vec<Edit>, ProcessingError> {
    let mut pieces = if original.is_empty() {
        vec![]
    } else {
        vec![Piece::Original(0..original.len())]
    };
    let mut events = vec![];
    stage(original, &mut pieces, &mut events, prior)?;
    stage(original, &mut pieces, &mut events, next)?;
    events.sort_by_key(|event| (event.anchor.start, event.anchor.end));
    let mut result = vec![];
    let mut replacement = String::new();
    let mut cursor = 0;
    let mut event_cursor = 0;
    for piece in pieces {
        match piece {
            Piece::Original(span) => {
                if span.start < cursor {
                    return Err(ProcessingError::EditConflict);
                }
                if let Some(edit) = flush(
                    original,
                    cursor..span.start,
                    std::mem::take(&mut replacement),
                    &events,
                    &mut event_cursor,
                )? {
                    result.push(edit);
                }
                cursor = span.end;
            }
            Piece::Inserted { text, .. } => replacement.push_str(&text),
        }
    }
    if let Some(edit) = flush(
        original,
        cursor..original.len(),
        replacement,
        &events,
        &mut event_cursor,
    )? {
        result.push(edit);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::compose_edits;
    use crate::edit::Edit;
    use std::ops::Range;

    fn edit(span: Range<usize>, replacement: &str, origin: &str) -> Edit {
        Edit {
            span,
            replacement: replacement.into(),
            origin: origin.into(),
        }
    }

    fn check(original: &str, prior: &[Edit], next: &[Edit], expected: &str) -> Vec<Edit> {
        let edits = compose_edits(original, prior, next).unwrap();
        let mut output = String::new();
        let mut cursor = 0;
        for edit in &edits {
            output.push_str(&original[cursor..edit.span.start]);
            output.push_str(&edit.replacement);
            cursor = edit.span.end;
        }
        output.push_str(&original[cursor..]);
        assert_eq!(output, expected);
        edits
    }

    #[test]
    fn deleting_a_replacement_keeps_original_coordinates_and_both_origins() {
        let edits = check(
            "a\t\n",
            &[edit(1..2, "    ", "MD010")],
            &[edit(1..5, "", "MD009")],
            "a\n",
        );
        assert_eq!(edits, vec![edit(1..2, "", "MD009+MD010")]);
    }

    #[test]
    fn partial_deletion_and_replacement_of_inserted_text_are_composed() {
        check(
            "a\tb",
            &[edit(1..2, "    ", "MD010")],
            &[edit(2..4, "", "MD009")],
            "a  b",
        );
        check(
            "Hello world",
            &[edit(6..11, "wide world", "A")],
            &[edit(6..10, "Rust", "B")],
            "Hello Rust world",
        );
        check(
            "abc",
            &[edit(1..2, "XX", "A")],
            &[edit(2..2, "Y", "B")],
            "aXYXc",
        );
        check(
            "abcd",
            &[edit(1..3, "XY", "A")],
            &[edit(1..2, "12", "B"), edit(2..3, "34", "B")],
            "a1234d",
        );
    }

    #[test]
    fn eof_insertion_after_deletion_coalesces_the_original_suffix() {
        let edits = check(
            "# Title!",
            &[edit(7..8, "", "MD026")],
            &[edit(7..7, "\n", "MD047")],
            "# Title\n",
        );
        assert_eq!(edits, vec![edit(7..8, "\n", "MD026+MD047")]);
        check(
            "",
            &[edit(0..0, "😀", "A")],
            &[edit(4..4, "\r\n", "B")],
            "😀\r\n",
        );
    }

    #[test]
    fn independent_changes_and_unicode_crlf_boundaries_remain_original() {
        check(
            "alpha\tbeta\r\n",
            &[edit(5..6, "  ", "MD010")],
            &[edit(0..5, "a", "MD013"), edit(7..11, "b", "MD013")],
            "a  b\r\n",
        );
        check(
            "😀\tβ\r\n",
            &[edit(4..5, "  ", "MD010")],
            &[edit(6..8, "γ", "B")],
            "😀  γ\r\n",
        );
    }

    #[test]
    fn net_noop_stages_do_not_export_edits() {
        assert!(check("ab", &[edit(0..1, "x", "A")], &[edit(0..1, "a", "B")], "ab").is_empty());
        assert!(check("abc", &[edit(1..2, "b", "A")], &[], "abc").is_empty());
        assert!(check("", &[], &[], "").is_empty());
    }

    #[test]
    fn composed_results_can_feed_another_stage() {
        let first = check(
            "a\tb\r\n",
            &[edit(1..2, "    ", "MD010")],
            &[edit(2..4, "", "MD009")],
            "a  b\r\n",
        );
        let second = check("a\tb\r\n", &first, &[edit(2..3, "", "MD012")], "a b\r\n");
        assert_eq!(second, vec![edit(1..2, " ", "MD009+MD010+MD012")]);
    }

    #[test]
    fn invalid_unicode_crlf_ranges_and_conflicting_stages_return_errors() {
        assert!(compose_edits("😀x", &[edit(1..2, "?", "A")], &[]).is_err());
        assert!(
            compose_edits(
                "😀\tβ\r\n",
                &[edit(4..5, "  ", "A")],
                &[edit(9..9, "x", "B")]
            )
            .is_err()
        );
        assert!(compose_edits("abc", &[], &[edit(4..4, "x", "B")]).is_err());
        assert!(compose_edits("abcd", &[], &[edit(1..3, "x", "B"), edit(2..4, "y", "B")]).is_err());
        assert!(compose_edits("abcd", &[], &[edit(1..1, "x", "B"), edit(1..1, "y", "B")]).is_err());
        assert!(
            compose_edits(
                "abcd",
                &[edit(std::ops::Range { start: 3, end: 1 }, "", "A")],
                &[]
            )
            .is_err()
        );
    }
}
