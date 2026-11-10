# Literal Content Diagnostics Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement native MD046 code-block style, MD034 bare-URL and MD044 proper-name diagnostics without changing links or literal content.

**Architecture:** Add final-output diagnostic consumers with original-byte ranges and existing document permissions. MD046 uses recovered code ownership. A field-selected content snapshot supplies bare URLs and source-token-like prose/code/HTML segments for MD034/MD044, with bounded image recovery and distinct raw/source-normalized identities.

**Tech Stack:** Existing Rust toolchain, `markdown`, `regex`, `serde_json`; pinned markdownlint development oracle only, no Node runtime or new dependency.

**Spec:** `docs/superpowers/specs/2026-10-21-mdfmt-design.md`; rule contracts in `docs/compatibility/rules.json`, pinned `lib/md034.mjs`, `lib/md044.mjs`, `lib/md046.mjs` and `docs/compatibility/proper-name-audit.md`.

## Global Constraints

- Native Rust; use the toolchain in `mise.toml` and keep `Cargo.lock` tracked.
- Respect supported configuration, rule enablement, severity and inline directives.
- Preserve links, tables, literal code, definitions and metadata. This batch has no proposer or edit authority.
- MD013 and MD026 retain their existing approved behavior; all new diagnostics run on final output.
- Distinguish source review, recorded runtime evidence and corrected native policy.
- Require unchanged source, fix/no-fix equivalence and second-call stability for diagnostic-only cases.
- Do not read `.env`; reuse the current sibling feature worktree and shared configuration.
- Execute inline, with one independent whole-batch review and one regression-first Important/Critical fix pass. No second review, merge, push or routine approval questions.

## Decisions and assumptions

- Implement these rules diagnostically first. Upstream fixes are not edit authorization: names in code/HTML and URL spelling must remain intact.
- Ignore empty MD044 name entries, while checking other configured names. Empty strings identify no text and must never enter a zero-length global scan. This is a corrected policy, not pinned parity.
- MD044 excludes actual URL ranges rather than copying upstream's oversized exclusions. Names following URLs remain reportable, including in code when `code_blocks` is true.
- Use source spellings and JavaScript-compatible ASCII word boundaries/case matching for name checks; do not substitute rendered text or Rust Unicode word boundaries.
- Disabled first code blocks still anchor MD046's consistent style. Report permissions do not change document classification.
- Baseline is `485c584`: 447 tests across 62 targets and 47/53 built-ins. These three rules remain unimplemented until each native test/oracle gate passes. MD005/MD007/MD051 and later milestones remain outside this batch.

## Review Focus

- Literal code/HTML with URLs and configured names must not lose later proper-name findings or gain edit authority; Task 3 pins corrected URL boundaries and code flags.
- HTML sibling nesting, unmatched/void tags, image labels and link text must not be mistaken for exposed bare URLs; Task 2 pins these ownership boundaries.
- Empty/overlapping names, ASCII underscores, non-ASCII case pairs and UTF-16 positions must terminate and select actual source bytes; Task 3 pins matching/priority and bounded empty-name cases.
- Consistency anchors must survive disabled regions, nested list/quote ownership, tabs, metadata and generic HTML recovery; Task 1 pins classification independently from reporting.
- Dense URLs/names and repeated/distinct image labels must not add per-token whole-document/definition-payload parsing; Tasks 2/3 pin actual work budgets and repeated-call stability.

## File map

- Create `src/md046.rs`, `tests/md046.rs`: code-block classification and diagnostics.
- Create `src/parse/content.rs`, `src/md034.rs`, `tests/md034.rs`: field-selected content snapshot and bare-URL diagnostics.
- Create `src/md044.rs`, `tests/md044.rs`: configured-name matching, exclusions and diagnostics; extend content fields only with consuming tests.
- If image recovery is needed, extract the already-tested compact-header/paragraph cache into `src/parse/image_recovery.rs` and migrate the reference consumer in the same step. Do not duplicate full-definition recovery.
- Create `tools/parser-oracle/literal-content.mjs` and per-rule receipt files; separate ordinary observations from corrected-policy rows.
- Update `src/lib.rs`, `src/parse.rs`, CLI help, coverage assertions, README, rule inventory, roadmap and `docs/compatibility/literal-content-diagnostics.md` with each completed rule.

## Task 1: MD046 code-block style

**Files:** `src/md046.rs`, `tests/md046.rs`, shared oracle, MD046 receipt and integration/evidence files above.

**Interfaces:** Consume `Source`, `ParsedDocument`, `DocumentConfig` and recovered `heading_spaces::literal_exclusions` code ownership. Produce `md046::diagnose(&Source, &ParsedDocument, &DocumentConfig) -> Vec<Diagnostic>`. No shared model changes are needed for later tasks.

- [ ] Record pinned cases for `consistent`, `fenced`, `indented`, disabled, aliases, warnings, all newline forms/BOM, empty/unclosed fences, nested containers, tabs, metadata, HTML and disabled first anchors.
  Run `node tools/parser-oracle/literal-content.mjs --record --source-only --rule MD046`.
  Expected: source-only evidence, no native coverage claim.
- [ ] Write `mixed_code_styles_report_without_converting_code`: input `"```rs\nlet x = 1;\n```\n\n    code\n"` remains exact with no edits; MD046 reports one finding on physical line 5, empty span `22..22`, detail `Expected: fenced; Actual: indented`.
  Write `explicit_styles_and_disabled_first_anchors_preserve_consistency`, receipt comparison, CLI alias/warning/check/write/no-fix and MD013/MD026 composition tests.
- [ ] Run `mise exec -- cargo test --test md046`.
  Expected: missing MD046 diagnostics, not a broken test harness.
- [ ] Implement code style from owned source fence syntax and document order; retain disabled anchors and original physical lines. Emit `Code block style [Expected: ...; Actual: ...]` with an empty span on the opening line. No code conversion.
- [ ] Run focused tests, `--rule MD046` native oracle, affected code/fence/container tests, full native suite, fmt/clippy/diff checks.
  Expected: all pass; source bytes and repeated results remain exact.
- [ ] Update truthful evidence/coverage to 48/53 and commit `feat: report code block style without rewriting literal code`.

## Task 2: MD034 bare URLs

**Files:** `src/parse/content.rs`, optional shared image recovery extraction, `src/md034.rs`, `tests/md034.rs`, shared oracle/receipt and integration/evidence files.

**Interfaces:** Produce `ContentFields { bare_urls: bool, segments: bool }`, `ContentAnalysis { bare_urls: Vec<Range<usize>>, segments: Vec<ContentSegment> }`, `ContentSegment { span: Range<usize>, kind: ContentKind }`, `ContentKind::{Data, Code, Html}` and `content_analysis(&Source, &ParsedDocument, ContentFields) -> ContentAnalysis`. Task 2 requests only bare URLs; do not build unused segments. Produce `md034::diagnose(&Source, &ParsedDocument, &DocumentConfig) -> Vec<Diagnostic>`.

- [ ] Record native-parser/pinned cases for HTTP(S)/WWW/email literals, punctuation/escapes/entities, explicit links/autolinks, math/code, inline/flow HTML, balanced/nested/unmatched/void HTML tags, images/reference images, tables, BOM/CRLF, directives and warnings.
  Run source-only `--rule MD034` mode; expected: ordinary observations plus separately named native-policy differences only if supported by the approved preservation contract.
- [ ] Write `bare_urls_report_without_wrapping_or_changing_destinations`: `"go https://example.com now\n"` remains exact with MD034 span `3..22`, context `https://example.com`; `"![https://example.com](img)\n"` and `"[https://example.com](url)\n"` remain clean.
  Write `html_sibling_ownership_and_literal_scopes_match_recorded_observations`, source-range/directive/alias/CLI tests and dense-root/image work-budget tests.
- [ ] Run `mise exec -- cargo test --test md034`.
  Expected: missing MD034 diagnostics.
- [ ] Implement a source-backed visitor selecting native literal links but excluding links/images and paired inline HTML content according to pinned sibling matching. Keep unmatched-tag/void-tag behavior source-backed. Use recovered diagnostic ownership with original-byte spans; no blanket URL regex over Markdown.
  If sharing image recovery, extract `ImageRecovery::new(&Source, &Node, usize)` and `paragraph(&mut self, &str) -> Option<Arc<Node>>` from the existing reference implementation, retaining selected compact headers/native keys and paragraph-only cache. Retest both reference work budgets and all reference receipts immediately.
- [ ] Emit `Bare URL used [Context: "..."]`, using pinned UTF-16 context ellipses and actual source URL ranges. No wrapping edits; preserve tables and URL spelling.
- [ ] Run focused/native oracle and affected HTML/reference/inline tests; full suite, fmt/clippy/diff checks.
  Expected: unchanged source/idempotence and bounded diagnostic work; coverage 49/53 only after verification.
- [ ] Update evidence/roadmap/coverage and commit `feat: report bare URLs without altering link or table content`.

## Task 3: MD044 proper names

**Files:** `src/md044.rs`, `tests/md044.rs`; extend `src/parse/content.rs` with requested segments; shared oracle/receipt and integration/evidence files.

**Interfaces:** Consume Task 2's exact content snapshot with `segments: true`; extend its visitor to produce source data/code/HTML segments only when requested. Produce `md044::diagnose(&Source, &ParsedDocument, &DocumentConfig) -> Vec<Diagnostic>`. Preserve Task 2's bare-only work bound and reference recovery contracts.

- [ ] Record ordinary name/boundary/priority/flags cases: empty arrays, mixed case, longest-first names, already accepted spellings, punctuation-bearing names, underscores, Unicode/BMP/astral/UTF-16, escapes/entities, labels versus destinations/definitions, HTML/code/math, directives, aliases and warnings. Never send empty names to the upstream oracle.
  Retain corrected native-policy rows for empty entries and actual-URL exclusions; compare ordinary source outcomes separately.
- [ ] Write `proper_names_report_without_rewriting_prose_or_literal_code`: `"github\n"` with `names: ["GitHub"]` reports span `0..6`, detail `Expected: GitHub; Actual: github`, unchanged source/no edits.
  Write `urls_do_not_hide_following_names_in_enabled_code`: `"`https://example.com github`\n"` reports span `21..27`, clean with `code_blocks: false`; source audit remains false clean and is not used as native expected output.
  Write `empty_names_terminate_without_hiding_nonempty_names`: `names: ["", "GitHub"]` checks `github`, all-empty names report nothing under a bounded child-process regression.
  Add longest-first/overlap/correct-spelling, token boundary, Unicode/ASCII word-boundary, HTML flag, CLI and composition tests; dense-name/URL/image budgets must exercise real parsing.
- [ ] Run `mise exec -- cargo test --test md044`.
  Expected: missing MD044 diagnostics.
- [ ] Implement nonempty name checks with pinned longest-first precedence and accepted-name spelling set. Use ASCII `\w`/boundary semantics and legacy JavaScript case equivalence, not broad Rust Unicode regex boundaries or expansions. Record actual matched source ranges and prevent overlapping duplicate findings.
  Scan only requested source-token-like segments: exclude definitions/resources/reference labels, escapes/entities and math; include visible labels. Respect `code_blocks` and `html_elements` independently. Compute actual URL exclusions once per relevant segment; never extend them to segment end or reparse complete document/definitions per name.
- [ ] Emit `Proper names should have the correct capitalization [Expected: ...; Actual: ...]`; ignore empty names explicitly before matching. No capitalization edits, including inside code/HTML.
- [ ] Run all three new suites/native oracles, existing six MD044 audit observations, reference/HTML/code/whitespace/style regressions; full suite, fmt/clippy/diff checks.
  Expected: ordinary parity plus explicit corrected policies, exact literal content and idempotence; coverage 50/53 only after verification.
- [ ] Update evidence/roadmap/coverage and commit `feat: diagnose configured proper names with safe URL exclusions`.
- [ ] Request the one independent batch review over the plan baseline through final implementation HEAD, including every Review Focus and ledger ruling. Reproduce/fix only Important/Critical findings once, then run a green full suite; no second review.
- [ ] Retain all rulings/evidence in this plan, remove only its completed scratch workspace, and continue MD005/MD007/MD051 and later roadmap work without merge/push or approval questions.

## Planning check

The project already supplies native parsing, source offsets, config schemas and final-output diagnostic dispatch. Task 1 is independent; Task 2 produces the exact snapshot fields Task 3 consumes. Optional shared recovery extraction preserves the reference consumer in the same step. All five Review Focus items have owning test steps. No new dependency or edited content is authorized. Runtime-confirmed literals and spans must be checked before RED tests; any correction is a ledgered ruling, not an unrecorded plan change.
