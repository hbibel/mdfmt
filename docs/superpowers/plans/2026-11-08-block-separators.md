# Block Separators Implementation Plan

> For agentic workers: REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax for tracking.

**Goal:** Implement MD022, MD031 and MD032 findings and conditional blank-line
insertions without changing heading text, code, list tightness or container ownership.

**Architecture:** A shared read-only separator model drives three rule modules.
Merge compatible insertions at the same boundary, preserve physical directive
targets, and validate both individual insertions and their combined native tree.
Run the separator stage after fence-style repairs and before MD058/MD003.

**Tech Stack:** Existing Rust toolchain, native source/parser/configuration/edit
pipeline, pinned markdownlint 0.42.0 development oracle. No runtime dependency added.

**Spec:** `docs/superpowers/specs/2026-10-21-mdfmt-design.md`

## Decisions and assumptions

- Continue inline under the user's existing instruction to finish the roadmap
  autonomously. This is a bounded batch, not a claim that the roadmap is complete.
- Source-preserving insertions are permitted; converting tight lists to loose
  lists, manufacturing container markers or editing metadata/code/table bytes is
  not. Unsafe cases remain findings.
- Reuse `parse::diagnostic_parse` for diagnostics and native ownership for edits.
  Recovered diagnostic blocks do not independently authorize edits.
- Share upstream `isBlankLine` semantics with MD058 by extracting its existing
  helper. Comment-only and quote-prefix-only neighboring lines count as blank.
- Context strings use upstream's comment-masked lines, while token selection and
  byte spans remain raw. Extract MD060's existing masker as
  `parse::comment_masked_source(&Source, &ParsedDocument) -> Source`; preserve its
  UTF-16 length semantics and existing width regressions.
- Insert the document's preferred newline and a verified continuation prefix.
  Never repeat a list marker or synthesize a new list item as a blank prefix.
- Interior separator requirements above an enabled MD012 maximum are only fixed
  up to that maximum; report the unresolved separator rule. This avoids spacing
  ping-pong without silently disabling either configured rule.
- Preserve any physical line targeted by a next-line/configuration override.
  Do not insert between the directive and its target, even for unrelated rules.
- Count virtual boundary blanks arithmetically. Bound separator insertion bytes
  to 1 MiB per stage; oversized requirements remain diagnostics without allocating
  the requested replacement. This is resource safety, not a changed rule option.
- The existing roadmap is the approved design. No new user-facing configuration,
  unsafe repair policy or runtime language is introduced.

## Global Constraints

- Must respect the effective supported markdownlint configuration.
- Must preserve content and Markdown meaning rather than force an unsafe fix.
- Must preserve tables. Violations involving tables may remain unresolved.
- Must report remaining violations when the effective configuration flags them.
- Must be idempotent: formatting the result again produces identical output.
- Keep the existing sibling worktree and shared parent OpenCode configuration.
- Record only Important/Critical findings in the roadmap, with reproducing tests.

## Review Focus

- Inserting blanks into nested or lazy containers must not loosen lists, end a
  quote, change code ownership or move a next-line directive's original target.
- Setext headings, heading-level option arrays and metadata-adjacent headings need
  physical line boundaries rather than rendered text or native child ranges.
- Unterminated fences, closing-looking code and empty/list-contained fences must
  retain exact code bytes and report the pinned opening/ending line only.
- MD032 checks outermost lists even inside quotes, but uses their visual end;
  trailing container prefixes, definitions and recovered HTML are not interchangeable.
- Shared insertion boundaries and conflicting MD012/MD022 widths must settle in
  one invocation alongside MD003, MD013, MD026, MD048 and MD058.

## Task 1: Heading separators and shared insertion safety

Files: create `src/blank_lines.rs`, `src/blank_lines/md022.rs`,
`tests/md022.rs`, `tools/parser-oracle/block-separators.mjs`, its MD022 receipt,
`src/parse/comments.rs` and `docs/compatibility/block-separators.md`;
modify `src/md058.rs`, `src/md060/comments.rs`, `src/md060.rs`, `src/parse.rs`, `src/lib.rs`,
CLI help, coverage tests, README, inventory and roadmap.

Interfaces:

- `blank_lines::is_blank(&str) -> bool` is the extracted MD058 helper.
- `blank_lines::Insertion { at: usize, prefix: String, missing: usize,
  rule: &'static str }` describes a source boundary and missing blank lines.
- `blank_lines::propose(&Source, &ParsedDocument, &DocumentConfig) -> Vec<Edit>`
   collects rule insertions, checks permissions and MD012 budget, merges compatible
   prefix alternatives by maximum missing count, and verifies native preservation.
- `blank_lines::diagnose(&Source, &ParsedDocument, &DocumentConfig) -> Vec<Diagnostic>`
  collects enabled final-output findings; line-only spans are byte-valid line starts.
- Each rule module produces `insertions(...) -> Vec<Insertion>` and
  `diagnose(...) -> Vec<Diagnostic>` with the same three reference parameters.

- [x] Record pinned MD022 observations for ATX/setext/closed headings, levels 1–6,
  scalar/array/negative options, `include_front_matter`, BOM/mixed terminators,
  comments, quotes/lists, HTML, math, directives and warnings.
  Run: `node tools/parser-oracle/block-separators.mjs --record --source-only --rule MD022`.
  Expected: receipt written with no native implementation claim.
- [x] Add failing `heading_separators_preserve_caption_and_settle_once`:
  `before\n# Heading\nafter\n` has Above/Below
  findings with fixes disabled, and becomes `before\n\n# Heading\n\nafter\n`
  with MD022 alone. Both findings refer to line 2, have no invented range, and
  contain expected 1/actual 0 plus pinned context. A second format has no edits.
  Cover `lines_above: [2,0]`, omitted array entries defaulting to 1, negative
  `-1` requirements disabling that side, and metadata excluded unless requested.
  The fixed-output assertions are:
  ```rust
  assert_eq!(first.output, "before\n\n# Heading\n\nafter\n");
  assert!(first.diagnostics.is_empty());
  assert_eq!(again.output, first.output);
  assert!(again.edits.is_empty());
  ```
- [x] Run: `mise exec -- cargo test --test md022`.
  Expected: missing MD022 findings/insertions, not fixture or configuration errors.
- [x] Extract the unchanged blank classifier, implement MD022's line counts and
  shared insertion safety, extract the unchanged MD060 comment masker for
  diagnostic contexts, and integrate the stage before MD058/MD003. Validate
  each candidate and the merged batch with `parse::equivalent`; reject uncertain
  prefixes, metadata edits, list-tightness changes and directive-target shifts.
- [x] Add literal preservation tests for `> before\n> # H\n> after\n`, list-contained
  headings, setext headings, EOF, an unrelated disabled-next-line target, and
  interior MD022 count 2 against MD012 maximum 1. Assert source-backed findings,
  original edit coordinates, unchanged protected bytes and second-call stability.
  Pin masked comment contexts independently; never use masked UTF-8 offsets as
  original-source spans or feed masked quotes into raw HTML tokenization.
- [x] Run: `mise exec -- cargo test --test md022 --test md058 --test md003 --test md012`;
  `mise exec -- cargo build`; `node tools/parser-oracle/block-separators.mjs --rule MD022`;
  full Rust suite, fmt and clippy.
  Expected: all pass; native oracle checks raw diagnostics and preservation-aware
  fixed output rather than requiring unsafe upstream edits.
- [x] Update truthful MD022 evidence/coverage and commit the tested feature.

Task 1 verified: 358 Rust tests across 47 targets; fmt/clippy clean. All 657
MD022 raw/fixed comparisons pass, plus 1,206 MD003, 125 MD012, 207 MD058,
1,133 MD060/734 widths and 602 MD033 comparisons. Native-only regressions cover
bounded huge counts, unsigned counts, atomic allocation admission and shared gaps.

## Task 2: Fence separators

Files: create `src/blank_lines/md031.rs`, `tests/md031.rs` and MD031 receipt;
modify the oracle, shared module registration, coverage/evidence/help and roadmap.

Interfaces: consume Task 1's `Insertion`, blank classifier and shared stage;
produce the two per-rule functions defined in Task 1.

- [x] Record pinned MD031 observations for both fence styles, longer closers,
  indented-code exclusions, unclosed/empty fences, list/quote containment,
  `list_items: false`, HTML/math exclusions, comments, directives and severity.
  Run the oracle's `--record --source-only --rule MD031` mode.
  Expected: pinned receipt only.
- [x] Add failing `fence_separators_preserve_code_and_settle_once`:
  `before\n```rust\nx\n```\nafter\n` reports line 2
  and line 4 with fixes disabled, then gains one blank above and below without
  changing `x\n`. `list_items: false` excludes list-contained fences only.
  ```rust
  assert_eq!(first.output, "before\n\n```rust\nx\n```\n\nafter\n");
  assert!(first.diagnostics.is_empty());
  assert_eq!(again.output, first.output);
  assert!(again.edits.is_empty());
  ```
- [x] Run: `mise exec -- cargo test --test md031`.
  Expected: missing MD031 findings.
- [x] Implement fenced-block selection with tab-expanded original-byte mappings,
  pinned first/last physical lines and neighbor blank checks. Reuse Task 1's
  proposal guard; never insert inside an unclosed fence or alter code bytes.
- [x] Add regressions for a quote nested in a list, closing-looking literal code,
  CRLF/BOM, an active next-line target at an insertion boundary, MD048 fence conversion
  and MD013 prose reflow. Assert stable final positions and byte preservation.
- [x] Run focused MD031/MD048 tests, the MD031 oracle, full Rust tests/fmt/clippy.
  Expected: all pass, including unchanged code snapshots and second-call output.
- [x] Update truthful MD031 evidence/coverage and commit.

Task 2 verified: 362 Rust tests across 48 targets; fmt/clippy clean. All 264
MD031 and 657 MD022 raw/fixed comparisons pass; 363 MD014 and 220 MD048
comparisons retain literal-code preservation. Coverage is 38/53.

## Task 3: Outermost-list separators

Files: create `src/blank_lines/md032.rs`, `tests/md032.rs` and MD032 receipt;
create `src/parse/list_flow.rs`; modify `src/parse.rs`, the oracle, shared
registration, coverage/evidence/help and roadmap.

Interfaces: consume Task 1's shared API; produce the same per-rule functions.
Use source-mapped diagnostic list ownership, not raw marker regexes.
The read-only adapter produces `parse::diagnostic_lists(&Source) ->
Option<(Source, ParsedDocument, Vec<usize>)>`; its byte map composes synthetic
container boundaries with existing tab expansion. No formatter edit consumes it.

- [x] Record pinned MD032 observations for ordered/unordered/mixed lists,
  nested lists, quote-contained outer lists, lazy continuations, empty items,
  definitions, trailing blanks/prefixes, HTML/math, metadata, directives and warnings.
  Run the oracle's `--record --source-only --rule MD032` mode.
  Expected: pinned receipt only.
- [x] Add failing `outer_list_separators_preserve_tightness_and_settle_once`:
  `before\n- item\n# After\n` reports list lines 2
  twice (above and below), then becomes `before\n\n- item\n\n# After\n` with
  MD032 alone. A nested child list does not gain a separate separator requirement.
  ```rust
  assert_eq!(first.output, "before\n\n- item\n\n# After\n");
  assert!(first.diagnostics.is_empty());
  assert_eq!(again.output, first.output);
  assert!(again.edits.is_empty());
  ```
- [x] Run: `mise exec -- cargo test --test md032`.
  Expected: missing MD032 findings.
- [x] Implement outermost-list selection and visual-end recovery, using pinned
  non-content-token boundaries. Keep unsafe lazy/ownership ambiguities diagnostic-only.
- [x] Add regressions for quote-only trailing lines, list-contained fences/tables,
  repeated empty markers, references and definitions, tabbed uncertain prefixes,
  neighboring enabled/disabled rules, original-coordinate edits and idempotence.
- [x] Run focused MD032/list-rule tests, the MD032 oracle, full Rust tests/fmt/clippy.
  Expected: all pass, with list start values, item ownership and tightness unchanged.
- [x] Update truthful MD032 evidence/coverage and commit.

Task 3 verified: 368 Rust tests across 49 targets; fmt/clippy clean. All 345
MD032 raw/fixed comparisons pass, with 657 MD022 and 264 MD031 cases unchanged.
List-rule focused suites remain green. Coverage is 39/53; batch review is pending.

## Task 4: Interactions and independent batch review

Files: create `tests/block_separators.rs`; modify `src/blank_lines.rs` for
compatible prefix selection; extend per-rule evidence, this plan and the roadmap.
Do not count unimplemented rule repairs as coverage.

- [x] Add literal combined-rule cases for a heading directly after a list, a fence
   adjacent to a heading, overlapping above/below requirements, MD012 maximum 1/2
   and rejection of unsupported maximum 0,
  MD003 setext conversion exposed by separators, MD058 table neighbors and MD026
  punctuation removal followed by MD013 reflow. Add check/write/warning CLI cases.
  Watch any uncovered interaction fail before changing scheduling or safety logic.
- [x] Run the combined tests and all three raw/fixed native oracles, then full
  Rust tests/fmt/clippy and affected MD003/MD012/MD048/MD058 comparisons.
  Expected: final output and diagnostics are stable on the second call, no
  directive changes target, no table/code bytes change, and unsafe findings remain.
- [ ] Request one independent review of this entire plan's commit range, including
  every Review Focus item and ledgered decision. Fix Important/Critical findings
  in one RED→GREEN pass and rerun the complete suite; record substantial findings
  in the roadmap. Do not spend a fix pass on cosmetic issues.
- [ ] Record verification and accurate coverage, commit, and continue the remaining
  roadmap in the existing worktree without a merge/push or routine approval pause.

Status: Tasks 1–3 are implemented and verified; coverage is 39/53. Task 4's
combined interactions pass (378 Rust tests across 50 targets, fmt/clippy and all
seven affected oracles). Independent review remains pending.

Planning check: pinned runtime observations confirm the three literal diagnostic
examples and their upstream insertion locations. The shared interfaces, stage
order and preservation requirements were checked against the roadmap. These are
planning observations, not native application tests.
