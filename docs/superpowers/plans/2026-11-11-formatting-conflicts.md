# Formatting Conflicts Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete milestone 5's interaction/convergence and explicit conflict reporting without claiming formatter integration, later compatibility or hardening complete.

**Architecture:** Bound normalization using exact phase/source states and a transaction-level fallback, then surface structured reasons from existing list/separator conflict gates. Keep safe deterministic priorities and final rule findings; do not guess that every remaining violation is a conflict.

**Tech Stack:** Existing Rust toolchain and standard collections; existing native/upstream development oracles. No new runtime dependency.

**Spec:** `docs/superpowers/specs/2026-10-21-mdfmt-design.md`, existing `list-indentation.md`, `block-separators.md`, `md013.md` and per-rule preservation contracts.

## Global Constraints

- Native Rust; use `mise.toml`, tracked `Cargo.lock` and existing sibling worktree. No `.env` reads, merge, push or configuration/session move.
- Respect rule enablement, inline directives, aliases, severity and both fix controls. Preserve code, tables, metadata, labels and inline links; do not convert links to references.
- Keep MD013 safe configured-width reflow and MD026's approved punctuation removal. Report unresolved configured violations on final output and require idempotence.
- Diagnostic recovery never authorizes edits. All existing original-parser/protected-byte checks remain.
- Inline execution, regression-first changes, one independent review and one Important/Critical fix pass; no second review.
- Close the active MD051 review/fix pass before implementing this independent next batch.

## Decisions and assumptions

- Exact repeated source at the same normalization phase proves a cycle; do not use hash-only equality or an arbitrary cutoff as a successful result.
- On a detected cycle, discard the entire attempt's edits and return the original source with ordinary no-fix diagnostics plus a `rule-conflict` diagnostic. Running that result again follows the same transaction/fallback, so output and original bytes remain stable.
- Bound stored exact source states to 32 times original input bytes plus 1 MiB, and 256 observations per phase. Exhaustion is a distinct `ProcessingError::WorkLimit` (CLI exit 2), not a guessed rule conflict or successful partial output. File writes remain untouched on processing failure.
- Guard each existing normalization loop at its iteration boundary with a distinct phase key. Also settle the complete formatting pipeline until no output changes; this closes later-stage/earlier-stage dependencies in one invocation. Preserve original-coordinate edit composition across passes.
- A normalizer cycle message names actual contributing edit origins, not a claimed minimal unsatisfiable rule set. Existing typed candidate gates retain their narrower priorities and explanations.
- A `rule-conflict` diagnostic uses `rule_id: None`; severity is Error when any contributing enabled rule is Error, otherwise Warning. Warning-only configurations must not gain an error exit merely from the explanation. Disable either relevant rule to remove a known pair's explanation.
- Known-pair explanations require actual document-specific rejected proposals: list indentation opposed by MD023/MD027/MD030, or separator requests capped by MD012. Do not flag option combinations when no affected Markdown exists, the competing line is disabled, or preservation alone prevented an edit.
- Keep existing priority: heading/quote/padding normalization over competing list whitespace, MD012's blank maximum over excess separator insertion. Preserve ordinary remaining rule findings alongside explanations.
- Leave `Coverage.complete` false at53 built-ins: diagnostic-only subsets, later unsupported options and parser fidelity are separate from milestone 5's tested scope.

## Review Focus

- Valid conflicting settings and warning-only/partially disabled variants must terminate with stable source and appropriate exits, not bounce across passes; both tasks own actual-rule/CLI controls.
- Work/source-state limits must prevent quadratic history allocation without mislabeling a resource failure as a configuration conflict; Task 1 tests exact-state and bounded-resource behavior.
- Cross-stage heading/reflow/list/fragment/style/separator changes must compose original byte ranges and settle within one invocation, without rolling back MD026 or editing protected content on ordinary successful paths; Task 1 owns the matrix.
- Conflict explanations must use original/final physical spans and effective permissions, never candidate directive byte ranges or global option guesses; Task 2 owns newline/container/directive scope cases.
- Conflict analysis must not add per-token document reparsing or manufacture explanations for missing content/unsafe repairs; Task 2 tests dense and negative controls.

## File map and interfaces

- Create `src/stabilize.rs` for exact-state tracking and private attempt outcomes; create `src/conflicts.rs` for shared conflict records/severity/rendering.
- Modify `src/lib.rs` to wrap the existing formatting pass, guard loop phases, settle complete passes, preserve composed edits and produce stable fallback diagnostics.
- Add `ProcessingError::WorkLimit(String)` in `src/diagnostic.rs`, with processing-failure display/CLI/file-preservation tests.
- Extend private list/separator evaluation to return structured outcomes while keeping existing `propose(...) -> Vec<Edit>` wrappers for callers. No global mutable conflict registry.
- Create `tests/formatting_conflicts.rs`, extend appropriate CLI/preservation/composition tests, add `docs/compatibility/formatting-conflicts.md`, update README/roadmap truthfully after verification.
- Task 1 produces `Phase` (one variant per loop plus complete pass), `StateTracker::new(input_bytes: usize)`, and `observe(phase: Phase, source: &str, origins: &[String]) -> Result<(), AttemptFailure>`. `AttemptFailure::{Cycle { rules: Vec<String> }, Processing(ProcessingError)}` stays private. It also produces `RuleConflict { span: Range<usize>, rules: Vec<String>, retained: Option<String> }` and `diagnose_conflicts(&Source, &DocumentConfig, &[RuleConflict]) -> Vec<Diagnostic>` for Task 2.
- Task 2 extends existing private evaluation to `RuleProposals { edits: Vec<Edit>, conflicts: Vec<RuleConflict> }`; `md005::propose`/`md007::propose` keep their Vec signature, while final conflict collection consumes their structured sibling evaluation. Separator budgeting similarly returns capped count plus an optional typed conflict, retaining the existing edit policy.

## Task 1: Transactional convergence and phase/source budgets

- [x] Write exact-state unit tests for the production tracker: same text in different phases is allowed; repeating the same phase/text after actual changes returns Cycle with contributing origins; distinct states never collide through hash-only shortcuts; history/observation limits return WorkLimit before excessive source cloning. Test unchanged/no-fix paths create no spurious cycle.
- [x] Write bounded integration tests for existing real-rule reverse edits and priority controls, including list heading/quote/padding cases and warning-only/disabled variants. Use the existing individual-rule forward/reverse controls rather than weakening their semantic gates to force a cycle.
- [x] Write a cross-stage public matrix spanning MD026/MD003, MD013 linked prose, MD005/MD007/MD029/MD030, emphasis/strong and separators/MD051. Assert final diagnostics, source preservation, exact original-edit reconstruction and no edits on a second call; include BOM/CRLF and no-fix/config.fix:false.
- [x] Run `mise exec -- cargo test --release --test formatting_conflicts --lib`. Expected: missing guard/conflict behavior before implementation; fixtures must not depend on unsupported settings.
- [x] Implement the exact phase-state tracker and complete-pass settlement. On proven cycle, call the no-fix diagnostic path on original source and add the shared conflict diagnostic; do not return transient candidate output/edits. Resource exhaustion returns explicit WorkLimit and performs no write. Ordinary successful paths retain all rule-prescribed edits and original coordinates.
- [x] Run focused/CLI suites, the complete optimized suite, fmt/clippy/diff and relevant native reflow/heading/list/style/reference comparisons. Expected: unchanged assertions/CPU limits, warning-only exits remain non-error, all normal successful cases still settle safely.
- [x] Update evidence and commit `fix: make formatting convergence bounded and transactional`.

## Task 2: Document-specific conflict explanations

- [x] Write `opposed_indentation_reports_the_retained_rule_priority`: `"> - a\n"` with MD007 start_indented and MD027 reports stable MD007 plus `rule-conflict` naming MD007/MD027 and retained MD027. Add MD023 heading, partially disabled MD005 anchor and bidirectional MD030 nesting cases; disabled opposing rules/lines and list_items:false retain safe repairs without an explanation.
- [x] Write `blank_separator_budget_explains_only_actual_conflicts`: `"one\n# Heading\ntext\n"` with MD012 maximum1 and MD022 lines_above/lines_below2 formats to one blank on each side, retains ordinary MD022 findings and explains the MD012 cap. Controls with no heading, MD022 disabled or compatible maximum2 have no conflict. Include warnings, comments/quotes, newline variants and partially disabled blank lines.
- [x] Write negative cases for diagnostic-only missing targets/content, unsafe code/table ownership and huge resource targets: those are not automatically competing-rule conflicts. Add CLI repeated check/write/fix-control/status and exact count tests, plus dense bounded evaluation with disabled-conflict bypass.
- [x] Run `mise exec -- cargo test --release --test formatting_conflicts`. Expected: missing document-specific explanations, not altered formatting priorities.
- [x] Implement structured list/separator proposal outcomes and shared final diagnostics. Map candidate overlaps back to original source anchors before reporting; line states may be reused across whitespace-only candidates, original directive byte ranges may not. Sort/deduplicate by physical span and rule set; preserve normal rule findings.
- [x] Run all prior batch regressions/native comparisons and complete optimized suite, fmt/clippy/diff. Expected: no new content changes, all successful output idempotent, no conflict added solely because options coexist.
- [ ] Update evidence/roadmap for milestone 5's tested built-ins/interactions/conflicts without claiming later milestones complete; commit `feat: explain conflicting formatting requirements`.
- [ ] Request one independent review with every Review Focus and chronological ruling; fix Important/Critical findings once regression-first, then full verification. Retain all rulings/evidence before deleting only the completed scratch directory.
- [ ] Continue explicit code formatter adapters, later declarative compatibility and release hardening autonomously, without merge/push or routine questions.

## Planning check

Native probes confirm the supported MD012 maximum1/MD022 two-blank case already
retains one blank and ordinary MD022 findings; explanations, not a new edit policy,
are missing. An initial maximum0 probe was rejected by the pinned schema and is
not an application-conflict example. The prior list regressions establish actual
reverse proposals and preserved priorities. Task 1 produces exactly the private
tracker/conflict record Task 2 consumes; all Review Focus classes have owning tests.
MD051's independent review closed at `daa0b8c`, after one regression-first fix
pass and complete optimized522/522 across72 targets. All12 rulings/costs remain
tracked; only its completed scratch was removed. This plan may now begin in the
same branch/worktree, without configuration/session changes.

## Execution evidence and rulings

Task 1 committed as `e8c17eb`; exact-state, rollback and resource-limit tests
failed on missing APIs before implementation. Targeted severity misclassification
was separately RED-to-GREEN. Complete optimized Task 1 verification passed
533 tests across73 targets. Task 2's actual-list/separator/CLI tests failed on
absent explanations before implementation; eight public tests now pass, including
dense400-count workloads and negative permission/preservation/resource controls.
Complete optimized verification passes539/539 across73 targets, fmt/clippy clean;
all affected native list/separator/heading/reflow/style/fragment/reference gates
pass. Three prior exact-total/string assertions were extended to assert the new
explanations while retaining exact original findings/content/status checks.

1. Scope trackers to each logical loop invocation, including complete settlement,
   not reuse inner history across outer rounds. Cost if wrong: fixed-depth multiple
   budgets instead of one shared cache; false rollback controls guard normal paths.
2. Capture successful edit origins before composition cancels source changes.
   Cost if wrong: summaries name a conservative actual contributing set rather
   than a minimal conflict; original bytes and ordinary findings remain authoritative.
3. Exercise pathological cycle/growth through the real transaction with injected
   pass outputs rather than weaken real-rule safety gates. Cost if wrong: no
   real-rule CLI WorkLimit reproduction is claimed; library failure and CLI
   sequencing prevent candidate writes, and old failure-preservation tests remain.
4. Require bounded uncapped separator validation before explaining a cap. Cost
   if wrong: oversized/exhausted proof batches can omit explanations, retaining
   ordinary findings instead of inventing conflicts or reparsing per token.
