#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Source, parse::parse};

    fn edit(span: std::ops::Range<usize>, replacement: &str) -> Edit {
        Edit {
            span,
            replacement: replacement.into(),
            origin: "test".into(),
        }
    }

    #[test]
    fn localized_edits_preserve_unrelated_bytes() {
        let source = Source::new("Hello world!\r\n".into());
        let parsed = parse(&source).unwrap();
        assert_eq!(
            apply_edits(&source, &parsed, vec![edit(6..11, "Rust")]).unwrap(),
            "Hello Rust!\r\n"
        );
        assert_eq!(
            apply_edits(&source, &parsed, vec![edit(0..5, "Hi"), edit(5..6, ", ")]).unwrap(),
            "Hi, world!\r\n"
        );
    }

    #[test]
    fn rejects_invalid_and_conflicting_batches() {
        let source = Source::new("a😀 world".into());
        let parsed = parse(&source).unwrap();
        for edits in [
            vec![edit(2..3, "x")],
            vec![edit(0..100, "x")],
            vec![edit(std::ops::Range { start: 6, end: 1 }, "x")],
            vec![edit(6..8, "a"), edit(7..9, "b")],
            vec![edit(6..6, "a"), edit(6..6, "b")],
            vec![edit(6..8, "a"), edit(8..8, "b")],
            vec![edit(6..6, "b"), edit(6..8, "a")],
        ] {
            assert!(apply_edits(&source, &parsed, edits).is_err());
        }
        assert_eq!(source.text(), "a😀 world");
    }

    #[test]
    fn rejects_edits_inside_and_at_protected_boundaries() {
        for text in [
            "[link](url)",
            "![image](img)",
            "`code`",
            "<!-- comment -->",
            "<b>",
            "| a |\n| - |\n| b |",
            "```\ncode\n```",
            "    code",
            "[id]: url",
            "---\ntitle: Hi\n---\n",
            "hard  \nbreak",
            "*emphasis*",
            "# heading",
        ] {
            let source = Source::new(text.into());
            let parsed = parse(&source).unwrap();
            for region in parsed.regions.iter().filter(|r| r.kind.is_protected()) {
                for span in [
                    region.span.clone(),
                    region.span.start..region.span.start,
                    region.span.end..region.span.end,
                ] {
                    assert!(
                        apply_edits(&source, &parsed, vec![edit(span, "x")]).is_err(),
                        "{text}"
                    );
                }
            }
        }
    }

    #[test]
    fn eligible_edits_do_not_grant_access_to_adjacent_link_or_container() {
        let source = Source::new("> Hello [link](url)\n".into());
        let parsed = parse(&source).unwrap();
        assert_eq!(
            apply_edits(&source, &parsed, vec![edit(2..7, "Hi")]).unwrap(),
            "> Hi [link](url)\n"
        );
        assert!(apply_edits(&source, &parsed, vec![edit(0..1, "")]).is_err());
    }

    #[test]
    fn upstream_frontmatter_whitespace_does_not_expose_metadata() {
        let source = Source::new("---\ntitle: Hello\n---\u{00a0}\n\nworld\n\n---\nProse\n".into());
        let parsed = parse(&source).unwrap();
        assert!(apply_edits(&source, &parsed, vec![edit(24..29, "Rust")]).is_err());
    }

    #[test]
    fn multiline_text_cannot_grant_access_to_container_prefixes() {
        for text in ["> first\n> second\n", "- first\n  continuation\n"] {
            let source = Source::new(text.into());
            let parsed = parse(&source).unwrap();
            assert!(apply_edits(&source, &parsed, vec![edit(7..10, "\n# ")]).is_err());
        }
    }
}
use crate::{
    Source,
    diagnostic::ProcessingError,
    parse::{ParsedDocument, RegionKind},
};
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedEdit {
    pub span: Range<usize>,
    pub replacement: String,
    pub origin: String,
}

pub(crate) type Edit = AppliedEdit;

pub(crate) fn validate_ranges(source: &Source, edits: &[Edit]) -> Result<(), ProcessingError> {
    for edit in edits {
        let span = &edit.span;
        if span.start > span.end
            || source.position(span.start).is_err()
            || source.position(span.end).is_err()
        {
            return Err(ProcessingError::InvalidRange(span.clone()));
        }
    }
    Ok(())
}

pub(crate) fn apply_edits(
    source: &Source,
    parsed: &ParsedDocument,
    mut edits: Vec<Edit>,
) -> Result<String, ProcessingError> {
    edits.sort_by_key(|edit| (edit.span.start, edit.span.end));
    validate_ranges(source, &edits)?;
    for edit in &edits {
        let span = &edit.span;
        let protected = parsed.regions.iter().any(|region| {
            region.kind.is_protected()
                && if span.is_empty() {
                    region.span.start <= span.start && span.start <= region.span.end
                } else {
                    span.start < region.span.end && region.span.start < span.end
                }
        });
        let eligible = parsed.regions.iter().any(|region| {
            region.kind == RegionKind::Text
                && region.parent.is_some()
                // Text-node ranges can include list/quote prefixes after soft breaks.
                && !source.text()[region.span.clone()].contains(['\r', '\n'])
                && region.span.start <= span.start
                && span.end <= region.span.end
        });
        if protected || !eligible {
            return Err(ProcessingError::ProtectedRegion(span.clone()));
        }
    }
    for pair in edits.windows(2) {
        let (a, b) = (&pair[0].span, &pair[1].span);
        if a.end > b.start || (a.end == b.start && (a.is_empty() || b.is_empty())) {
            return Err(ProcessingError::EditConflict);
        }
    }
    let mut output = String::new();
    let mut cursor = 0;
    for edit in edits {
        output.push_str(&source.text()[cursor..edit.span.start]);
        output.push_str(&edit.replacement);
        cursor = edit.span.end;
    }
    output.push_str(&source.text()[cursor..]);
    Ok(output)
}
