# Reference diagnostics

MD052 reports undefined reference labels without inventing destinations. It works
independently when no other rule is enabled. Configured shortcut handling, exact
ignored-label matching, aliases, warnings and inline directives are respected.

The read-only model derives original-byte spans and helper-compatible keys from
source-backed inline scopes, definitions and unresolved-label candidates. It does
not substitute native parser identifiers: for example, native case folding can
resolve `[ß]` against `[SS]: /url`, while the pinned helper still reports `ß`.
Definitions on disabled lines remain resolution anchors.

Undefined labels follow the pinned shim's adjacent full/collapsed/shortcut grouping
and data/line-ending projection. Escapes, entities, attention markers and code
tokens are not ordinary data. Shortcut quote prefixes are excluded, while failed
full-reference line-ending tokens can retain raw quote prefixes. Hard-break
escapes omit the backslash but retain their line ending. Image-label recovery
keeps original definitions and nested quote context.

Contexts use the first physical comment-masked display line; diagnostic spans
retain valid original-byte ranges. BOM/CRLF, UTF-16 lengths, metadata, HTML, math,
tables, nested images and escaped image markers are covered. No source edit is
proposed; fixes enabled/disabled and second-call behavior remain identical.

## Evidence

`tests/md052.rs` contains four tests and consumes 2,037 paired pinned observations.
Each receipt records direct runtime and parser-forced outcomes separately. Pinned
MD052 declares `parser: "none"` but otherwise reads an empty token cache; an inert
micromark rule supplies the source-reference observations used for comparison.
Native diagnostics intentionally do not inherit that false-clean dependency.

The native development oracle checks diagnostics, exact source, exit status and
idempotence with fixes enabled/disabled. A CLI budget test covers 1,000 missing
references plus a 256 KiB trailing segment, without reparsing the document per use.
Balanced rejected outer labels are skipped rather than repeatedly projecting
nested closing brackets.

Task 1's full native suite passes 430 tests across 59 targets; fmt/clippy are
clean. Prior spacing/style diagnostic and preservation comparisons also pass.

```sh
mise exec -- cargo test --test md052
mise exec -- cargo build
node tools/parser-oracle/reference-diagnostics.mjs --rule MD052
```

## Unused and duplicate definitions

MD053 reports unused first definitions and later duplicates independently,
including footnotes. Definitions and uses on disabled lines still contribute to
identity/usage bookkeeping; only reporting permissions are filtered by line.
Ignored-definition arrays match normalized helper keys exactly, without silently
normalizing the option values.

Findings cover the original whole physical definition line, including container
prefixes. Contexts use masked display text and UTF-16 head ellipses; BOM columns
exclude the leading marker. Unused/duplicate definition content is never deleted,
including when destinations or titles differ.

`tests/md053.rs` contains five tests and consumes 1,825 paired direct/parser-forced
observations. It verifies disabled anchors/uses, quote/list and multiline records,
Unicode/comment contexts, CLI aliases/warnings, MD052/MD013/MD026 composition and
second-call stability. MD053 alone works despite the pinned parser-less cache bug.
Task 2's full suite passes 435 tests across 60 targets; fmt/clippy are clean.

```sh
mise exec -- cargo test --test md053
node tools/parser-oracle/reference-diagnostics.mjs --rule MD053
```

MD054 remains planned; its source audit is not native coverage.
