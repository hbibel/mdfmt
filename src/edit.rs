mod compose;
pub(crate) use compose::compose_edits;

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

    #[test]
    fn indexed_permissions_match_the_original_region_policy() {
        for text in [
            "alpha [a *b*](url) omega `code` tail",
            "> first\n> second\n\nplain text\n",
            "| a |\n| - |\n| b |\n\nplain text\n",
            "a😀 b\r\n\r\nhello world",
            "---\ntitle: test\n---\n\nplain text",
        ] {
            let source = Source::new(text.into());
            let parsed = parse(&source).unwrap();
            let index = EditableRegions::new(&source, &parsed);
            let boundaries: Vec<_> = (0..=text.len())
                .filter(|&offset| text.is_char_boundary(offset))
                .collect();
            for &start in &boundaries {
                for &end in boundaries.iter().filter(|&&end| end >= start) {
                    let span = start..end;
                    let protected = parsed.regions.iter().any(|region| {
                        region.kind.is_protected()
                            && if span.is_empty() {
                                region.span.start <= start && start <= region.span.end
                            } else {
                                start < region.span.end && region.span.start < end
                            }
                    });
                    let eligible = parsed.regions.iter().any(|region| {
                        region.kind == RegionKind::Text
                            && region.parent.is_some()
                            && !text[region.span.clone()].contains(['\r', '\n'])
                            && region.span.start <= start
                            && end <= region.span.end
                    });
                    assert_eq!(
                        index.allows(&span),
                        !protected && eligible,
                        "{text:?} {span:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn expanded_reflow_permissions_only_allow_exact_modeled_gaps() {
        let source = Source::new("- alpha\n  beta gamma\n".into());
        let parsed = parse(&source).unwrap();
        let mut join = edit(7..10, " ");
        join.origin = "MD013".into();
        assert_eq!(
            apply_edits(&source, &parsed, vec![join.clone()]).unwrap(),
            "- alpha beta gamma\n"
        );
        for replacement in ["\n> ", "\n- ", " text ", ""] {
            let mut invalid = join.clone();
            invalid.replacement = replacement.into();
            assert!(apply_edits(&source, &parsed, vec![invalid]).is_err());
        }
        join.origin = "MD026".into();
        assert!(apply_edits(&source, &parsed, vec![join]).is_err());
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
    /// Sorted `+`-joined rule IDs when multiple stages share one original region.
    pub origin: String,
}

pub(crate) type Edit = AppliedEdit;

pub(crate) struct EditableRegions {
    protected: Vec<Range<usize>>,
    eligible: Vec<Range<usize>>,
}

impl EditableRegions {
    pub(crate) fn new(source: &Source, parsed: &ParsedDocument) -> Self {
        let mut protected: Vec<_> = parsed
            .regions
            .iter()
            .filter(|region| region.kind.is_protected())
            .map(|region| region.span.clone())
            .collect();
        protected.sort_by_key(|span| (span.start, span.end));
        let mut merged: Vec<Range<usize>> = vec![];
        for span in protected {
            if let Some(last) = merged.last_mut()
                && span.start <= last.end
            {
                last.end = last.end.max(span.end);
            } else {
                merged.push(span);
            }
        }
        let mut eligible: Vec<_> = parsed
            .regions
            .iter()
            .filter(|region| {
                region.kind == RegionKind::Text
                    && region.parent.is_some()
                    && !source.text()[region.span.clone()].contains(['\r', '\n'])
            })
            .map(|region| region.span.clone())
            .collect();
        eligible.sort_by_key(|span| (span.start, span.end));
        Self {
            protected: merged,
            eligible,
        }
    }

    pub(crate) fn allows(&self, span: &Range<usize>) -> bool {
        let protected_index = self.protected.partition_point(|region| {
            if span.is_empty() {
                region.end < span.start
            } else {
                region.end <= span.start
            }
        });
        let protected = self.protected.get(protected_index).is_some_and(|region| {
            if span.is_empty() {
                region.start <= span.start
            } else {
                region.start < span.end
            }
        });
        let eligible_index = self
            .eligible
            .partition_point(|region| region.start <= span.start);
        let eligible = eligible_index
            .checked_sub(1)
            .and_then(|index| self.eligible.get(index))
            .is_some_and(|region| span.end <= region.end);
        !protected && eligible
    }
}

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
    edits: Vec<Edit>,
) -> Result<String, ProcessingError> {
    apply_authorized_edits(source, parsed, edits, &[])
}

pub(crate) fn apply_authorized_edits(
    source: &Source,
    parsed: &ParsedDocument,
    mut edits: Vec<Edit>,
    authorized: &[Edit],
) -> Result<String, ProcessingError> {
    edits.sort_by_key(|edit| (edit.span.start, edit.span.end));
    validate_ranges(source, &edits)?;
    let editable = EditableRegions::new(source, parsed);
    let layout = std::cell::OnceCell::new();
    let mut expanded = false;
    for edit in &edits {
        if source
            .front_matter_end()
            .is_some_and(|end| edit.span.start < end)
        {
            return Err(ProcessingError::ProtectedRegion(edit.span.clone()));
        }
        if authorized.contains(edit) {
            continue;
        }
        let span = &edit.span;
        if !editable.allows(span) {
            if !layout
                .get_or_init(|| crate::reflow::Layout::new(source, parsed))
                .allows(edit)
            {
                return Err(ProcessingError::ProtectedRegion(span.clone()));
            }
            expanded = true;
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
    for edit in &edits {
        output.push_str(&source.text()[cursor..edit.span.start]);
        output.push_str(&edit.replacement);
        cursor = edit.span.end;
    }
    output.push_str(&source.text()[cursor..]);
    if !source.metadata_stable(&output) {
        return Err(ProcessingError::ProtectedRegion(
            0..source.front_matter_end().unwrap_or(0),
        ));
    }
    if expanded {
        let mut baseline = String::new();
        let mut cursor = 0;
        for edit in edits.iter().filter(|edit| authorized.contains(edit)) {
            baseline.push_str(&source.text()[cursor..edit.span.start]);
            baseline.push_str(&edit.replacement);
            cursor = edit.span.end;
        }
        baseline.push_str(&source.text()[cursor..]);
        let baseline = source.derive(baseline);
        let baseline_parsed = crate::parse::parse(&baseline).map_err(ProcessingError::Parse)?;
        let candidate = source.derive(output.clone());
        let reparsed = crate::parse::parse(&candidate).map_err(ProcessingError::Parse)?;
        if !crate::parse::equivalent(&baseline, &baseline_parsed, &candidate, &reparsed) {
            return Err(ProcessingError::ProtectedRegion(0..source.text().len()));
        }
    }
    Ok(output)
}
