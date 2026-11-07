# Remaining Content Diagnostics Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add source-preserving MD001, MD025, MD036, MD041, MD043 and MD059 findings as the next built-in coverage batch.

**Architecture:** Reuse source-aware heading extraction and native parsed inline nodes. Compare final-output diagnostics against pinned markdownlint 0.42.0 receipts, retaining byte-valid locations and configured permissions. No diagnostic in this batch authorizes content edits.

**Tech Stack:** Rust 1.99, existing markdown/configuration pipeline, pinned Node development oracle.

**Spec:** `docs/superpowers/specs/2026-10-21-mdfmt-design.md`

## Global Constraints

- Must respect the effective supported markdownlint configuration.
- Must preserve content and Markdown meaning rather than force an unsafe fix.
- Must preserve tables. Violations involving tables may remain unresolved.
- Must report remaining violations when the effective configuration flags them.
- Must be idempotent: formatting the result again produces identical output.
- Execute inline in the sibling worktree without routine approval pauses, as requested.

## Review Focus

- Disabled reference headings still affect history; directives filter findings, not structural state.
- Front-matter title defaults differ from the schema literal; empty patterns disable title recognition.
- Nested containers, raw HTML, Docfx tabs and multiline setext headings must not be flattened into incorrect title history.
- Required-heading wildcards and loops must terminate; unsupported regex headings must receive explicit errors.
- Unicode, BOM, mixed line endings, protected inline nodes and prose reflow must retain valid final-output locations and stable findings.

## Task 1: Heading structure diagnostics

**Files:** add `src/heading_structure.rs`, `tests/heading_structure.rs`, `tools/parser-oracle/heading-structure.mjs` and receipt; reuse `src/md024.rs` heading extraction; integrate in `src/lib.rs`.

**Interfaces:** `heading_structure::diagnose(&Source, &ParsedDocument, &DocumentConfig) -> Vec<Diagnostic>`; shared heading snapshots expose original start, level and raw text.

- [x] Record pinned MD001/MD025/MD041/MD043 findings for title metadata, heading levels, containers, HTML, references, wildcard/loop requirements, directives, BOM and newline variants.
- [x] Add failing receipt and literal regression tests, plus CLI check/write/warning tests; run `mise exec -- cargo test --test heading_structure` and observe missing findings.
- [x] Implement heading history, title detection, first-content selection and required-heading matching; retain explicit errors for deferred JavaScript regex patterns.
- [x] Run heading and existing duplicate-heading tests; expected all pass.

## Task 2: Content-dependent emphasis and link findings

**Files:** add `src/md036.rs`, `src/md059.rs`, `tests/content_diagnostics.rs`, `tools/parser-oracle/content-diagnostics.mjs` and receipt; integrate in `src/lib.rs`.

**Interfaces:** each module exposes `diagnose(&Source, &ParsedDocument, &DocumentConfig) -> Vec<Diagnostic>`.

- [x] Record pinned root/nested emphasis and inline/reference link findings, custom options, punctuation, HTML/code exclusions, Unicode, multiline labels and directives.
- [x] Add failing receipt and preservation tests; run `mise exec -- cargo test --test content_diagnostics` and observe missing findings.
- [x] Implement source-based matching without renaming links or converting emphasis to headings.
- [x] Run content tests; expected all pass and repeated formatting stable.

## Task 3: Verification and roadmap evidence

**Files:** update README, CLI help, rule inventory, roadmap and add `docs/compatibility/content-diagnostics.md`.

- [x] Run `mise exec -- cargo test --all-targets`, `mise exec -- cargo fmt --check`, `mise exec -- cargo clippy --all-targets -- -D warnings`, and new/affected pinned runtime comparisons; expected all pass.
- [x] Review the changes and address important or critical findings with failing regressions before fixes.
- [x] Record only important findings and critical flaws in the roadmap, distinguish completed work from remaining milestones, and leave execution evidence.

Verified 2026-11-07: 320 Rust tests across 42 targets; fmt/clippy clean; all 27
existing rule/reflow oracle scripts pass, plus 1,500 heading and 554 content
diagnostic CLI comparisons. Independent review found five Important issues and
no Critical issues; each Important issue has a failing regression and verified
fix. See `docs/compatibility/content-diagnostics.md` for policy boundaries.

## Continuation after this batch

Remaining built-ins: MD003, MD005, MD007, MD022, MD031, MD032, MD033,
MD034, MD037, MD038, MD039, MD044, MD046, MD049, MD050, MD051,
MD052, MD053, MD054, MD060. Then complete rule interactions/conflicts,
external formatter adapters, custom patterns/installed package resolution,
and release hardening. This batch does not complete the overall roadmap.
