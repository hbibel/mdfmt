# Link Fragments Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement native MD051 target diagnostics and uniquely determined safe ASCII case corrections, reaching 53/53 built-in rule coverage without claiming full roadmap completion.

**Architecture:** A field-selected fragment snapshot collects source-backed heading/HTML targets and inline/definition destinations. Diagnostics match the pinned rule's target/projection semantics; repairs separately require an unambiguous ASCII-only inline destination and strict original-parser validation. Run repairs after heading normalization and report against final output.

**Tech Stack:** Existing Rust toolchain, `markdown`, `regex`, `icu_properties`, `serde_json`; pinned markdownlint development oracle only. No new runtime dependency.

**Spec:** `docs/superpowers/specs/2026-10-21-mdfmt-design.md`, `docs/compatibility/rules.json`, pinned markdownlint `lib/md051.mjs`, existing heading/reference/HTML compatibility evidence.

## Global Constraints

- Native Rust; use `mise.toml` and tracked `Cargo.lock`. Reuse the sibling roadmap-completion worktree, with no session/configuration move.
- Must respect the effective supported markdownlint configuration, aliases, severity, inline permissions and both fix controls.
- Preserve inline links as indivisible units and never convert to references. Preserve tables, literal code, labels, titles and metadata.
- Keep MD013 safe width-based reflow and MD026's authorized punctuation fix unchanged. External formatter failure must preserve original code.
- Require idempotence and regression coverage for implemented transformations. Diagnostic recovery never grants editing permission.
- Until custom JavaScript pattern compatibility is implemented in its later roadmap task, nonempty `ignored_pattern` retains its explicit unsupported-setting error; do not silently use a Rust regex substitute.
- Execute inline autonomously, with one independent batch review and one Important/Critical regression-first fix pass; no second review, no merge/push, no `.env` reads.
- Finish the list-indentation batch's active review/fix pass before starting implementation. This plan is independent preparation.

## Decisions and assumptions

- Diagnostic targets follow the pinned source, not a generic GitHub-slugger package: heading data/escape/code/math leaves, entity/image/reference/resource exclusions, lowercase, Unicode Letter/Mark/Number/Connector_Punctuation plus ASCII space/hyphen, percent encoding, duplicate counters, custom `{#anchor}` and HTML id/a-name precedence. `#top` and pinned line fragments are recognized explicitly.
- Keep encoded target identity separate from raw source destination identity. The existing UTF-8 byte/UTF-16 coordinate model must survive BOM/CRLF, tabs, containers and recovered image labels.
- Missing targets are diagnostics, never guessed replacements. A case-only repair requires exactly one case-insensitive target, an inline (not reference/definition/image) destination, one physical line, no table/protected-code overlap and raw `#[A-Za-z0-9_-]+` syntax for both current and replacement values. Escaped/entity/percent/non-ASCII destinations remain diagnostic when mismatched.
- Preserve definitions for the first repair subset; changing a definition can affect distant/table references. Do not import upstream's first-match fix when multiple HTML/heading targets differ only by case.
- Preserve general semantic-equivalence guards. MD051's private edit validator may normalize only the admitted ASCII fragment URL case when comparing position-free original/candidate trees; labels/titles/other node fields remain exact, with only the declared source destination bytes edited.
- `ignore_case` suppresses the pinned case-only findings and corresponding edits; it does not hide genuinely missing targets.
- Built-in coverage at 53/53 includes diagnostic-only rules, not blanket fixability. General conflict reporting, formatter integration, later regex/package compatibility and release hardening remain separate unfinished roadmap work.

## Review Focus

- Duplicate headings, custom anchors, explicit HTML ids and `#top` collisions must preserve pinned target ordering/counters and never produce an ambiguous automatic correction; Task 1 tests targets, Task 2 tests rejection.
- Escapes/entities, Unicode case/category changes, math/code/HTML/image/reference heading projection and percent-encoded destinations must not invent or hide targets; Task 1 owns these projection/range cases.
- Multiline/recovered links and definitions, nested image links, tables, docfx tabs and disabled target lines must retain correct ownership and original positions; Task 1 diagnoses, Task 2 preserves unsafe bytes.
- Case repairs after MD026/heading changes, MD013 wrapping, inline directives and partial permissions must converge without editing protected targets/labels or reopening earlier passes; Task 2 owns composition/CLI/repetition.
- Dense headings/links and repeated image labels must not rescan all targets or reparse the whole document per link; both tasks include bounded workloads with exact finding counts and shared-parser consumer regressions.

## File map and interfaces

- Create `src/parse/fragments.rs` for target/destination projection and source mappings, exported privately from `src/parse.rs`.
- Create `src/md051.rs`, `tests/md051.rs`, `tools/parser-oracle/link-fragments.mjs`, `tools/parser-oracle/md051-observations.json` and `docs/compatibility/link-fragments.md`.
- Integrate final diagnostics/repairs in `src/lib.rs`; update README, CLI help, inventory, coverage assertions and roadmap truthfully after each verified task.
- Task 1 produces `FragmentSnapshot { targets: Vec<String>, destinations: Vec<FragmentDestination> }`, `FragmentDestination { span: Range<usize>, destination: Range<usize>, value: String, inline: bool, docfx_tab: bool }` and `fragment_snapshot(&Source, &ParsedDocument) -> FragmentSnapshot`. Targets retain insertion order and uniqueness, while a lookup set/map supports linear classification.
- Prefer consuming existing `reference_analysis` inline-style destination ranges and `html_tokens`; add a definition-destination field only with consuming tests and existing reference-consumer gates. Do not duplicate the full link/definition grammar or parse unrelated definition payloads per link.
- Task 1 exposes `md051::diagnose(&Source, &ParsedDocument, &DocumentConfig) -> Vec<Diagnostic>`. Task 2 adds `md051::propose(&Source, &ParsedDocument, &DocumentConfig) -> Vec<Edit>` and a focused private AST validator, not a new permissive generic `parse::equivalent` mode.

## Task 1: Target identity and MD051 diagnostics

- [x] Record ordinary targets/destinations/options, including duplicate/custom anchors, HTML id/name precedence, top/line fragments, Unicode/UTF-16/percent, entities/escapes, code/math/images/references in headings, multiline links/definitions, docfx tabs, tables, recovered containers, directives and warnings. Keep nonempty ignored patterns as explicit configuration-rejection tests until the later regex task.
- [x] Write `fragment_diagnostics_distinguish_missing_and_wrong_case_targets`: `"# Hello\n\n[a](#hello) [b](#Hello) [c](#missing)\n"` reports line 3 ranges `[13,11]` with `Expected: #hello; Actual: #Hello` and `[25,13]` with no detail, unchanged source in no-fix mode. With `ignore_case:true`, only the missing target remains.
- [x] Write `heading_projection_and_duplicate_counters_match_pinned_targets`: `"# Hello\n# Hello\n# Hello-1\n\n[x](#hello-1) [y](#hello-2)\n"` flags only `[y]` at line 5/range `[15,13]`; `"# A &amp; B\n\n[x](#a--b)\n"` is clean. Add literal Unicode/emoji/code/math/image/reference/custom/HTML collision controls and disabled-target anchors.
- [x] Write `definition_and_nested_image_links_keep_source_ownership`: `"[x][id]\n\n[id]: #missing\n"` flags only the definition on line 3/range `[1,14]`; `"![alt](#missing) ![[x](#missing)](img)\n"` flags only the nested link at range `[20,13]`. Multiline `[a](\n #missing\n)\n` reports line 1 without context, anchored by a zero-width span at the physical line start. Add original byte/BOM/newline/table/docfx regressions and check/write/fix/no-fix preservation.
- [x] Run `mise exec -- cargo test --test md051`. Expected: missing MD051 findings, not invalid fixtures.
- [x] Implement source-backed projections and indexed target classification, using pinned percent/unescape/case semantics. Preserve source; diagnostics say `Link fragments should be valid` with pinned optional detail/context. Check existing reference/HTML/heading consumers if their snapshot fields change.
- [x] Run focused tests, native oracle, affected reference/heading/HTML/MD013/MD026 comparisons, full optimized suite, fmt/clippy/diff. Expected: all assertions/budgets pass, no per-link document reparsing.
- [x] Update evidence and coverage to 53/53 without setting broader roadmap milestones complete; commit `feat: report missing and mismatched link fragments`.

## Task 2: Unique ASCII fragment case corrections

- [x] Write `unique_ascii_fragment_case_repairs_change_only_owned_destinations`: the Task 1 Hello input fixes only `[b](#Hello)` to `[b](#hello)`, leaving missing `[c]` and all labels/titles exact. `"<a id=\"Foo\"></a>\n\n[x](#foo)\n"` safely fixes to `#Foo`. Second call makes no edits; no-fix/config.fix:false remain exact.
- [x] Write `ambiguous_or_protected_fragment_targets_are_never_guessed`: colliding HTML ids `Foo`/`FOO`, definitions, multiline/percent/entity/escaped/non-ASCII destinations and table links remain exact even when diagnosed. An external or relative nonlocal URL containing `#` must never become a local-target repair.
- [x] Write composition/CLI/alias/warning/inline-directive tests, including headings changed by MD026/MD003, wrapped linked prose and original-coordinate edit reconstruction. Test disabled target lines still supply targets, while disabled link lines receive neither fix nor finding.
- [x] Run `mise exec -- cargo test --test md051`. Expected: missing safe case corrections before edit implementation, with Task 1 diagnostics still passing.
- [x] Implement the narrow proposal/AST validation contract after final heading normalization. Admit only one exact case-folded target and raw safe ASCII resource syntax, require original permissions/ownership, and compare every non-URL AST field exactly. Do not relax generic protected-link validation or rewrite tables/definitions.
- [x] Add bounded dense/ambiguous-target and repeated recovered-image workloads with exact semantic assertions; run source/fix/no-fix/idempotence oracle, affected suites and complete optimized tests, fmt/clippy/diff. Expected: safe corrections converge in one call and protected source remains exact.
- [x] Update preservation/policy evidence and commit `feat: correct unambiguous local fragment capitalization`.
- [x] Request one independent batch review with every Review Focus and chronological ruling. Reproduce/fix Important/Critical findings once, then complete the full suite; no second review.
- [x] Retain all rulings/evidence, remove only the completed scratch directory and continue milestone 5 interactions/conflict reporting and later roadmap work without merge/push or routine questions.

## Planning check

Runtime probes confirmed the literal ranges and target projections above, including
entity omission, definition-only findings, nested image links, duplicate collisions,
Unicode percent forms and line-fragment exceptions. Task 1 produces all fields
Task 2 consumes. The repair subset deliberately avoids a new general relaxation
of link preservation. All review-focus classes have owning regression steps.
List-indentation review closed at `9174d4a`, with one regression-first fix pass,
503 passing optimized tests across70 targets, and its debug-throughput caveat
retained. Its17 rulings/costs remain in the tracked plan; only its scratch was
removed. This plan may now begin without changing branch/worktree or configuration.

## Execution evidence

Task 1's initial behavioral/corpus tests failed before implementation. Eight public
tests and 664 native source comparisons now pass. Expanded footnote projection
was independently RED-to-GREEN. A real-parser HTML-image test also reproduced
per-image unrelated definition payload parsing; shared compact/header caching now
requires one parse/<4,096 bytes while varying both axes. Prior reference/HTML/image/
heading consumers pass their native oracles. The complete optimized suite passes
512 tests across71 targets, with fmt/clippy clean. Coverage53/53 includes
diagnostic-only/conditional rules; safe MD051 case repairs and review remain pending.

Rulings so far:

1. Request definition-destination ranges from the existing reference model and
   share resource scanning. Cost if wrong: shared reference extraction surface;
   original parser budgets and native consumers guard the migration.
2. Reuse compact/cached recovery for HTML image targets. Cost if wrong: shared
   HTML/image ownership; literal corpora and actual-parser budgets gate it, with
   no edit authority.
3. Keep `Coverage.complete` false at53 built-ins. Cost if wrong: a conservative
   flag despite the explicit all53 rule list; diagnostic-only subsets, later
   options and parser/preservation limits prevent a complete-fidelity claim.
4. Use a strict private AST comparison for admitted ASCII fragment case changes,
   not a generic link-preservation relaxation. Cost if wrong: conservative
   rejection of recovered/extension-owned links; labels/tables remain protected
   and remaining findings stay visible.
5. Reuse lexical ATX caption bounds rather than lossy native children alone.
   Cost if wrong: source-gap classification surface; hash/resource/container
   controls and native heading gates retain the distinction without new edits.
6. Expose consumed existing prefixes for literal lazy continuations while keeping
   complete-prefix semantics unchanged. Cost if wrong: shared prefix-walk internal
   result shape; partial/literal controls and old consumers gate the distinction.
7. Keep the narrow inline ASCII repair subset, as the reviewer set aside. Cost
   if wrong: broader otherwise-fixable cases need a later safe adapter, while
   definitions/tables/images/recovered-only/encoded/non-ASCII/ambiguous cases stay
   visible rather than guessed.
8. Keep custom ignored patterns and later regex/package compatibility explicitly
   deferred, as the reviewer set aside. Cost if wrong: users wait for required
   later work instead of receiving silent incompatible pattern semantics.
9. Keep general conflicts and formatter integration separate, as the reviewer set
   aside. Cost if wrong: unfinished milestones remain explicit at53 built-ins.
10. Do not infer debug-full/platform/fuzz/corpus guarantees from optimized Linux
    review, as the reviewer set aside. Cost if wrong: baseline-unknown debug and
    untested platform/input classes remain hardening work.
11. Leave unrelated completed defects outside this review without demonstrated
    regression, as the reviewer set aside. Cost if wrong: unrelated legacy issues
    remain discovery work; directly changed consumers have native gates.
12. Exclude bookkeeping/style polish below Important/Critical severity, as the
    reviewer set aside. Cost if wrong: pending wording can briefly lag commit/
    cleanup, but closure must reflect real actions before scratch removal.

Task 1 committed at `75c9c9c`. Task 2's literal unique/cross-rule tests reproduced
missing repairs before implementation; the fixture retains MD003's existing
three-character setext underline. Thirteen MD051 tests and664 native comparisons
now pass, including source-only/no-fix behavior and case repair/rejection controls.
Every changed code unit is independently checked against upstream-owned ASCII
inline destination spans outside tables; final findings/idempotence pass. Thirty
upstream-fix differences retain deliberately protected/ambiguous source.

The complete optimized suite passes517/517 across71 targets, fmt/clippy clean;
prior MD003/MD013/MD026/reflow/MD024/HTML/image comparisons pass after integration,
and shared6,305 reference comparisons passed after the range/cache extraction.
All assertions/budgets unchanged. Coverage53/53 is not roadmap completion; commit
and the one independent batch review follow.

Task 2 committed at `b4e00ae`. The single independent review over
`d0b618a..b4e00ae` found four Important findings, no Critical findings or minors.
Five primary tests in `tests/fragment_review.rs` failed before the corrections;
all now pass, including line-fragment navigation, automatic-link exclusion,
lexical ATX caption gaps/counters, code/math data whitespace and repeated CLI
byte preservation. Partial/lazy-prefix controls were separately RED-to-GREEN.
Diagnosis and repair share validity exemptions; source captions retain data lost
by child ranges, and cached actual container endpoints do not trim literal spaces.

Final complete optimized verification passes522/522 across72 targets, fmt/clippy
clean;1,144 ordinary MD051 comparisons check original wrong-case eligibility,
upstream destination bounds, final findings and idempotence. Thirty preserved
upstream-fix differences remain explicit. All affected heading/reference/HTML/
code/reflow oracles pass. All four Important findings fixed in one pass, no
Critical findings/deferred minors, no second review, no relaxed thresholds.
Commit the corrections before removing only this completed scratch directory;
retain all12 chronological rulings/costs here. Coverage53/53 remains distinct
from incomplete roadmap milestones.
Corrections committed at `daa0b8c`; all12 rulings retained before removing only
this completed scratch directory. Branch/worktree remain unmerged/unpushed.
