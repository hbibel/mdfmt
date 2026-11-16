# Custom Configuration Regex Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Support native custom metadata/title/heading/fragment patterns with pinned consumer semantics, bounded work and byte-exact metadata preservation.

**Architecture:** Reuse the bounded native ECMAScript engine for rule patterns and a validated RE2-compatible metadata compiler. Carry explicit metadata-prefix context through Sources and all complete-document reparses. No global parser state or JavaScript runtime.

**Tech Stack:** Existing Rust/mise toolchain, vendored regress with UTF-16/fuel, existing Rust regex, pinned development-only markdownlint/cli2/RE2JS oracles.

**Spec:** `docs/superpowers/specs/2026-11-16-custom-regex-design.md`; binding roadmap and existing preservation/configuration contracts.

## Global Constraints

- Installed-package review/fix pass closed at `89c0c36`: 561 optimized tests across 75 targets and 102 package comparisons pass; all 18 decisions retained.
- No Node runtime, downloads, executable config or global/thread-local parser context.
- JS consumers retain exact flags: title `i` without `u`, MD043 `u` plus conditional `i`, MD051 no flags. Metadata first validates JS `u`, then follows the pinned RE2-style subset.
- Preserve default metadata recognition, original prefix bytes, BOM/newline mapping, directives and original-coordinate edits. Recognized HTML directives in metadata retain pinned permissions.
- Patterns max 65,536 UTF-16 units, nesting max 64, metadata compiled size max 1 MiB. Per-document pattern cache max 256 compiled entries and 1 MiB source. Exhaustion is an explicit error, never a nonmatch or write.
- ResolvedConfig gains optional compiled FrontMatterPattern; existing exhaustive prerelease literals need the new field or `..Default::default()`. FormatOptions and pure/process-I/O split stay unchanged.
- Inline execution in tested batches, one independent review and one Important/Critical regression-first fix pass; no second review, merge/push or routine prompts.

## Review Focus

- A full-document candidate or transformed diagnostic view losing custom metadata context must not expose protected bytes to edits or processes; owning `metadata_context_survives_all_reparse_stages`.
- Unicode/non-Unicode flags, surrogate escapes and case folding must not silently alter required-heading or ignored-fragment decisions; owning `rule_patterns_match_pinned_flags_and_utf16`.
- Invalid or hostile regexes must fail with provenance before output/write and not poison later documents; owning `pattern_errors_are_transactional_and_recoverable`.
- No-match/zero-width/partial-line metadata prefixes must not fall back to default recognition, loop forever or shift original byte coordinates; owning `metadata_prefix_boundaries_preserve_original_coordinates`.
- Aliases/inline permissions/title suppression and MD051 ignored links must affect actual repairs and diagnostics, not just config acceptance; owning `configured_patterns_control_real_rule_actions`.

## Task 1: Bounded JavaScript rule patterns

Files: create `src/config/pattern.rs`, `tests/config_patterns.rs`, `tools/parser-oracle/config-patterns.mjs`, `tests/fixtures/config-patterns/cases.json`; modify `src/config/mod.rs`, `src/config/rules.rs`, `src/config/directives.rs`, `src/heading_structure.rs`, `src/md051.rs`, `src/lib.rs`, affected callers/tests.

Interfaces:

- `PatternFlags { unicode: bool, ignore_case: bool }` is private, ordered and copyable; no arbitrary flags are accepted from declarative strings.
- `JsPattern::compile(source: &str, flags: PatternFlags, property: &str) -> Result<JsPattern, ConfigError>` validates syntax/length/nesting, using existing native regress and preserving UTF-16 source semantics for unflagged patterns.
- `JsPattern::is_match(&self, input: &str) -> Result<bool, ProcessingError>` executes one bounded search and detects fuel exhaustion, including after no result.
- `DocumentConfig::pattern(&self, source: &str, flags: PatternFlags, property: &str) -> Result<JsPattern, ProcessingError>` returns a cheap immutable shared handle from a bounded context-local cache (256 entries/1 MiB source), not a process-global cache.
- Heading diagnostics and MD051 proposals/diagnostics become fallible where they consume patterns. Propagate errors through callers; preserve ordinary edit origins and independent rule permissions.

- [x] Write `configured_patterns_control_real_rule_actions`: custom MD001/MD025/MD041 title keys affect applicable findings; `%` headings enforce case/level/loops; MD051 ignored invalid and wrong-case fragments remain exact while unrelated findings persist; aliases and inline configure/disable variants agree.
- [x] Write `rule_patterns_match_pinned_flags_and_utf16`: actual lookaround/backrefs for JS consumers, Kelvin/long-s/astral/lone-surrogate escape distinctions and empty patterns. Pin literal findings and source receipts rather than engine-generated expectations.
- [x] Write `pattern_errors_are_transactional_and_recoverable`: invalid syntax/length/nesting, actual deterministic fuel exhaustion, disabled rules not executing hostile patterns, CLI exit2/no output/write and a later independent safe success. Preserve invalid MD036 class errors.
- [x] Record source-only pinned cases; run `mise exec -- cargo test --release --test config_patterns` and the native oracle consumer. Expected: existing unsupported-rule-pattern failures, not fixture/schema errors.
- [x] Implement bounded JsPattern/cache and replace custom-pattern rejection only for actual supported consumers. Use per-rule flags and exact upstream matching inputs; include ignored MD051 destinations in both diagnose and propose paths.
- [x] Run focused pattern/title/fragment/config tests, `node tools/parser-oracle/config-patterns.mjs`, existing content/MD051 native gates, complete optimized suite, fmt/clippy/diff. Expected: literal and pinned findings/actions/idempotence pass; limits return named errors without input changes.
- [x] Commit `feat: support bounded native rule configuration patterns`; task-done with the focused public tests plus heading/fragment/config regressions.

## Task 2: Configured front-matter recognition throughout formatting

Files: create `src/config/front_matter.rs`, `tests/custom_front_matter.rs`, `tools/parser-oracle/front-matter.mjs`, `tests/fixtures/front-matter/cases.json`; modify `src/source.rs`, `src/parse.rs`, derived views in `src/parse/`, candidate Source construction in `src/edit.rs`, rule modules/reflow/blank-lines/code-formatters, `src/config/load.rs`, `src/config/resolve.rs`, `src/lib.rs`, docs/README/roadmap and exhaustive ResolvedConfig literals.

Consumes: Task1 `JsPattern::compile` for JS-u validation and its limit/error policy; actual rule patterns/title handling remain in Task1 owners.

Interfaces:

- Public `FrontMatterPattern::new(source: &str) -> Result<FrontMatterPattern, ConfigError>` owns validated immutable compiled metadata regex (cheap clone); `prefix_end(&self, source: &str) -> Result<usize, ProcessingError>` returns the original-byte prefix end after normalized matching, with BOM accounted for. No-match/zero-width returns BOM boundary, not default recognition.
- `ResolvedConfig::front_matter: Option<FrontMatterPattern>` defaults None. Resolver uses invocation-level cli2 frontMatter precedence; empty strings map to default recognition. Non-string/invalid options fail with frontMatter/path provenance.
- `ResolvedConfig::source(&self, text: &str) -> Result<Source, ProcessingError>` selects configured/default parsing context. Source gains private explicit-prefix metadata and a same-origin derive method; transformed views must map its boundary through their existing source-offset tables.
- Parser uses the explicit boundary when present; fresh Source::new retains current default recognizer. Complete-source candidates inherit context, while genuinely synthetic isolated probes remain fresh. No edit-authority relaxation follows custom recognition.

- [x] Write `metadata_prefix_boundaries_preserve_original_coordinates` for custom/missing/empty/no-match/zero-width/partial-line patterns, LF/CRLF/CR/BOM/astral source, default-looking Markdown after a custom nonmatch, exact original edit reconstruction and second-call no edits.
- [x] Write `metadata_context_survives_all_reparse_stages`: metadata contains rule-sensitive headings/prose/tables/links/code; enabled Markdown repairs and optional controlled formatters affect only the body, preserve prefix bytes and final warning anchors through all active transformations. Include tab-expanded/recovered diagnostic views and recognized metadata HTML directive permissions.
- [x] Add actual pinned RE2JS cases for ASCII versus JS whitespace, classes/escapes/anchors/Unicode properties and unsupported lookaround/backrefs. Literal tests must pin no unexpected language formatter launch from metadata and title-pattern recognition of custom metadata.
- [x] Run public tests and source/native oracle RED. Expected: unsupported-front-matter-pattern or missing compiled option/context, not parser-fixture mistakes.
- [x] Implement FrontMatterPattern compiler/normalization mapping, resolver option, Source inheritance and pipeline propagation. Audit every Source::new: same-document candidate, transformed mapped view or synthetic probe. Fail explicitly on unsupported semantics; never silently restore default parsing after custom no-match.
- [x] Run `mise exec -- cargo test --release --test custom_front_matter --test config_patterns` plus affected heading/fragment/config/reflow/code/preservation tests; run new metadata/rule oracles and prior config/content/MD051/reflow/code gates, complete optimized suite, fmt/clippy/diff. Expected: exact protected prefix and original-coordinate edits with unchanged prior defaults; no false all-input/platform guarantee.
- [ ] Update compatibility/README/roadmap and record Important/Critical discoveries; commit `feat: preserve configured front matter across formatting passes`; task-done with focused consumer evidence.
- [ ] Request one independent batch review with every Review Focus and chronological ruling. Reproduce/fix Important/Critical issues once regression-first, complete suite; retain every ruling/cost before completed scratch removal.
- [ ] Continue bounded non-Unix formatter transport and remaining release hardening autonomously; preserve branch/worktree without merge/push.

## Planning check

Task1 ships actual rule consumers, not disconnected infrastructure; Task2 consumes
its explicit validation interface and error/limit policy. Both preserve existing
rule permissions and source-edit ownership. The five Review Focus entries have
owning public tests. The Source audit includes all candidate and transformed
views, not only the public entry parse. The optional compiled field's prerelease
source-compatibility cost and partial-prefix coordinate difference are explicit
builder decisions, not inferred user approval. The package prerequisite is met;
reuse the existing sibling worktree without a session/configuration move.

Task1 verification: initial three unsupported-rule-pattern RED tests preceded the
implementation; four public tests and 42 actual source/native CLI comparisons now
pass. The mixed `%#` prefix case additionally failed before its level-grammar
correction. Genuine fuel/cache exhaustion, disabled no-execution, original-source
check/write failure preservation, aliases/inline controls and repeat stability pass.
One initial complete run stopped at an obsolete deferred-feature assertion; that
assertion now tests actual match/nonmatch/source preservation. Final complete
optimized verification passes 565 tests across 76 targets, with fmt/clippy/diff
clean and prior 554 content/1144 fragment/110 config/53 schema comparisons passing.

Task1 rulings: replace obsolete supported-string rejection tests with actual
consumer assertions (cost: supported callers no longer get deferred errors), and
share only the existing non-Unicode source-unit decoder with picomatch (cost:
helper coupling, not a change to glob compilation/execution). MD036 class-injection
errors and custom frontMatter deferral remain unchanged. No new dependencies.

Task1 committed at `6e4c1d7`. Task2 initially failed on the missing compiled API
and explicit ResolvedConfig field. Five public tests and 29 actual pinned metadata
cases now pass. Added controls reproduced literal class-set/range mismatches and
JS-versus-RE2 class-backspace acceptance before corrections. The compiler now
translates these deliberately rather than exposing metadata through backend syntax.
Original prefix bytes, actual partial-line coordinates, mapped views, all-default
rule protection, active body repairs, code-stage no-spawn/success/final-failure
anchors and idempotence pass.

Source audit: complete-document candidates use derive, transformed diagnostic/tab/
HTML/list/table views use mapped boundaries, and processing/code stages construct
configured Sources. Remaining fresh Sources are tests, isolated label/HTML/container/
restarted-flow probes or coordinate/display-only views (CLI rendering, edit
composition, directive positions, comment display). No global/thread-local state.
Generic authorized edits also reject explicit protected-prefix overlap.

Complete pre-review optimized verification passes 570 tests across 77 targets;
29 metadata/42 rule-pattern/102 package cases and prior 554 content/1144 fragment/
220 fence-style/1662 code-style/29 reflow/110 config/53 schema comparisons pass.
Root fmt/clippy/diff checks are clean. No dependency versions changed. Commit/
task-done then one independent review of the combined regex batch.
