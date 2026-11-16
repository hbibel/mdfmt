# Content-dependent diagnostics

MD001, MD025, MD036, MD041, MD043 and MD059 now report configured findings
on the final formatted output. They do not rewrite heading levels, add titles,
replace emphasized text with headings, or invent descriptive link labels.

## Implemented behavior

- MD001 tracks heading increments, including disabled reference headings and
  a recognized front-matter title.
- MD025 checks configured title levels, front-matter titles, preceding content
  and Docfx tab exceptions.
- MD041 checks the first content block, configured levels and preamble handling,
  including HTML heading tags. Comments alone do not constitute a preamble.
- MD043 checks literal heading requirements, case handling, wildcard markers
  and bounded loops. Bounded native regex requirements are now supported in the
  [configuration-pattern batch](config-patterns.md).
- MD036 reports root paragraphs consisting of a single plain emphasis or strong
  span, respecting configured trailing punctuation and nested/inline exclusions.
  Punctuation uses a JavaScript character class, including ranges, negation and
  escapes. Invalid classes and attempts to close the class early fail explicitly;
  arbitrary injected regular expressions remain outside the supported boundary.
- MD059 normalizes label text using upstream's ASCII word semantics. It checks
  resolved Markdown links, including nested image descriptions, without changing
  destinations or labels. Direct code/HTML label children retain upstream exceptions.

All six respect aliases, severity and inline permissions. Disabled fixes retain
enabled findings; enabling default rules now also enables MD041's missing-title
finding on ordinary untitled prose. Successful formatting can therefore return
exit 1 without changing the document.

## Evidence

The pinned markdownlint 0.42.0 development oracle records 1,578 heading-structure
observations and 554 emphasis/link observations, exercised by native Rust tests
and CLI comparisons. Coverage includes YAML/TOML/JSON metadata, custom title
options, nested lists/quotes, multiline setext headings, HTML, math, image
descriptions, directives, Unicode, BOM and line terminators. A diagnostic-only
parse view maps expanded tabs and recovered HTML boundaries back to the original
source. Formatter edit authorization never uses this synthetic tree.

Independent review found and regressions corrected indented HTML title handling,
raw Docfx destination classification, setext headings after recovered HTML,
quoted multiline image labels/reference definitions, and punctuation classes.
The literal-caption follow-up preserves hashes in MD025 contexts and MD043
requirements. A Docfx exception now requires that its link occupy the entire
lexical caption, preventing a preceding literal hash from hiding duplicate titles.

Reproduce from the repository root, with the pinned oracle dependencies installed:

```sh
mise exec -- cargo test --test heading_structure --test content_diagnostics --test md024
mise exec -- cargo build
node tools/parser-oracle/heading-structure.mjs
node tools/parser-oracle/content-diagnostics.mjs
node tools/parser-oracle/md024.mjs
```

## Intentional upstream-error differences

- MD043 throws an internal error for some missing-heading configurations on a
  metadata-only input without a final newline. Native findings point to the last
  physical source line instead of crashing.
- MD059 throws when a prohibited normalized empty label produces a zero-length
  error range. Native findings use a line location without a character range.

Receipts identify these rows with `upstreamError`; their native expectations are
explicit safety decisions, not successful upstream runtime findings. Remaining
parser differences documented in the parser evaluation still apply; these
comparisons do not establish full upstream parser equivalence.
