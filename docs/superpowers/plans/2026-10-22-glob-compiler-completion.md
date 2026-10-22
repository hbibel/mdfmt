# Glob compiler completion implementation plan

> For agentic workers: use executing-plans, test-first implementation and one independent final review. The user delegated decisions and requested continuous execution toward milestone 3 completion; do not pause for batch approvals.

Goal: replace isolated substitutions with the pinned picomatch token grammar and close advanced native selection gaps without invoking Node at runtime.

Architecture: compile the fixed cli2 matching options from picomatch 2.3.2 source into ECMAScript regexes. Execute them with the native, fuel-bounded `regress` engine vendored from `d0684188b1c39275f3caaab578bd94a9c0c5bc25` (MIT/Apache, unsafe prohibited, UTF-16 enabled), with a regression-tested nested-loop state correction. Keep brace enumeration and directory pruning distinct from override/ignore filtering. A separately evaluated full Rust picomatch port disagreed on six existing fixtures and is not the compatibility reference.

Spec: `docs/compatibility/configuration.md` and milestone 3 of `docs/superpowers/specs/2026-10-21-mdfmt-design.md`.

## Constraints and review focus

- Preserve existing uncommitted optional/repeated work and all recorded runtime observations; replacement is allowed only after the new native implementation satisfies their actual results.
- Never read `.env`, execute configuration, use Node at runtime, or implement deferred custom regex/package configuration.
- Regex exhaustion and allocation/nesting limits must produce errors, never false nonmatches.
- Preserve UTF-16 accounting, exact-input filtering shortcuts, negative/extglob list ordering, escaping, quotes, and legacy class/range backtracking.
- Match enumeration brace expansion and ancestor pruning, not just whole-path regex results.
- Do not merge, push or create another commit without a request. Keep supported-policy differences explicit and do not invent non-Linux runtime evidence.

## Task 1: Pinned token compiler and bounded execution

- [x] Add a new regression test comparing all existing direct observations with `picomatch::Pattern`; initially fail because that API does not exist.
- [x] Create `src/config/picomatch/{mod,compile,safety}.rs`. `Pattern::new(pattern: &str, posix: bool)` compiles the fixed dot-enabled native grammar; `regex_matches(path: &str) -> Result<bool, ConfigError>` executes UTF-16 regexes with explicit fuel errors. Retain token value/output separation so upstream backtracking is reproduced, not approximated.
- [x] Implement all five extglob operators, nested/repeated safety rewrites, ordinary groups/quantifiers, quotes, POSIX classes and legacy range reconstruction. Pin engine source in Cargo.lock and retain upstream MIT attribution.
- [x] Run targeted comparisons RED→GREEN, including formerly unsupported cases. Add safety/invalid-range tests before their implementation.

## Task 2: Enumeration and filtering integration

- [x] Add full upstream scenarios for negative/nested extglobs, range suffixes, ordinary-group interactions, brace expansion, cross-segment pruning and escaped paths; remove nativeError only when native behavior matches actual observations.
- [x] Extend native brace enumeration to return expanded pattern alternatives separately from filtering. Compile traversal segment matchers before walking; use upstream partial-match pruning where path alternatives cross groups.
- [x] Change `GlobMatcher::is_match` to `Result<bool, ConfigError>` and propagate errors through enumeration, exclusions, overrides and ignores. Preserve literal paths and intentional missing-file policy.
- [x] Remove obsolete substitution modules/tests only after their actual behavior is covered by the new oracle comparisons. Run every Rust test and pinned oracle.

## Task 3: Remaining configuration boundary and platform evidence

- [x] Audit non-glob milestone blockers independently while implementing the compiler; close concrete required gaps with pinned failing regressions rather than broadening policy exclusions.
- [x] Add reproducible generated glob comparisons and portable test/CI coverage. Distinguish local Linux results from configured but unexecuted platform jobs.
- [x] Run Rust tests, Node oracles, rustfmt, Clippy and whitespace checks; obtain one final independent review and fix Important findings test-first.
- [x] Update milestone checklist/evidence based on actual completed requirements. If an external validation blocker remains, state it plainly rather than claiming an unverified milestone complete.

Outcome: milestone 3's first-release native configuration contract is implemented.
Verification covers 69 application tests, 537 dependency tests and all recorded
upstream comparisons. Remote platform jobs are defined but were not run locally;
that limitation is recorded rather than claimed away. All changes after checkpoint
`48e2925` remain uncommitted for the user.
