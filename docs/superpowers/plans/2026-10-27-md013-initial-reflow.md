# Initial MD013 Reflow Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Begin milestone 4 with safe, configuration-aware wrapping of single-line, top-level prose paragraphs.

**Architecture:** Propose localized whitespace replacements only inside existing eligible text regions. Pack indivisible inline source units greedily to the configured UTF-16 width; reparse proposed output and reject the document's batch if its structure or protected content changes. Keep the existing edit validator and document permissions in force.

**Tech Stack:** Rust 1.99, markdown 1.0.0, existing configuration and edit pipeline.

**Spec:** `docs/superpowers/specs/2026-10-21-mdfmt-design.md`, especially milestone 4; approved width policy in `docs/compatibility/baseline.md`.

## Global Constraints

- Must respect the effective supported markdownlint configuration.
- Must preserve inline links without conversion to reference links.
- Must preserve tables. Violations involving tables may remain unresolved.
- Must be idempotent: formatting the result again produces identical output.
- No new runtime dependencies; use the toolchain declared in `mise.toml`.
- This is a milestone slice, not completion: multiline joining, lists/quotes, diagnostic semantics, and full MD013 coverage remain pending. Do not advertise complete MD013 coverage.

## Review Focus

- A wrap may expose a heading, list marker, thematic break, HTML, or reference definition: reject structural changes.
- Astral characters count as two UTF-16 units; exact-width input stays unchanged.
- Links, images, code, emphasis, HTML, and hard breaks must not be edited internally.
- Inline directives apply to original source lines; a line-scoped directive on later content must not change meaning when earlier content gains lines.
- BOM, mixed terminators, non-breaking spaces, indentation, and final-newline state must survive.

## Task 1: Conservative single-line paragraph wrapping

**Files:** Create `src/reflow.rs`, `tests/reflow.rs`; modify `src/lib.rs`.

**Interfaces:** `reflow::propose(source: &Source, parsed: &ParsedDocument, document: &DocumentConfig) -> Vec<AppliedEdit>`; consume `DocumentConfig::state`, `DocumentConfig::allows`, existing source positions and protected regions. Adapt the private `process` producer callback to receive the parsed document and document configuration; keep synthetic edit tests.

- [x] Add failing integration tests for greedy wrapping, exact width, UTF-16, indivisible inline syntax, overlong tokens, disabled MD013/fix flags, configure-file, enable/disable directives, preservation, syntax hazards, and second-pass idempotence.
- [x] Run `mise exec -- cargo test --test reflow`; verify behavior assertions fail against the no-op formatter.
- [x] Implement `propose` with single-line root-paragraph eligibility, ASCII-space break candidates in unprotected text, configured `line_length` (default 80, minimum 1), source-local newline selection, and reparsing safety checks. Do not grant edits any new protected-region privileges.
- [x] Treat each paragraph as an all-or-nothing permission unit. Preserve source directives byte-for-byte and ensure inserted lines do not detach disable-next-line from its intended target.
- [x] Run `mise exec -- cargo test --test reflow` and `mise exec -- cargo test --all-targets`.

## Task 2: CLI behavior, evidence, and accurate status

**Files:** Modify `tests/config_cli.rs`, `README.md`, `docs/compatibility/parser-evaluation.md`, and roadmap status. Add `tools/parser-oracle/reflow.mjs` and its evidence fixtures if the installed pinned oracle is available.

- [x] Add failing CLI tests: configured stdin output, check exit 1 without writes, write output and second-pass check exit 0, and `fix: false` preservation.
- [x] Run `mise exec -- cargo test --test config_cli`; verify new tests pass through the library implementation, adding no alternate CLI formatting path.
- [x] Compare representative output against actual pinned upstream parsing and lint observations, distinguishing formatter policy from lint exceptions. Record reproducible evidence, not a broad compatibility claim.
- [x] Document supported wrapping and all remaining milestone-4 work; preserve incomplete rule coverage metadata.
- [x] Run `mise exec -- cargo test --all-targets`, `mise exec -- cargo fmt --check`, and `mise exec -- cargo clippy --all-targets -- -D warnings`; review the complete diff for scope and preservation regressions.

## Review follow-up

The independent review found upstream math/directive and raw-HTML preservation
gaps, plus quadratic edit/permission checks. Added failing regressions, conservative
document-level preservation guards, indexed edit eligibility, source reuse, and a
development scaling smoke test. The expanded pinned oracle covers 20 cases.

## Execution

The user requested uninterrupted continuation. Execute natively in the sibling worktree without a question/approval handoff. No commit or merge was requested; leave the changes available for review.

This initial slice was subsequently extended by
`2026-10-27-md013-multiline-and-diagnostics.md`. The user then requested committing
and merging the completed work into main; that request supersedes the original
leave-uncommitted ending above.
