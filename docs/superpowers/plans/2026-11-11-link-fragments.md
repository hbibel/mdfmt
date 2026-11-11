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
- [ ] Update evidence and coverage to 53/53 without setting broader roadmap milestones complete; commit `feat: report missing and mismatched link fragments`.

## Task 2: Unique ASCII fragment case corrections

- [ ] Write `unique_ascii_fragment_case_repairs_change_only_owned_destinations`: the Task 1 Hello input fixes only `[b](#Hello)` to `[b](#hello)`, leaving missing `[c]` and all labels/titles exact. `"<a id=\"Foo\"></a>\n\n[x](#foo)\n"` safely fixes to `#Foo`. Second call makes no edits; no-fix/config.fix:false remain exact.
- [ ] Write `ambiguous_or_protected_fragment_targets_are_never_guessed`: colliding HTML ids `Foo`/`FOO`, definitions, multiline/percent/entity/escaped/non-ASCII destinations and table links remain exact even when diagnosed. An external or relative nonlocal URL containing `#` must never become a local-target repair.
- [ ] Write composition/CLI/alias/warning/inline-directive tests, including headings changed by MD026/MD003, wrapped linked prose and original-coordinate edit reconstruction. Test disabled target lines still supply targets, while disabled link lines receive neither fix nor finding.
- [ ] Run `mise exec -- cargo test --test md051`. Expected: missing safe case corrections before edit implementation, with Task 1 diagnostics still passing.
- [ ] Implement the narrow proposal/AST validation contract after final heading normalization. Admit only one exact case-folded target and raw safe ASCII resource syntax, require original permissions/ownership, and compare every non-URL AST field exactly. Do not relax generic protected-link validation or rewrite tables/definitions.
- [ ] Add bounded dense/ambiguous-target and repeated recovered-image workloads with exact semantic assertions; run source/fix/no-fix/idempotence oracle, affected suites and complete optimized tests, fmt/clippy/diff. Expected: safe corrections converge in one call and protected source remains exact.
- [ ] Update preservation/policy evidence and commit `feat: correct unambiguous local fragment capitalization`.
- [ ] Request one independent batch review with every Review Focus and chronological ruling. Reproduce/fix Important/Critical findings once, then complete the full suite; no second review.
- [ ] Retain all rulings/evidence, remove only the completed scratch directory and continue milestone 5 interactions/conflict reporting and later roadmap work without merge/push or routine questions.

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
