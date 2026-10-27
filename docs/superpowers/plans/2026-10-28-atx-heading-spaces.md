# ATX Heading Spaces Implementation Plan

> For agentic workers: REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Goal: Implement safe MD019/MD021 heading-separator repairs and diagnostics.

Architecture: Derive opening/closing ATX marker and separator ranges from parsed
heading source. Delete only separator units after the first unit, matching the
pinned rule; preserve the first space or tab. Run in the stable heading stage
alongside MD026 so style changes and newly exposed closing markers settle before
blank-line repairs and reflow.

Tech Stack: Existing Rust source/edit pipeline; pinned markdownlint development oracle.

Spec: `docs/superpowers/specs/2026-10-21-mdfmt-design.md`, MD019/MD021 safe
repair classifications in `docs/compatibility/rules.json`, pinned
`lib/md019-md021.mjs` and `helpers/micromark-helpers.cjs:getHeadingStyle`.

## Global Constraints

- Must respect the effective supported markdownlint configuration.
- Must preserve content and Markdown meaning rather than force an unsafe fix.
- Must preserve tables. Violations involving tables may remain unresolved.
- Must be idempotent: formatting the result again produces identical output.
- Continue on main without worktrees or approval interruptions.

## Review Focus

- Setext text beginning with `#` must not be mistaken for ATX source.
- Open headings use MD019; closed headings use MD021 for both separators,
  independently respecting each rule's enablement and directives.
- Tabs are source units, not expanded display columns; retain the first unit.
- Empty/hash-only headings, escaped closing markers and trailing whitespace need
  pinned runtime evidence rather than visual guesses about heading style.
- MD026 can expose a closing marker and switch which spacing rule applies; the
  whole heading stage must settle, with original edit coordinates and stable output.

## Task 1: Heading spacing rules and compatibility evidence

Files: Create `src/heading_spaces.rs`, `tests/heading_spaces.rs`, pinned oracle
script/receipt and `docs/compatibility/heading-spaces.md`; modify stable-heading
pipeline, coverage tests, CLI help, README and roadmap.

Interfaces: `heading_spaces::propose(&Source, &ParsedDocument, &DocumentConfig)
-> Vec<Edit>`; `heading_spaces::diagnose(&Source, &ParsedDocument, &DocumentConfig)
-> Vec<Diagnostic>`. All repairs are deletions and reuse original-coordinate
composition and exact authorization.

- [ ] Add failing literals: `#  Hello` → `# Hello` (MD019),
  `#  Hello  ###` → `# Hello ###` (MD021); single separators unchanged;
  independent disabling, aliases/tags, directives, fix false, severity,
  containers, BOM/CRLF, tabs and hash-prefixed setext headings.
- [ ] Run `mise exec -- cargo test --test heading_spaces`. Expected: missing
  fixes/findings.
- [ ] Implement source boundaries and pinned style-dependent rule dispatch.
  Authorize only exact whitespace deletions, never marker/content edits.
- [ ] Add combined MD026 exposure and all-rule original-edit/idempotence tests.
  Record pinned runtime observations and native diagnostic/fix comparisons.
- [ ] Run full Rust suite, fmt/clippy, rebuild CLI and all implemented-rule
  oracles. Expected: all pass.
- [ ] Obtain independent read-only review; fix Important/Critical findings
  RED→GREEN, rerun the full suite, document coverage and commit to main.
