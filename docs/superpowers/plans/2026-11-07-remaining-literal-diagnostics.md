# Remaining Literal Diagnostics Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Correct literal-hash heading keys and add MD033 HTML and MD060 table-column findings without deleting HTML or rewriting table source.

**Architecture:** Reuse source-mapped diagnostic parsing, HTML recovery and table snapshots. Keep both rules diagnostic-only under the preservation contract. Compare final-output diagnostics against pinned markdownlint, retaining separate display-width and UTF-16 coordinate calculations.

**Tech Stack:** Rust 1.99, existing Markdown/configuration pipeline, pinned markdownlint 0.42.0 development oracle. Use a native Unicode display-width implementation for MD060, locked in Cargo.lock; Node remains development-only.

Width baseline decision: target the audited string-width 8.3.0 behavior under
Node 24.19.0, Unicode 17.0/ICU 78.3. Do not substitute `UnicodeWidthStr::width`
unchanged: upstream runtime reports width 2 for Arabic `لا`, 1 for unmatched
Hangul `ᅡ`, 0 for tabs, and strips ANSI escapes, while native libraries have
different ligature policies. Add pinned differential fixtures before selecting
the native adapter; document runtime-dependent compatibility boundaries.

**Spec:** `docs/superpowers/specs/2026-10-21-mdfmt-design.md`

## Global Constraints

- Must respect the effective supported markdownlint configuration.
- Must preserve content and Markdown meaning rather than force an unsafe fix.
- Must preserve tables. Violations involving tables may remain unresolved.
- Must report remaining violations when the effective configuration flags them.
- Must be idempotent: formatting the result again produces identical output.
- Keep the sibling worktree and shared parent configuration. Continue inline without routine approval pauses.

## Review Focus

- Opening versus closing tags, comments/declarations/autolinks, multiline tags and HTML in image descriptions require token boundaries, not a global angle-bracket regex.
- An explicitly empty table allow-list does not inherit ordinary HTML allowances; omitted table allowances do.
- MD060 uses terminal display width for alignment, unlike MD013's UTF-16 width. CJK, emoji sequences, combining marks and ANSI escapes need pinned comparisons.
- MD060 style `any` chooses the fewest findings per table, with upstream tie precedence; `aligned_delimiter` also affects compact/tight styles.
- Nested/recovered tables, literal pipes in code, escaped pipes, empty cells, physical line overrides, metadata, BOM and mixed terminators must retain exact source and byte-valid ranges.

## Task 1: Literal ATX caption keys

**Files:** extract `src/parse/heading.rs` from MD003's lexical caption helper; modify `src/parse.rs`, `src/md003.rs`, `src/md024.rs`, heading/duplicate receipts and tests, and corresponding evidence/roadmap.

**Interfaces:** `parse::heading_content(&Source, Range<usize>, bool) -> Range<usize>` selects lexical source content (`bool` is setext); existing diagnostic interfaces stay unchanged.

- [x] Add failing pinned/native regressions: MD024 must not conflate `# # Title` with `# Title`; repeated `# # Title` headings report context `# Title`; MD043 requirements `["# # Title"]` accept source `# # Title\n` without a finding.
- [x] Run MD024 and heading-structure tests to observe the false findings/range-key mismatch.
- [x] Move the verified lexical helper into the parser and use it for single-line ATX key boundaries. Keep existing HTML exclusion and multiline setext key logic unchanged. Decision: lexical source wins over native child positions because the pinned parser retains literal hashes the native parser omits.
- [x] Run the focused suites, heading/content/style oracles and full Rust tests/fmt/clippy; expected all pass, including the MD003 critical preservation regressions.

Task 1 verified: 335 Rust tests across 43 targets; fmt/clippy clean; 224 duplicate,
1,578 heading-structure, 1,206 style/identity and 554 content runtime comparisons.

## Task 2: HTML findings

**Files:** create `src/md033.rs`, `src/parse/html.rs`, `tests/md033.rs`, `tools/parser-oracle/md033.mjs`, its receipt and `docs/compatibility/md033.md`; modify final diagnostics/coverage, parser exports, CLI help, coverage tests, README, inventory and roadmap.

**Interfaces:** `md033::diagnose(&Source, &ParsedDocument, &DocumentConfig) -> Vec<Diagnostic>`.

Source-model decision: expose `parse::html_tokens(&Source, &ParsedDocument) ->
Vec<HtmlToken>`, where `HtmlToken` contains original `span: Range<usize>` and
`in_table: bool`. Exclude front matter and retain raw comment text in a
newline-normalized diagnostic view mapped back to original bytes; never change
the edit tree. MD033 consumes raw micromark tokens, before comment sanitization.
Reparse image descriptions as inline content, masking quote prefixes and
flattening line endings without changing offsets, so labels cannot invent block
or table ownership. These choices mirror the upstream diagnostic preprocessing,
not HTML execution or a DOM interpretation.

- [x] Record pinned opening-tag findings for inline/block HTML, multiline attributes, nested images, quoted continuations, code/math exclusions, raw scripts, allowances, tables, aliases, directives and severity.
- [x] Add failing literal tests: `<b>x</b>\n` reports the opening `b` tag only; allowing `B` suppresses it case-insensitively; `table_allowed_elements: []` still reports `<b>` inside a table even when ordinary `b` is allowed. Assert unchanged source and original byte-safe ranges.
- [x] Run `mise exec -- cargo test --test md033`; expected missing findings.
- [x] Implement source-backed opening-tag selection and allowance precedence using existing recovery patterns, without deleting or escaping HTML. Multiline diagnostic spans cover only the first physical line as pinned.
- [x] Add CLI check/write/warning and final-output reflow stability tests. Run the focused suite and oracle, then full Rust tests/fmt/clippy; expected all pass.

Task 2 verified: 340 Rust tests across 44 targets; fmt/clippy clean; 602 HTML,
207 image, 1,206 heading-style, 1,578 heading-structure and 554 content cases.

## Task 3: Table-column style findings

**Files:** create `src/md060.rs`, `tests/md060.rs`, `tools/parser-oracle/md060.mjs`, its receipt and `docs/compatibility/md060.md`; modify Cargo dependencies/lock if needed, final diagnostics/coverage, CLI help, coverage tests, README, inventory and roadmap.

**Interfaces:** `md060::diagnose(&Source, &ParsedDocument, &DocumentConfig) -> Vec<Diagnostic>`; reuse existing table rows/delimiters and source-backed pipe scanning.

- [ ] Record pinned aligned/compact/tight/any selection, equal-error ties, `aligned_delimiter`, nesting/recovery, escaped/code pipes, extra/missing cells, Unicode display widths, ANSI escapes, comments, metadata, directives and severity.
- [ ] Add failing literal tests on `| a |\n| --- |\n| b |\n`: aligned reports line 2 column 7, compact reports nothing, tight reports six padding findings, and any selects compact with no findings. Every mode preserves table bytes.
- [ ] Run `mise exec -- cargo test --test md060`; expected missing findings.
- [ ] Implement separate actual UTF-16 pipe columns and effective display columns, per-table candidate findings and upstream style/tie selection. Use a pinned native width dependency; document any verified Unicode-version or ANSI differences instead of claiming unsupported parity.
- [ ] Add CLI status, warning, permission and final-output position tests. Run focused/native oracle and full Rust tests/fmt/clippy; expected all pass.

## Task 4: Review and evidence

**Files:** update per-rule evidence and roadmap Important/Critical findings only.

- [ ] Review the batch independently; fix Important/Critical issues with failing regressions and a green full suite.
- [ ] Record verification commands/results and accurate remaining coverage. Do not count acceptance examples or unimplemented safe fixes as finished behavior.

Status: Tasks 1 and 2 are implemented and verified. MD060 and the batch review
remain pending; MD060 fixtures do not establish implemented behavior.
