# Emphasis Styles Implementation Plan

> For agentic workers: REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax for tracking.

Goal: Implement MD049/MD050 diagnostics and conditional paired delimiter changes
without altering inline content, nesting, links, code or table bytes.

Architecture: One source-mapped selector serves emphasis and strong styles.
Each node produces paired marker edits; admit only complete permitted pairs
whose batch reparses preserve structure and protected content, with bounded
individual fallback for rejected batches of at most 16 pairs.
Run before prose reflow and heading suffix/style edits so final diagnostics and
wrapping use the formatted markers in one invocation.

Tech stack: Existing Rust toolchain and parser/configuration/edit pipeline;
pinned markdownlint 0.42.0 development oracle, no new runtime dependency.

Spec: `docs/superpowers/specs/2026-10-21-mdfmt-design.md`

## Decisions and assumptions

- Continue inline under the user's standing autonomous roadmap instruction.
  This plan covers two of the remaining fourteen rules, not the whole roadmap.
- Use `lib/md049-md050.mjs` as the diagnostic reference: separate per-rule
  consistency, first token including disabled occurrences, and ASCII `\w`
  intraword exclusion when underscore is requested.
- Marker swaps preserve byte length. Verify lexical opening/closing runs from
  original source rather than trusting rendered child ranges or global searches.
- Respect both ends' inline permissions; never change only one delimiter.
- Preserve links/images and tables exactly, even when their contained emphasis
  is diagnostic. Do not authorize whole protected regions for marker edits.
- Preserve ambiguous recovered HTML/container ownership with findings. A read-only
  diagnostic view never grants edit authority.
- Validate preferred batches together to avoid one full reparse per pair. For
  rejected batches of at most 16 pairs, try safe pairs and validate their combined
  output; larger rejected batches remain findings. Preservation and bounded work
  are more important than edit coverage.
- Account for pinned micromark's UTF-16-unit flanking behavior at astral neighbors;
  native AST equality alone cannot authorize changes anywhere in the affected
  attention subtree.
- Adjacent rules can remove each other's intraword exclusions. Resolve only the
  marker stage's dependencies, at most 16 rounds with original-byte composition;
  withhold atomically if it does not settle. This is not a generic repeated-pass
  engine for all formatter stages.

## Global Constraints

- Must respect the effective supported markdownlint configuration.
- Must preserve content and Markdown meaning rather than force an unsafe fix.
- Must preserve inline links without conversion to reference links.
- Must preserve tables. Violations involving tables may remain unresolved.
- Must report remaining violations when the effective configuration flags them.
- Must be idempotent: formatting the result again produces identical output.
- Use the existing sibling worktree, parent configuration and pinned Rust toolchain.
- Record Important/Critical findings in the roadmap with reproducing tests.

## Review Focus

- Triple/nested/adjacent runs must preserve the exact emphasis/strong nesting,
  including literal delimiter characters omitted from native child ranges.
- A multiline node with a disabled closing marker must never receive a one-ended
  edit; consistency must follow token order rather than enabled finding order.
- Underscore intraword suppression uses ASCII word characters, not Unicode word
  classes or native parser eligibility guesses.
- HTML recovery, tab expansion and comments can change diagnostic ownership and
  columns; raw link/code/table/metadata bytes remain protected.
- Marker swaps must settle once with MD013, MD026, MD003 and the separator stages;
  no reparse may reveal another safely actionable change on the second invocation.

## Task 1: Shared selection and MD049

Files: create `src/emphasis_styles.rs`, `tests/md049.rs`,
`tools/parser-oracle/emphasis-styles.mjs`, `tools/parser-oracle/md049-observations.json`
and `docs/compatibility/emphasis-styles.md`; modify `src/lib.rs`, CLI help, README,
coverage assertions, inventory and roadmap.

Interfaces:

- `emphasis_styles::propose(&Source, &ParsedDocument, &DocumentConfig) -> Vec<Edit>`.
- `emphasis_styles::diagnose(&Source, &ParsedDocument, &DocumentConfig) -> Vec<Diagnostic>`.
- Private selector produces opening/closing original-byte ranges, native owner
  if available, rule ID, actual style and expected style. No public API added.

- [x] Record source-only MD049 observations for consistent/asterisk/underscore,
  intraword ASCII/Unicode, nested/triple/adjacent runs, escapes/entities, multiline
  quotes/lists, HTML/comments, math, links/tables/code, metadata, BOM/mixed endings,
  aliases, disabled nodes, next-line directives and warnings.
  Run `node tools/parser-oracle/emphasis-styles.mjs --record --source-only --rule MD049`.
  Expected: pinned receipt only, not implemented-rule coverage.
- [x] Write `emphasis_pairs_change_only_markers_and_settle_once`:
  `_one_ and *two*\n` with MD049 asterisk becomes `*one* and *two*\n`;
  no-fix diagnostics cover columns 1/5 with length 1 and expected/actual styles.
  Applied edits address original bytes, second call has no edits or diagnostics.
  Write preservation tests for `[a _b_](url)`, a table cell, literal code and
  a disabled closing line of multiline emphasis; findings remain where enabled.
- [x] Run `mise exec -- cargo test --test md049`.
  Expected: missing MD049 diagnostics/repairs, not fixture/configuration errors.
- [x] Implement source-backed delimiter selection and per-rule style resolution.
  Map diagnostic offsets through existing parser views; handle raw ownership
  separately for edit authorization. Validate complete pairs and their merged
  batch using existing source/structure/protected-byte checks.
- [x] Add literal regressions for `a*one*b` under underscore (no finding/edit),
  non-ASCII neighbors, disabled first-style anchors, nested/triple runs, adjacent
  runs, active next-line targets, recovered HTML, tabs, BOM and final UTF-16 spans.
- [x] Run `mise exec -- cargo test --test md049`, the MD049 native oracle, full
  Rust tests/fmt/clippy. Expected: pinned findings and stable preservation-aware
  output, no link/table/code/metadata changes.
- [x] Update truthful coverage/evidence/roadmap and commit.

Task 1 verified: 394 Rust tests across 52 targets; fmt/clippy clean. All 540
MD049 pinned/native comparisons pass, including supplementary-scalar safeguards;
1,206 MD003, 288 MD013 and 440 MD026 comparisons remain green. Coverage is 40/53.

## Task 2: MD050 and shared-rule interactions

Files: modify `src/emphasis_styles.rs`, oracle and evidence; create `tests/md050.rs`,
`tools/parser-oracle/md050-observations.json` and
`tools/parser-oracle/emphasis-combined-observations.json`; update registration/help,
coverage assertions, README, inventory and roadmap.

Interfaces: consume Task 1's shared functions and selector; MD050 uses strong
nodes and two-character sequences, with its own consistency anchor and options.

- [x] Record MD050 observations with the same input classes and independent
  MD049/MD050 settings, including `***both***` and mixed nesting.
  Run the oracle's `--record --source-only --rule MD050`.
  Expected: pinned receipt only.
- [x] Write `strong_pairs_change_only_markers_and_settle_once`:
  `__one__ and **two**\n` with MD050 asterisk becomes `**one** and **two**\n`;
  raw findings cover columns 1/6 with length 2. Second call is unchanged.
  Test MD049 underscore plus MD050 asterisk on `***both***\n` yielding
  `_**both**_\n`, with identical parsed nesting and protected bytes.
- [x] Run `mise exec -- cargo test --test md050`.
  Expected: missing MD050 findings/repairs.
- [x] Register MD050 in the shared selector and diagnostics, retaining paired
  permission and preservation checks. Add no generic repeated-pass engine.
- [x] Add combined literal cases for heading punctuation, setext styles,
  configured-width prose reflow and heading/fence/list separators. Check original
  edit application and stable final positions/output/findings on a second call.
  Add CLI check/write and warning-only no-fix cases using both rule aliases.
- [x] Run both focused suites and native oracles, full Rust tests/fmt/clippy and
  affected MD003/MD013/MD026 comparisons.
  Expected: all pass, with unsafe fixes retained as findings.
- [ ] Update coverage/evidence/roadmap and commit; request one independent review
  of the whole emphasis-style batch. Fix Important/Critical findings in one
  RED-to-GREEN pass with a green full suite, and record them in the roadmap.
- [ ] Retain ledger rulings in the tracked plan, complete the batch, and continue
  the roadmap without merging, pushing or routine approval prompts.

Status: both rules are implemented; Task 2's independent batch review is pending.
Before review: 402 Rust tests across 53 targets; fmt/clippy/diff clean. Both
540-case per-rule oracles and 180 combined cases pass, as do 1,206 MD003,
288 MD013 and 440 MD026 comparisons. Coverage is 41/53. Release probes for 400
and 800 pairs take 0.183s and 0.518s respectively, with the expected literal output.

Planning check: reviewed the shared pinned upstream source and runtime findings
for consistency, intraword exclusions and nested/triple sequences. Those are
reference observations, not native formatter tests. The two tasks share one
selector, use the same interfaces and preserve the approved roadmap boundaries.
