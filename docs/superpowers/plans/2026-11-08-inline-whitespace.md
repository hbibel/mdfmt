# Inline Whitespace Diagnostics Implementation Plan

> For agentic workers: REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax for tracking.

Goal: Implement MD037/MD038/MD039 findings without guessing intended emphasis or
changing literal code, link text or protected source.

Architecture: Generalize the verified read-only attention adapter into an inline
snapshot shared by the three whitespace rules and MD049/MD050. Preserve original
byte mappings, parent-token scopes, code-span data/padding and link-label ranges.
Diagnose final output after existing repair/reflow stages; this batch adds no
source-changing proposer or generic repeated-pass engine.

Tech stack: Existing Rust toolchain, native parser/configuration/source model;
pinned markdownlint 0.42.0 development oracle, no new runtime dependency.

Spec: `docs/superpowers/specs/2026-10-21-mdfmt-design.md`

## Decisions and assumptions

- Continue inline under the standing autonomous roadmap instruction. Implement
  these related inline rules next because the verified source attention model
  already supplies their hardest delimiter/ownership boundaries. MD052/MD053's
  separate parser-cache audit remains retained for the later reference batch.
- Report all three rules, preserving source instead of applying upstream edits
  that infer emphasis, remove literal code whitespace or alter link labels.
  This follows the approved preservation contract and inventory classifications;
  MD026's explicit approved content edit remains unchanged.
- Do not describe a receipt as an implemented rule. Coverage advances one rule
  only after focused tests, native comparisons and the full suite pass.
- Source-reviewed differences between bare attention, code padding and link-label
  whitespace remain separate policies, not a generic whitespace-trimming regex.
- Use original-source byte ranges for findings. Upstream columns exclude BOM;
  native spans retain it and CLI presentation normalizes it.
- Reuse comment-masked display text only where the pinned rule reads `params.lines`.
  Raw token contexts/data must not accidentally consume masked UTF-8 offsets.
- Retain bounded/batched marker validation and all preservation/MD026 dependency
  decisions from the completed emphasis-style plan while moving the shared model.

## Global Constraints

- Must respect the effective supported markdownlint configuration.
- Must preserve content and Markdown meaning rather than force an unsafe fix.
- Must preserve inline links without conversion to reference links.
- Must preserve tables. Violations involving tables may remain unresolved.
- Must report remaining violations when the effective configuration flags them.
- Must be idempotent: formatting the result again produces identical output.
- Use the existing sibling worktree, parent configuration and pinned Rust toolchain.
- Record Important/Critical findings with reproducing tests in the roadmap.

## Review Focus

- Bare marker pairing must follow direct token parents and identical marker runs,
  not pair delimiters across nested attention, code, link labels or table cells.
- Code-span whitespace can be meaningful data rather than syntax padding;
  preserve exact bytes while reporting tabs, Unicode spaces and backtick exceptions.
- Multiline labels and code spans must exclude container prefixes from data without
  losing their original ranges, including nested images and reference definitions.
- Comment masking, BOM, CRLF and UTF-16 truncation must not fabricate findings,
  corrupt range lengths or move diagnostics to an invalid physical line.
- Shared-model changes must preserve MD049/MD050 topology, native ownership and
  MD026 conflict behavior without per-token full-document reparsing.

## Task 1: Shared inline snapshot and MD037

Files: move `src/emphasis_styles/attention.rs` to `src/parse/attention.rs`;
modify `src/parse.rs` and `src/emphasis_styles.rs`; create `src/inline_spaces.rs`,
`src/inline_spaces/md037.rs`, `tests/md037.rs`,
`tools/parser-oracle/inline-whitespace.mjs`, its MD037 receipt and
`docs/compatibility/inline-whitespace.md`; update registration/help, README,
coverage assertions, inventory and roadmap.

Interfaces:

- `parse::inline_analysis(&Source, &ParsedDocument, InlineFields) -> InlineAnalysis`
  is read-only; review added explicit bare/code/label selection so existing style
  consumers do not extract unused whitespace records.
- `InlineAnalysis` initially owns `attention: Vec<AttentionPair>` and
  `bare: Vec<BareMarker>`. Tasks 2/3 extend it with their tested code/label records.
- `AttentionPair { opening: Range<usize>, closing: Range<usize> }` is parser data,
  not a rule ID. Its width selects MD049/MD050 in the existing rule module.
- `BareMarker { span: Range<usize>, parent: Range<usize> }` retains each remaining
  attention data token and its direct inline/attention-text parent. Clipped surplus
  runs stay separate tokens; a matched parent's interior is not its outer sibling.
- `inline_spaces::diagnose(&Source, &ParsedDocument, &DocumentConfig) -> Vec<Diagnostic>`
  constructs one snapshot for enabled spacing rules and dispatches to rule modules.
- Each rule consumes the source, effective document and shared snapshot. MD037
  additionally uses comment-masked display lines for its contexts.

- [x] Record source-only MD037 observations for one/two/three/four-marker runs,
  unmatched/odd/different markers, nested parent scopes, escapes, code, links,
  images, tables, HTML/math, comments, metadata, tabs, Unicode, directives and warnings.
  Run `node tools/parser-oracle/inline-whitespace.mjs --record --source-only --rule MD037`.
  Expected: pinned receipt only.
- [x] Write `bare_emphasis_spaces_are_reported_without_creating_emphasis`:
  `alpha * beta * omega\n` remains exact with fixes enabled/disabled, has no edits,
  and reports spans `7..8` and `12..13` with contexts `* b` and `a *`.
  Its second call has identical findings and no edits.
  Pin mismatched `*`/`**` markers as no finding and nested/direct-parent cases
  independently so a global marker regex cannot satisfy the test.
- [x] Run `mise exec -- cargo test --test md037`.
  Expected: missing MD037 findings, not fixture/configuration errors.
- [x] Move/generalize the attention adapter and emit remaining data-marker tokens
  with direct-parent identities after pinned resolution. Preserve document
  definitions and original container context in diagnostic image recovery;
  code/label records are added only with their consuming tasks' failing tests.
  Adapt MD049/MD050 to consume `AttentionPair` without changing their behavior.
- [x] Implement MD037's fixed marker order `_`, `__`, `___`, `*`, `**`, `***`,
  sequential pairing within each direct parent, and same-line JS-whitespace checks.
  Never promote unmatched data markers into new emphasis as a formatter repair.
- [x] Add original-byte/UTF-16/BOM/CRLF, disabled-target and alias-warning cases.
  Exercise bare markers inside protected inline units and after recovered HTML.
- [x] Run focused MD037 plus MD049/MD050/review suites and native oracles, then
  full Rust tests/fmt/clippy. Expected: MD037 matches pinned findings, source
  remains exact, and both 1,145-case style oracles plus 180 combined cases pass.
- [x] Update truthful coverage/evidence/roadmap and commit.

## Task 2: Literal code-span whitespace diagnostics

Files: create `src/inline_spaces/md038.rs`, `tests/md038.rs` and MD038 receipt;
extend the shared oracle/snapshot only as required by source-mapped code padding;
update registration/help, evidence, coverage assertions, README, inventory and roadmap.

Interfaces: extend Task 1's `InlineAnalysis` with `code: Vec<CodeSpan>` and
`CodeSpan { span: Range<usize>, prefixes: Vec<Range<usize>> }`, retaining full
source spans and original-byte container-prefix exclusions. Emit MD038 diagnostics
through the same dispatcher. Code-span data is not an edit authority.

- [x] Record pinned code spans with varying backtick lengths, balanced/unbalanced
  padding, all-space bodies, meaningful spaces/tabs/Unicode whitespace, literal
  leading/trailing backticks, newlines, quotes/lists, links/images, tables,
  comments/HTML/math, metadata, disabled endpoints and warnings.
  Run the oracle's `--record --source-only --rule MD038` mode.
  Expected: source-only receipt with no coverage claim.
- [x] Write `code_span_findings_preserve_literal_code_value`:
  `` `  text  `\n `` stays exact with no edits and reports byte spans `1..3`
  and `7..9`, both with context `` `  text  ` ``.
  Exactly one balanced padding space remains valid. All-space code is preserved.
  Assert stable second-call output/findings under enabled fixes.
- [x] Run `mise exec -- cargo test --test md038`.
  Expected: missing MD038 findings.
- [x] Derive first/last code-data segments and syntax padding from original
  source-backed code spans, excluding physical container prefixes without
  inventing edit offsets. Follow `lib/md038.mjs`'s backtick exception and
  conditional padding removal when selecting diagnostic ranges, not when editing.
  Preserve raw token context, upstream newline normalization and 30-unit head/tail
  ellipsizing independently of MD037's masked display text.
- [x] Add this quoted multiline literal, with findings on lines 1/2 at columns
  4/7, length 2:
  ```text
  > `  first
  > last  `
  ```
  The normalized context is
  `` `  first > last  ` ``. Cover list indentation, nested image labels, tabs and
  non-BMP comment text, and test code bytes unchanged with MD013/MD049/MD050 active.
- [x] Run MD038/shared/style suites and native oracles, full Rust tests/fmt/clippy.
  Expected: all pass, with no code/table/link modifications or source-map regressions.
- [x] Update truthful coverage/evidence/roadmap and commit.

## Task 3: Link-label whitespace and batch review

Files: create `src/inline_spaces/md039.rs`, `tests/md039.rs` and MD039 receipt;
extend the oracle, final registration, evidence/coverage/help, README, inventory,
this plan and roadmap.

Interfaces: extend `InlineAnalysis` with `labels: Vec<LinkLabel>` and
`LinkLabel { span: Range<usize> }`, including brackets but not the following
reference/resource. Keep actual link labels distinct from an image's own label,
destinations and reference identifiers; no synthetic label grants edits.

- [x] Record full/inline/collapsed/shortcut links, nested inline content, images,
  resolved/undefined references, tabs/Unicode/newlines, quoted labels, HTML/math,
  tables, metadata, disabled endpoints, aliases and warnings.
  Run the oracle's `--record --source-only --rule MD039` mode.
  Expected: pinned receipt only.
- [x] Write `link_label_findings_do_not_trim_indivisible_links`:
  `[ label ](url)\n` remains exact with no edits and reports spans `1..2` and
  `7..8`, context `[ label ]`. Image-only labels do not produce MD039 findings.
  `[\n label\n](url)\n` reports twice on line 2 with no invented nonempty range.
- [x] Run `mise exec -- cargo test --test md039`.
  Expected: missing MD039 findings.
- [x] Implement label endpoint whitespace selection under `lib/md039.mjs`.
  Physical newlines can require line-only findings on the adjacent content line;
  horizontal-space findings retain exact original-byte spans. Collapse context
  whitespace and apply the pinned start/end ellipsis policy.
- [x] Add `alpha beta [ label ](url) gamma delta\n` with MD013 width 20: expected
  `alpha beta\n[ label ](url) gamma\ndelta\n`, unchanged link bytes, final MD039
  findings on line 2, original edit coordinates and identical second-call results.
  Add combined spacing/marker/MD026/separator cases and CLI check/write/error/
  warning-only/no-fix behavior; preserve literal code, labels and table bytes.
- [x] Run all three spacing/style/review suites, all three raw/final native oracles,
  full Rust tests/fmt/clippy and affected MD013/MD026 comparisons.
  Expected: all pass; findings and output settle in one public invocation.
- [x] Update coverage/evidence/roadmap and commit. Request one independent review
  of this whole batch, including every Review Focus and ledger ruling; fix only
  Critical/Important findings in one RED-to-GREEN pass with a green full suite.
- [x] Retain all rulings in this tracked plan, complete the batch and continue
  the roadmap without merging, pushing or routine approval questions.

Status: Task 1 complete: 412 Rust tests across 55 targets, fmt/clippy/diff clean;
915 MD037, 1,145 MD049, 1,145 MD050 and 180 combined native observations pass.
Task 2's five tests, 1,203 ordinary pinned observations and three explicit
upstream-error policy cases pass. Both spacing/style oracles pass; fmt/clippy/diff
are clean. Fresh full suite passes 417 tests across 56 targets after the indexed
prefix lookup. Task 2 is complete. Task 3's seven tests and 957 successful pinned
observations pass; 42 upstream exceptions and six Unicode policy observations
are retained separately. All spacing/style oracles plus MD013/MD026 pass;
fmt/clippy/diff clean. Pre-review full suite passes 424 tests across 57 targets;
coverage is 44/53. One independent review found two Important issues and no
Critical/minor findings. Both reproducing tests failed before the single fix pass
and pass after it. All spacing/style/MD013/MD026 oracles pass after the fixes;
the fresh post-fix whole suite passes 426 tests across 58 targets. Final fmt/clippy/
diff checks are clean. No second review was dispatched. The dense 8,000-span
release regression is restored to baseline (5.321s vs 5.458s); code/source and
diagnostics remain exact. The batch is complete; keep the branch/worktree and
continue the roadmap without merging or pushing.

## Retained execution rulings

- Preserve post-resolution data coalescing inside attention/labels, not every
  unmatched delimiter run. Cost if wrong: MD037 pairing/scope findings, not edits.
- Store well-formed UTF-8 display contexts plus optional raw UTF-16 units in
  receipts. Cost if wrong: context display parity, not source ranges/content.
- Exclude only mandatory container prefixes from code records; keep extra
  indentation as data. Cost if wrong: code-whitespace ranges, not code edits.
- Use valid physical data-space ranges where upstream MD038 throws. Cost if
  wrong: documented diagnostic range differences on upstream-crashing inputs.
- Select full JS horizontal whitespace for MD039 instead of copying RE2JS's
  Unicode wrong-line behavior. Cost if wrong: documented coordinate differences,
  never link-label edits.
- Keep raw container prefixes in label contexts/endpoint tests, unlike code data.
  Cost if wrong: label context/endpoint classification, not source edits.
- Add private `InlineFields` selection to the snapshot interface so style-only
  consumers skip unused whitespace records. Cost if wrong: adapter maintenance or
  missing records; all spacing/style comparisons guard the change.
- Keep automatic whitespace repairs outside this batch, as the reviewer set aside.
  Cost if wrong: actionable findings remain instead of modifying meaning.
- Keep valid native coordinates instead of upstream invalid MD038/Unicode MD039
  anchors, as the reviewer set aside. Cost if wrong: intentional parity differences.
- Defer pre-existing dense-line parser costs, as the reviewer set aside; remove
  only this batch's regression. Cost if wrong: baseline slowness remains visible.
- Leave other built-ins, external formatters and cross-platform runtime validation
  outside this batch, as the reviewer set aside. Cost if wrong: explicit incomplete
  rule/release coverage remains rather than an unsupported completion claim.

Deferred minors: none.

Planning check: pinned source and runtime confirm the three literal diagnostic
examples, byte/UTF-16 ranges, multiline context and line-only label findings.
These are reference observations, not native application tests. All tasks use
the same source snapshot and dispatcher; style compatibility remains a global
gate through the shared-model move. Existing coverage is 41/53, with 407 tests.
