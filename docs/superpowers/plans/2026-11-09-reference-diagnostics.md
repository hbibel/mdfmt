# Reference and Link-Style Diagnostics Implementation Plan

> For agentic workers: REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax for tracking.

Goal: Implement independent MD052/MD053 reference diagnostics and MD054 link-style
diagnostics without inventing destinations or discarding link/definition content.

Architecture: Build a source-mapped, read-only reference model from diagnostic
inline scopes and definition regions. Keep helper-compatible keys separate from
native parser reference identity. Share one model between enabled reference rules;
add style records only with MD054's failing tests, and dispatch final-output
findings after existing transformations. This batch adds no source-edit proposer.

Tech stack: Existing Rust toolchain/parser/configuration/source model; pinned
markdownlint development oracle. Task 3 adds `url` 2.5 for native WHATWG-style
absolute-URL validation rather than a home-grown URL regex. Keep Cargo.lock tracked.

Spec: `docs/superpowers/specs/2026-10-21-mdfmt-design.md`

## Decisions and assumptions

- Continue inline under the standing autonomous roadmap instruction, with one
  independent review of this batch and one regression-first Important/Critical
  fix pass. Reuse the existing sibling worktree; no merge/push or routine prompts.
- Keep all link/definition bytes. MD052 cannot invent destinations; MD053 unused
  definitions may carry author content; MD054 upstream fixes can discard labels
  and titles. Even identical duplicate-definition removal is outside this batch's
  edit authority. Coverage means implemented diagnostics, not blanket fixability.
- Use `docs/compatibility/reference-rule-audit.md` and
  `docs/compatibility/link-style-audit.md` as the source-reviewed handoff. Their
  observations are not native implementations or tests.
- MD052/MD053 work with only that rule enabled. Their oracle records both direct
  pinned runtime results and parser-forced observations using an inert micromark
  rule, because pinned parser:none cache behavior otherwise reports false clean.
- Preserve the helper's lowercase/JS-whitespace keys, raw definition spelling and
  undefined-reference data-token projection; do not substitute native identifier
  folding, entity decoding or author intent.
- MD054 classifies syntax, including empty inline resources, and uses normalized
  resolved-reference identity. Do not reproduce the audited false-clean uppercase/
  entity-reference or empty-destination bugs. Retain these as explicit native
  policy cases beside direct pinned outcomes, not ordinary parity claims.
- The prior inline batch's valid physical ranges, selective field extraction and
  cached prefix work remain binding. No full-document reparse per reference.

## Global Constraints

- Must respect the effective supported markdownlint configuration.
- Must preserve content and Markdown meaning rather than force an unsafe fix.
- Must preserve inline links without conversion to reference links.
- Must preserve tables. Violations involving tables may remain unresolved.
- Must report remaining violations when the effective configuration flags them.
- Must be idempotent: formatting the result again produces identical output.
- Use the pinned toolchain and parent OpenCode configuration; never read .env.
- Keep coverage truthful; record Important/Critical findings with reproductions.

## Review Focus

- Native reference identity is not the helper key: case folding, escapes/entities,
  JS whitespace, footnote carets and quote-prefix projection must not silently
  resolve missing references or hide used definitions.
- Adjacent undefined labels must distinguish full/collapsed/shortcut syntax
  without pairing across escaped brackets, code, HTML, math or container scopes.
- Disabled definitions still resolve uses; unused first definitions and later
  duplicates are separate findings. Literal code/metadata and definitions remain
  byte-exact even when every reference rule is enabled.
- Empty resources, titles, images and differently cased reference labels must
  keep their syntax/content distinctions under MD054; fixes must not leak from
  upstream proposals into native edit authorization.
- Dense same-line labels and long multiline containers must not trigger per-use
  whole-document/whole-line work or invalid BOM/CRLF/UTF-16 ranges and permissions.

## Task 1: Source-backed reference model and MD052

Files: create `src/parse/references.rs`, `src/parse/references/scanner.rs`, `src/reference_rules.rs`,
`src/reference_rules/md052.rs`, `tests/md052.rs`,
`tools/parser-oracle/reference-diagnostics.mjs`, MD052 receipts and
`docs/compatibility/reference-diagnostics.md`; modify `src/parse.rs`, `src/lib.rs`,
`src/cli.rs`, README, inventory/coverage assertions and roadmap.

Interfaces:

- `parse::reference_analysis(&Source, &ParsedDocument) -> ReferenceAnalysis` is
  read-only and does not consult enabled states to decide reference ownership.
- `ReferenceAnalysis { definitions: Vec<ReferenceDefinition>, uses: Vec<ReferenceUse> }`.
- `ReferenceDefinition { key: String, span: Range<usize> }` retains every definition
  in source order, including duplicate/footnote records. First-definition selection
  is a consumer policy; disabled lines do not erase definitions.
- `ReferenceUse { key: String, span: Range<usize>, shortcut: bool }` preserves raw
  source coordinates and helper-compatible keys for resolved and shim-equivalent
  undefined references. Inline resources are not reference uses.
- `reference_rules::diagnose(&Source, &ParsedDocument, &DocumentConfig) -> Vec<Diagnostic>`
  obtains one diagnostic view/snapshot when an enabled consumer needs it.
- `md052::diagnose(&Source, &Source, &DocumentConfig, &ReferenceAnalysis)` consumes
  original source, comment-masked display source, effective states and the snapshot.
  It returns diagnostics; no candidate edit is produced.

- [x] Record direct and parser-forced MD052 observations for full/collapsed/
  shortcut links/images; ignored-label arrays; adjacent/nested/escaped brackets;
  entities, emphasis and code in unresolved labels; normalized/resolved keys;
  footnotes, quotes/lists/tabs, HTML/math, metadata, Unicode/BOM/CRLF, directives
  and warnings. Include `[ß]` against `[SS]: /url` and quote-prefix projections.
  Run `node tools/parser-oracle/reference-diagnostics.mjs --record --source-only --rule MD052`.
  Expected: separately labeled pinned evidence, no native coverage claim.
- [x] Write `missing_references_report_without_fabricating_destinations`:
  `[label][missing]\n` stays exact with fixes true/false, no edits, one MD052
  finding at `0..16`, context `[label][missing]`, detail
  `Missing link or image reference definition: "missing"`. The second invocation
  has identical output/findings. `[missing]\n` is quiet by default and reported
  under `shortcut_syntax: true`; a valid definition resolves independently of
  other enabled rules and its own disabled line.
- [x] Run `mise exec -- cargo test --test md052`.
  Expected: missing MD052 findings, not import/configuration/receipt errors.
- [x] Implement the reference snapshot using source-backed inline scopes and
  definition regions. Scan each scope once and reuse indexed exclusions/prefixes;
  preserve original definitions and container context in image-label recovery.
  Port the pinned shim's adjacent-label grouping and data/line-ending projection,
  not a document-wide bracket regex or native decoded text/identifier alone.
- [x] Implement configured MD052 finding selection, ignored-label exact matching,
  optional shortcuts and first-physical-line context/ranges from masked display
  columns. Keep original valid byte boundaries and source-order line permissions.
- [x] Add a dense same-line missing-label CLI budget regression with a substantial
  trailing text segment; cover malformed labels, 999/1,000 UTF-16-unit labels,
  nested image descriptions, disabled targets, warning aliases and line-only
  continuation hazards without per-reference full-document parsing.
- [x] Run MD052 tests/native oracle, existing spacing/style/review suites and
  native comparisons, then full Rust tests/fmt/clippy/diff.
  Expected: parser-forced diagnostic parity, explicit direct-cache difference,
  exact source/idempotence and no regression from the 426-test native baseline.
- [x] Update coverage/evidence/roadmap and commit.

## Task 2: Unused and duplicate definition diagnostics

Files: create `src/reference_rules/md053.rs`, `tests/md053.rs` and MD053 receipts;
extend shared oracle/dispatcher; update help, evidence, coverage, inventory, README,
roadmap and this plan.

Interfaces: consume Task 1's `ReferenceAnalysis`, `ReferenceDefinition` and
`ReferenceUse` unchanged. `md053::diagnose(&Source, &Source, &DocumentConfig,
&ReferenceAnalysis) -> Vec<Diagnostic>` reads original and masked display sources.

- [x] Record direct/parser-forced observations for used/unused definitions,
  duplicate same/different destinations/titles, ignored-definition exact matching,
  case/whitespace/escape/entity identity, footnotes, multiline definitions and
  quote/list physical lines, comments/metadata, directives, warnings and aliases.
  Run the oracle's `--record --source-only --rule MD053` mode.
  Expected: retained parser-cache distinction, no implementation claim.
- [x] Write `unused_and_duplicate_definitions_remain_source_content`:
  `[unused]: /url\n` stays exact, no edits, one finding at `0..14` with context
  `[unused]: /url` and detail `Unused link or image reference definition: "unused"`.
  `[id]: /url\n[id]: /url\n\n[text][id]\n` stays exact and reports only the second
  definition at `11..21`, detail `Duplicate link or image reference definition: "id"`.
  Assert stable output/findings under both fix settings and a second invocation.
- [x] Run `mise exec -- cargo test --test md053`.
  Expected: missing MD053 findings.
- [x] Implement first-definition/duplicate bookkeeping and use-set membership
  independently of line enablement. Respect ignored definitions without silently
  normalizing option-array strings. Follow pinned whole-physical-line ranges and
  head-ellipsized contexts, mapping masked UTF-16 display units back to valid bytes.
  Report unused-first and duplicate-later independently; never delete content.
- [x] Add quoted/listed/multiline and footnote definitions, disabled-first-use/
  definition cases, non-BMP comments and combined MD052/MD053/MD013/MD026 scenarios.
  Assert no reference/definition bytes change and final coordinates/idempotence hold.
- [x] Run both reference suites/oracles, relevant prior suites and full Rust
  tests/fmt/clippy/diff. Expected: all pass; MD053 alone reports normally despite
  upstream's direct false-clean cache behavior.
- [x] Update coverage/evidence/roadmap and commit.

## Task 3: Source-preserving link-style diagnostics and batch review

Files: create `src/reference_rules/md054.rs`, `tests/md054.rs` and MD054 receipts;
extend `src/parse/references.rs`, shared oracle/dispatcher and documentation;
add `url` in Cargo.toml/Cargo.lock; update help/coverage, README, inventory and roadmap.

Interfaces: extend `ReferenceAnalysis` with `styles: Vec<LinkStyle>` only now.
At this stage extend the private extractor signature with
`ReferenceFields { uses: bool, styles: bool }`, and update its callers together.
The dispatcher requests only enabled consumers' fields; MD054-only calls do not
scan undefined uses, and reference-only calls do not extract unused styles.
`LinkStyle { span: Range<usize>, kind: LinkKind, label: Option<Range<usize>>,
destination: Option<Range<usize>>, title: Option<Range<usize>> }` retains original
syntax fields. `LinkKind` distinguishes `Autolink`, `Inline`, `Full`, `Collapsed`,
`Shortcut`. Reference styles use resolved native ownership plus Task 1's definition
keys, not destination truthiness; literal autolinks remain distinct from bare URLs.
`md054::diagnose(&Source, &DocumentConfig, &ReferenceAnalysis) -> Vec<Diagnostic>`
emits findings without proposing conversions.

- [ ] Record all six style options independently/in combination, nested links in
  images, inline math/code/HTML, title-bearing resources, empty destinations,
  escaped/raw destination spelling, case/entity references, URL schemes/invalid
  absolutes, multiline/quoted/table source, directives and warnings. Keep audited
  false-clean cases in a native-policy section, separate from parity observations.
  Run the oracle's `--record --source-only --rule MD054` mode.
  Expected: source evidence with no native coverage claim.
- [ ] Write `disallowed_link_styles_preserve_labels_titles_and_destinations`:
  `[text][id]\n\n[id]: /url\n` under `full: false` stays exact with no edits and
  reports MD054 at `0..10`, context `[text][id]`. `[text][ID]` against the same
  definition reports the same physical span under the explicit corrected policy.
  `[label](https://example.com "title")\n` under `inline: false` stays exact,
  retains both label/title and reports a finding rather than the audited upstream
  `<https://example.com>` replacement. `[text]()` under `inline: false` reports
  its actual `0..8` resource span instead of falling into reference classification.
- [ ] Run `mise exec -- cargo test --test md054`.
  Expected: missing MD054 findings.
- [ ] Add source-backed style records and implement the six options with native
  `url::Url::parse` absolute-URL validation and pinned raw label/destination
  equality plus literal-space/angle-bracket exclusions. Preserve title/image
  exceptions, exact raw contexts and same-line ranges; multiline findings have
  a valid empty span on their opening line. No style grants edit authorization.
- [ ] Pin `[https://example.com](https://example.com)\n` under `url_inline: false`
  at `0..42`, context `[https://example.com](https://...`, unchanged source.
  Add original-byte MD013 reflow composition, all-three-rule CLI check/write/
  warning/no-fix cases, protected tables/code/labels and dense-prefix work budgets
  for both reference-only and style-only configurations.
  Assert one-invocation output/findings stability and no lost fields.
- [ ] Run all reference/spacing/style/review suites and native oracles; full Rust
  tests/fmt/clippy/diff and affected MD013/MD026 comparisons.
  Expected: green suites and truthful policy/parity separation; coverage 47/53
  only after the three native implementations and comparisons pass.
- [ ] Update evidence/roadmap/coverage and commit. Request one independent review
  over this batch's baseline through final HEAD, with every Review Focus and
  ledger ruling; fix only Critical/Important findings in one RED-to-GREEN pass
  followed by a green full suite. Do not dispatch a second review.
- [ ] Retain all rulings/evidence in this plan, remove only its completed scratch
  workspace and continue the roadmap without merging, pushing or routine questions.

Status: Task 1 complete: four MD052 tests and 2,037 paired direct/parser-forced
native comparisons pass. Full suite passes 430 tests across 59 targets;
fmt/clippy/diff clean. Prior spacing/style comparisons pass; coverage is 45/53.
Task 2 complete: five MD053 tests and 1,825 paired native comparisons pass, together
with the original 2,037 MD052 comparisons. Full suite passes 435 tests across 60
targets; fmt/clippy/diff and all prior spacing/style/MD013/MD026 comparisons pass.
Coverage is 46/53;
MD054 remains unimplemented and the batch review remains pending.
Baseline d9b4011 has 44/53 covered built-ins and 426 passing tests across 58 targets.

Planning check: literal MD052/MD053/MD054 examples and ranges were confirmed with
the pinned runtime (forced-parser mode for references). Task 1 produces the exact
definitions/uses consumed by Task 2; Task 3 adds tested style fields without
reinterpreting Task 1's helper identities. Every Review Focus has a named owning
test step. Remaining MD005/MD007/MD034/MD044/MD046/MD051, external formatters and
release hardening remain explicit roadmap work, not covered by this plan.
