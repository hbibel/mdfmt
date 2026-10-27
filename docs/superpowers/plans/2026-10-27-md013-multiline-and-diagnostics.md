# MD013 Multiline Reflow and Diagnostics Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Continue milestone 4 through multiline/container prose reflow and native remaining-MD013 diagnostics, not another single-line-only checkpoint.

**Architecture:** Model legal whitespace gaps within parsed paragraphs, including soft-break container prefixes. Preserve hard breaks and indivisible inline regions, derive continuation prefixes from list/quote ancestry, and validate expanded edits against the gap model and a position-independent semantic tree. Diagnose the formatted output using resolved per-line permissions and pinned MD013 semantics.

**Tech Stack:** Existing Rust toolchain, markdown 1.0.0, existing declarative configuration; pinned Node oracle for development only.

**Spec:** `docs/superpowers/specs/2026-10-21-mdfmt-design.md`, MD013 source and compatibility requirements in `docs/compatibility/baseline.md`.

## Global Constraints

- Must respect the effective supported markdownlint configuration.
- Must preserve inline links without conversion to reference links.
- Must preserve tables. Violations involving tables may remain unresolved.
- Must preserve content and Markdown meaning rather than force an unsafe fix.
- Must be idempotent: formatting the result again produces identical output.
- Keep unsupported-extension and HTML preservation guards; do not claim full upstream parser equivalence.
- Preserve previous uncommitted work in the existing sibling worktree; no merge/push without a request.

## Review Focus

- Nested lists/quotes, lazy continuation, task checkboxes, and ordered-marker widths must retain their block tree and continuation alignment (Task 1).
- Soft joins must not cross hard breaks, directives, disabled source lines, protected inline source, or tabs (Task 1).
- Exact widths and astral characters must use UTF-16, including prefix budget and diagnostic positions that land inside a surrogate pair (Tasks 1–2).
- Front matter/BOM and setext headings, fenced code, tables, reference definitions, link-only lines, and final whitespace affect diagnostic exceptions (Task 2).
- Check/stdout/write modes must distinguish changed output, warning/error findings, and fix permission; no hidden Node runtime dependency (Task 3).

## Task 1: Source-aware multiline and container reflow

**Files:** Modify `src/parse.rs`, `src/source.rs`, `src/edit.rs`, `src/reflow.rs`, `tests/reflow.rs`; add focused `src/reflow/layout.rs` and tests as needed.

**Interfaces:** Retain `reflow::propose(&Source, &ParsedDocument, &DocumentConfig) -> Vec<Edit>`. Add paragraph-gap layout with source spans and permitted join/wrap replacements; edit validation must authorize only exact modeled gaps, not arbitrary container changes. Add `parse::equivalent` using normalized mdast text, positions removed, exact protected source. Expose source line ranges internally.

- [ ] Write literal failing cases for joining `alpha\nbeta gamma` at width 10 to `alpha beta\ngamma`, nested-list and quote acceptance outputs, ordered/task/list-in-quote/quote-in-list prefixes, hard-break segments, lazy continuation, directives and idempotence.
- [ ] Run `mise exec -- cargo test --test reflow`; expect new multiline/container assertions to fail.
- [ ] Add semantic tree/prefix metadata and legal gaps. Pack eligible paragraph text greedily; retain horizontal spacing when it fits, join soft breaks with one space, repeat structural continuation prefixes, preserve hard-break source, and skip unsafe or partially disabled paragraphs.
- [ ] Admit expanded gaps through the existing edit pipeline only with exact gap/replacement validation and successful semantic/protected-source comparison. Retain original strict edit tests and add rejection tests for invented container edits.
- [ ] Run the full Rust suite and scaling smoke test; compare representative multiline/container outputs with pinned upstream structure and ensure second-pass edits are empty.

## Task 2: Native MD013 remaining-violation diagnostics

**Files:** Create `src/md013.rs`, `tests/md013.rs`; modify `src/lib.rs`, `src/parse.rs`, oracle scripts/fixtures.

**Interfaces:** `md013::diagnose(source: &Source, parsed: &ParsedDocument, document: &DocumentConfig) -> Vec<Diagnostic>`. Rebuild source/document permissions after edits; `FormatOptions.fix` and resolved `fix:false` suppress edits, not findings. Diagnostics use code/rule ID `MD013`, configured severity, byte spans in output, and expected/actual UTF-16 lengths in messages.

- [ ] Write failing diagnostic tests for default/strict/stern, independent/inherited heading/code widths, include/exclude flags, reference definitions, link/image-only exceptions versus inline code/data, BOM/front matter, directives, Unicode whitespace, severity and output coordinates.
- [ ] Run `mise exec -- cargo test --test md013`; expect missing findings against the no-op diagnostic pipeline.
- [ ] Implement pinned source semantics: code width before heading width before prose; exact limit valid; default replaces the final RE2JS-nonspace run by one unit for threshold comparison; strict bypasses standalone-link and stern-token exceptions but not definitions or category exclusions. Preserve directive/severity behavior and UTF-8-safe spans when the threshold lies within a UTF-16 surrogate pair.
- [ ] Extend the pinned development oracle to compare native findings (including disabled-fix input) against upstream; preserve explicit limits for unmodeled parser extensions instead of claiming full rule compatibility prematurely.
- [ ] Run the full Rust suite and pinned comparisons.

## Task 3: CLI, acceptance coverage, status, and final review

**Files:** Modify `src/cli.rs`, `tests/config_cli.rs`, `README.md`, roadmap, parser evidence; add milestone evidence documentation if useful.

- [ ] Write failing CLI cases for remaining strict error/warning findings, `fix:false` diagnostics, write/check/stdout behavior and positions after reflow; verify RED.
- [ ] Update CLI help and documentation to describe implemented MD013 behavior and genuine remaining limitations. Run applicable MD013 acceptance fixtures as real native tests, not just JSON consistency checks. Do not implement later-rule or external-formatter behavior merely because a fixture includes it.
- [ ] Run `mise exec -- cargo test --all-targets`, `mise exec -- cargo fmt --check`, `mise exec -- cargo clippy --all-targets -- -D warnings`, and rebuilt-CLI upstream/scaling comparisons.
- [ ] Obtain one independent read-only review of the complete changes; fix important issues with failing regressions and a green full suite. Keep work available without a blocking integration question.

## Execution

The user explicitly requested continued work without approval interruptions. Execute natively in the existing worktree. Track task status and decisions in this plan's own scratch ledger. This plan supersedes the initial slice's intentional multiline/container/diagnostic deferrals, not its preservation constraints.
