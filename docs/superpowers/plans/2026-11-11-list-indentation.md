# List Indentation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add native MD005/MD007 diagnostics and structurally validated indentation repairs, advancing built-in coverage from 50/53 to 52/53.

**Architecture:** A private source-backed list snapshot supplies sibling prefixes, original ranges and list ancestry. Rule-specific consumers classify the pinned indentation requirements; only localized whitespace proposals that preserve parsed ownership/content are authorized. Final diagnostics describe the formatted output.

**Tech Stack:** Existing Rust toolchain, `markdown`, `serde_json`; pinned markdownlint development oracle only, no new dependency.

**Spec:** `docs/superpowers/specs/2026-10-21-mdfmt-design.md`, `docs/compatibility/rules.json`, pinned `lib/md005.mjs`/`lib/md007.mjs` and existing MD029/MD030/list-flow compatibility evidence.

## Global Constraints

- Native Rust; use `mise.toml`, tracked `Cargo.lock` and the existing sibling worktree.
- Respect supported configuration, disabled rules, inline permissions, aliases and severity.
- Preserve Markdown meaning, code values, tables, definitions, links and directive targets. Do not force indentation changes that alter ownership.
- Preserve original inline links; never convert to references. MD013 and approved MD026 behavior remain unchanged.
- Use original-byte/UTF-16 ranges even when expanded diagnostic ownership is needed for tabs.
- Idempotence and regression coverage are mandatory. Source examples alone are not native implementation evidence.
- Execute inline without routine questions, with one independent review and one Important/Critical fix pass. No merge/push or `.env` reads.
- Do not begin implementation while the literal-content batch review/fix pass is active; this document is independent preparation.

## Decisions and assumptions

- Initial repairs change only an item's whitespace before its marker. Multiline/container shifts remain diagnostic when a localized edit cannot preserve semantics; do not rewrite code/table continuation lines merely to meet a target.
- Reuse `diagnostic_lists` for classification, not as edit authorization. Native parsing of the original candidate must independently validate every proposed change with existing preservation/permission gates.
- MD005 unordered repairs run only when MD007 is disabled for all item prefixes in that list. Otherwise MD007 owns unordered indentation; this avoids oscillation under partially disabled/conflicting anchors. Remaining findings are not hidden.
- Ordered right-alignment follows the pinned MD005 transition from indentation to marker-end matching; do not normalize intentionally aligned `9.`/`10.` prefixes into a new numbering policy.
- MD007 counts unordered ancestors only until a quote or ordered-list boundary, ignores unordered lists under ordered ancestry as pinned, and applies the pinned footnote base. Quote-prefix adjustment is physical-line-specific.
- Bound indentation arithmetic and allocation before constructing replacements. Huge impossible targets must not allocate unbounded strings or panic; preserve source and report remaining findings, or return an explicit configuration/resource error where arithmetic cannot be represented.
- MD051 and conflict reporting beyond these two consumers, external formatter integration, later declarative compatibility and release hardening remain explicit later work. 52/53 is not roadmap completion.

## Review Focus

- Changing sibling indentation must not move paragraphs, code, tables, definitions or child lists between owners; both tasks include literal preservation and rejected-edit cases.
- Ordered marker widths and end-alignment transitions must remain independent of MD029 numbering and MD030 padding; Task 1 pins their combined behavior.
- Quote/list boundaries, ordered ancestors, footnotes, tabs and generic HTML recovery must not change MD007 nesting or physical ranges; Task 2 owns these scope cases.
- Disabled first anchors and partial MD005/MD007 permissions must settle without oscillation or editing disabled bytes; Task 2 pins delegation and repeated-call stability.
- Dense/deep lists and huge configured targets must not introduce repeated whole-line/document work, overflow or uncontrolled allocation; both tasks include bounded real-parser work/resource cases.

## File map and interfaces

- Create `src/parse/list_indentation.rs` for source-backed list/prefix ownership; export it from `src/parse.rs`.
- Create `src/md005.rs`, `tests/md005.rs`, then `src/md007.rs`, `tests/md007.rs`.
- Create `tools/parser-oracle/list-indentation.mjs` and per-rule ordinary/policy receipts, keeping source/runtime evidence separate from native output checks.
- Integrate authorized proposals and final diagnostics in `src/lib.rs`; update CLI help, coverage assertions, README, inventory, roadmap and `docs/compatibility/list-indentation.md` with each completed rule.
- Task 1 produces `ListIndentation { ordered: bool, span: Range<usize>, items: Vec<ListItemIndentation> }`, `ListItemIndentation { span: Range<usize>, marker: Range<usize>, padding_end: usize, indent: usize, marker_end_column: usize }` and `list_indentation(&Source) -> Vec<ListIndentation>`.
- Task 2 adds `ul_depth: Option<usize>`/`footnote_base: usize` to lists and `quote_adjustment: usize` to item records only alongside consuming tests. All range fields remain original-source bytes; numeric columns follow pinned physical UTF-16 conventions, not synthesized display columns.
- Both consumers expose `propose(&Source, &ParsedDocument, &DocumentConfig) -> Vec<Edit>` and `diagnose(&Source, &ParsedDocument, &DocumentConfig) -> Vec<Diagnostic>`. Source snapshots never authorize edits directly.

## Task 1: MD005 sibling indentation

- [x] Record ordered/unordered siblings, right-aligned marker widths, empty items, nested quotes/lists, tabs, metadata, code/math/HTML recovery, aliases, warnings, disabled first anchors and BOM/CRLF. Expected: source-only receipts, no native implementation claim.
- [x] Write `sibling_indentation_reports_and_repairs_only_preserved_structure`: `"- a\n - b\n"` under MD005 alone reports line 2, range `[1,3]`, detail `Expected: 0; Actual: 1` with fixes disabled; safe native formatting yields `"- a\n- b\n"`, no remaining finding and stable second output. Record this native repair separately from upstream's no-edit unordered behavior.
- [x] Write `ordered_alignment_is_not_numbering`: `"1. a\n 2. b\n"` diagnoses range `[1,4]`, expected 0/actual 1 and repairs the extra space; `" 9. a\n10. b\n11. c\n"` remains unchanged and clean. Add MD029/MD030 composition and protected multi-block rejection cases.
- [x] Run `mise exec -- cargo test --test md005`. Expected: missing diagnostics/repairs before implementation, not broken fixtures.
- [x] Implement the exact snapshot and MD005 source-backed classification. Propose only bounded whitespace insertion/removal at a marker, with full permission and original-parser equivalence checks. Retain unsafe findings instead of shifting content.
- [x] Run native source/fix/no-fix/idempotence oracle, affected list/parser/reflow suites, full optimized Rust suite, fmt/clippy/diff. Expected: all tests/assertions pass, no weakened budgets or content changes.
- [x] Update truthful evidence/coverage to 51/53 and commit `feat: normalize safe sibling list indentation`.

## Task 2: MD007 unordered nesting indentation

- [x] Record every option (`indent`, `start_indented`, explicit/defaulted `start_indent`) and nesting boundaries, physical quote-prefix adjustments, ordered ancestors, footnotes, partial directives, tabs, empty/loose items, HTML recovery, UTF-16/BOM/newlines and warnings. Never run an upstream huge-allocation case unbounded.
- [x] Write `nested_unordered_indentation_preserves_ownership`: `"- a\n  - b\n"` with `indent: 4` reports line 2, range `[1,4]`, detail `Expected: 4; Actual: 2` in no-fix mode; safe native output is `"- a\n    - b\n"` with unchanged content/nesting and no remaining finding.
- [x] Write `root_start_indent_and_quote_adjustments_are_physical`: `"  - a\n"` under defaults repairs to `"- a\n"`; `"- a\n"` with `start_indented: true` repairs to `"  - a\n"`; `"> - a\n>   - b\n"` with `indent: 4` diagnoses line 2/range `[1,6]` and safely yields `"> - a\n>     - b\n"`.
- [x] Add rejection cases for code/table/definition ownership changes, ordered-ancestor/footnote tests, partially disabled MD005/MD007 anchors, large/deep workloads and huge-target termination. Check all diagnostic/CLI/severity/fix-mode contracts and second-call stability.
- [x] Run `mise exec -- cargo test --test md007`. Expected: missing MD007 findings/repairs before implementation.
- [x] Add requested ancestry fields and implement pinned expected indentation. Integrate marker-only proposals with preservation validation and MD005 delegation; do not add a competing whole-subtree reindent path.
- [x] Run both new native oracles and existing MD004/MD027/MD029/MD030/list separator/reflow/parser suites, then the complete optimized suite, fmt/clippy/diff. Expected: output converges in one invocation, disabled/protected bytes remain unchanged, no silent overflow or unbounded padding.
- [x] Update evidence/roadmap/coverage to 52/53 and commit `feat: apply ownership-safe unordered list indentation`.
- [x] Request one independent review of this batch with every Review Focus and ledger ruling. Reproduce/fix Important/Critical findings once, then run the complete suite; no second review.
- [ ] Retain all rulings/evidence, remove only this completed plan's scratch workspace and continue MD051 and later roadmap work without merge/push or routine questions.

## Planning check

Runtime probes confirmed the literal findings, ranges and upstream repairs above.
Task 1 produces the precise prefix fields Task 2 extends; no unused ancestry
extraction is introduced early. Both tasks include consuming tests for all review
focus items. Conditional whitespace changes are independently authorized through
original parsed ownership, not inferred from a diagnostic acceptance example.
The literal-content batch's independent review and fix pass closed at `485fc3f`.

## Execution evidence

Task 1's five primary tests failed before implementation; all seven tests now
pass. The 687-row native comparison checks diagnostics, upstream ownership/code
values and idempotence; 156 intentional upstream-fix differences are retained.
The complete optimized suite passes 482 tests across 67 targets, with fmt/clippy
clean. Prior MD004/MD027/MD029/MD030 native comparisons also pass. Coverage51/53;
MD007 and the batch review remain pending.

Task 2's seven initial tests failed before implementation. Twelve MD007 and eight
MD005 tests now pass, with 1,680/687 native comparison rows and 261/156 retained
upstream-fix differences. Two bounded conflict regressions independently exposed
MD007/MD027 and partially disabled MD005/MD027 cycles; the shared candidate gate
rejects added whitespace overlapping enabled MD027 findings. Direct MD013-only
and list-composition tests also reproduced prefix growth before protected inline
siblings; the enclosing paragraph now bounds soft-break prefix consumption.

The complete optimized suite passes 496 tests across 69 targets; fmt/clippy clean.
Existing list, 288 MD013/440 MD026 and 29 reflow comparisons pass. Debug scaling
hit ten seconds at 500 paragraphs; optimized 100/500/1,000 cases pass with the same
limit (211/1,117/2,509 ms). The scaling fixture now selects only MD013 instead of
accidentally testing later default MD041. Coverage52/53; review remains pending.

The one independent review over `1adf735..71350f7` found two Important findings,
no Critical findings and no minors. All five primary reproductions failed first:
four bounded normalization cycles with MD023/MD030 and one intermediate-target
allocation abort under Linux128MiB, including fixes disabled. The single fix pass
adds seven public regressions in `tests/list_indentation_review.rs`.

The candidate gate now includes MD023/MD027/MD030 at both inserted bytes and
collapsed deletion boundaries. Prior normalizers retain deterministic priority;
remaining indentation findings stay visible. Disabled-rule/line controls still
permit safe repairs. Classification retains numeric expectations and ranges only;
proposers check aggregate padding<=input bytes+4 before allocating replacements,
so expanded output<=2*input bytes+4. Disabled fixes no longer build authorization
proposals. Safe dense start indentation still produces exact original edits.

Final complete optimized verification passes503/503 across70 targets; fmt/clippy
clean. Both new native oracles and affected MD023/MD027/MD030/MD029/MD013/MD026/
reflow comparisons pass. Original reviewer probes now terminate, and the16KB/
4000-item intermediate target preserves all4000 findings under128MiB in both fix
modes. No second review, iteration cutoff or weakened CPU/memory budget.

The focused debug run passes six review tests, but
`admitted_intermediate_targets_keep_linear_memory_in_both_fix_modes` exceeds its
30-second CPU budget after the disabled control completes, without an allocation
abort. This and the older debug scaling timeout remain hardening obligations;
the optimized run executes the same assertions/limits and is the final full gate.
Coverage remains52/53; MD051 and later roadmap tasks continue after closure.

Rulings so far:

1. Share bounded whitespace selection/permission/original-parser validation in
   the list snapshot module. Cost if wrong: private helper placement, not a
   weaker preservation gate; unsafe-content and upstream ownership checks guard it.
2. Run MD005 after padding/numbering in the normalization loop. Cost if wrong:
   cross-rule instability; composition and repeated-output tests gate the order.
3. Keep MD027 priority when sibling/nesting insertions overlap its candidate
   findings. Cost if wrong: conservative withholding; remaining indentation
   findings stay visible instead of a nonterminating fix cycle.
4. Use read-only candidate findings, not original directive-byte permissions, to
   detect that conflict. Cost if wrong: withholding even when a quote repair would
   itself be unsafe; no changed-line permissions or iteration cap hide the issue.
5. Fix the discovered shared MD013 idempotence defect before this batch's review.
   Cost if wrong: broader reflow-gap boundary surface; literal protected-token/
   quote/list/newline cases, full suite and prior oracle guard the one-line change.
6. Isolate MD013 in its scaling fixture and add an explicit optimized-profile
   mode without changing the ten-second budget. Cost if wrong: debug-only
   performance regressions remain hardening obligations, explicitly not claimed
   passing by the optimized timings.
7. Check heading/quote/padding conflicts in both edit directions, including
   collapsed boundaries. Cost if wrong: conservative withholding where an opposing
   finding cannot itself be repaired; diagnostics/disabled controls remain intact.
8. Separate classification from allocation and cap total padding at input bytes+4.
   Cost if wrong: unusually large safe batches remain diagnostic; dense positives
   and bounded-memory intermediate targets guard useful repairs and resources.
9. Skip authorization proposals when either fix control is false. Cost if wrong:
   an undiscovered private proposer side effect disappears; existing no-fix/CLI/
   config regressions and full tests gate the pure-proposal contract.
10. Keep optimized verification as the final gate rather than relax debug CPU
    limits. Cost if wrong: debug-only throughput problems remain hardening work,
    explicitly named rather than represented as a completed debug run.
11. Retain approved repair differences, as the reviewer set aside. Cost if wrong:
    different fixed bytes; native diagnostics/ownership/value evidence stays
    separate from exact upstream edits.
12. Keep subtree/continuation/code/table/definition/link rewrites outside this
    subset, as the reviewer set aside. Cost if wrong: some fixable findings need
    a later safe adapter instead of risking content now.
13. Keep exact nativeu128 arithmetic, as the reviewer set aside. Cost if wrong:
    intentional huge-option detail differences; unsafe upstream allocation probes
    are not executed.
14. Defer general conflict explanations/UI, as the reviewer set aside, not stable
    termination. Cost if wrong: remaining rule findings lack a dedicated conflict
    explanation until milestone5's remaining work.
15. Do not infer a debug regression without a baseline, as the reviewer set aside.
    Cost if wrong: an older/newer debug slowdown remains to isolate in hardening;
    observed failure and optimized timings stay separately disclosed.
16. Keep MD051/formatters/later compatibility/release completion separate, as the
    reviewer set aside. Cost if wrong: unfinished deliverables remain explicit at
    52/53 instead of being hidden by batch completion.
17. Do not infer exhaustive parser/corpus/fuzz/platform guarantees from bounded
    Linux review, as the reviewer set aside. Cost if wrong: untested platforms and
    input classes remain release-hardening work.

Both Important findings are fixed; no deferred minors were reported.
