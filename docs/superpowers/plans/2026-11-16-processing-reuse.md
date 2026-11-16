# Processing Reuse Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Pass the existing debug list/prose resource gates without weakening limits or changing formatting decisions.

**Architecture:** Measure repeated parser/config construction first. Reuse one exact current Source/ParsedDocument/DocumentConfig snapshot inside a formatting pass, and one immutable recovered diagnostic parse per Source. Changed bytes or metadata context create fresh state; no global caches.

**Tech Stack:** Declared Rust/mise toolchain, standard OnceLock/Arc, existing native parsers and test/oracle scripts.

**Spec:** `docs/superpowers/specs/2026-11-16-release-hardening-design.md`, processing-reuse deliverable only. Windows transport and corpus/artifact work remain separate batches.

## Global Constraints

- Complete the custom-regex review correction pass before changing shared parser/processing sources for this plan.
- Keep the sibling roadmap-completion worktree and shared configuration; no session move, merge, push or publication.
- Preserve 4,000 list items, 128 MiB virtual-memory bound and 30-second subprocess limit in both fix modes.
- Preserve the prose scaling gate's 100/500/1,000 paragraphs and 10-second subprocess limit per case, in debug and release.
- Cache exact immutable source bytes and the current configured metadata context, not source length, hashes alone, filenames or mutable global state.
- Keep Source Send/Sync, original-byte coordinates, metadata protection/recognition stability, inline permissions, convergence scopes, explicit failures and edit provenance.
- One current processing snapshot per pass; discard it on byte changes. Recovered diagnostic results belong only to their immutable Source.
- One independent batch review and one regression-first Important/Critical correction pass; record Important/Critical findings in the roadmap, not polish notes.

## Review Focus

- Equal-length body/config edits cannot reuse stale parsing or directive permissions; owning `context_reuse_keeps_equal_length_changes_independent`.
- Custom metadata/default/no-match contexts and mapped/derived views cannot share cached diagnostic authority; owning `diagnostic_reuse_keeps_metadata_views_independent`.
- Invalid/hostile patterns and parse/config failures must remain fallible and cannot poison later inputs; owning existing `pattern_errors_are_transactional_and_recoverable` plus new context tests.
- Large flat lists and prose cannot exceed unchanged debug time/memory limits or retain every intermediate parse; owning existing list memory gate and prose scaling oracle.
- Cached recovered trees cannot be mutated by one rule and alter later rule findings/source positions; owning existing literal/HTML/list/heading consumer gates and `repeated_diagnostics_keep_original_positions`.

## Task 1: Reuse exact processing and diagnostic contexts

Files:

- Create `src/pipeline.rs` for invocation-local immutable snapshots.
- Modify `src/lib.rs` to consume shared snapshots at unchanged phases, not reconstruct Sources/config/trees.
- Modify `src/source.rs` and `src/parse/diagnostic.rs` for per-Source diagnostic recovery reuse.
- Modify only diagnostic consumers whose existing ownership requires explicit tree clones when receiving a shared result.
- Create `tests/processing_reuse.rs` for consumer-visible invalidation/position controls.
- Retain `tests/list_indentation_review.rs` and `tools/parser-oracle/reflow-scaling.mjs` unchanged.
- Update compatibility docs, roadmap and this plan with measured counts/budgets and review decisions.

Interfaces:

- `pipeline::Context<'config>` owns `&'config ResolvedConfig` and at most one `Snapshot`.
- `Snapshot` owns `Source`, `ParsedDocument` and `DocumentConfig`; these are immutable while borrowed by a phase.
- Snapshot also memoizes its immutable normalization-authorization batch. Module work is skipped only when all of that module's rules are disabled everywhere in the resolved document; parsing/config validation and fallible enabled consumers remain mandatory.
- `Context::snapshot(&mut self, text: &str) -> Result<&Snapshot, ProcessingError>` compares complete source bytes. Matching bytes reuse the current snapshot; changed bytes construct configured Source, native parse and directives before replacement. Failed construction must not expose partial state.
- `parse::diagnostic_parse(&Source) -> Option<Arc<ParsedDocument>>` computes the existing recovered/mapped tree once and shares it read-only. Cache None as well as success. A caller that mutates/consumes a tree must clone explicitly.
- Source's diagnostic OnceLock starts empty in new/derive/mapped Sources. Internal metadata setters invalidate any existing recovered result; no cache can survive a context mutation.
- Existing pure public APIs and FormatOptions/ResolvedConfig fields stay unchanged. No process-I/O, dependency or regex-grammar changes belong to this task.

- [x] Reproduce the unchanged list debug gate with `mise exec -- cargo test --test list_indentation_review admitted_intermediate_targets_keep_linear_memory_in_both_fix_modes`; capture its actual failure or pass. Expected from the prior baseline: deadline failure, not a compile error. Keep the log even if a later isolated run differs.
- [x] Run `node tools/parser-oracle/reflow-scaling.mjs` against the current debug binary; preserve all timing/output assertions. Expected from the prior baseline: 500-paragraph deadline failure.
- [x] Measure real native parse, diagnostic recovery and directive construction counts/times on those exact inputs using temporary local instrumentation. Record observations and remove throwaway instrumentation before implementation. If repeated work is not material, ledger a narrower measured optimization instead of implementing a speculative cache.
- [x] Write `context_reuse_keeps_equal_length_changes_independent`: alternate same-length inputs whose list/heading/HTML/inline directives differ, use distinct configurations, check literal outputs/findings and second-call no edits. Include disabled/enabled MD026 and custom/no-match metadata to expose stale rule/edit authority.
- [x] Write `diagnostic_reuse_keeps_metadata_views_independent`: alternate configured/default/nonmatching metadata with BOM/CRLF/astral/tab/HTML/list recovery, assert opaque prefix bytes, visible findings and real physical positions. Run interleaved threads to retain Source's thread-safety contract without sharing mutable permissions.
- [x] Write `repeated_diagnostics_keep_original_positions`: one input activates multiple consumers of recovered headings/references/emphasis/HTML/list trees; repeated calls and reordered independent configs must return the same literal diagnostic codes/spans and preserve source bytes.
- [x] Run new semantic controls before implementation. Expected: semantic controls pass against the baseline; the existing resource tests are the RED regressions proving the optimization is needed. Do not invent a missing API failure as performance evidence.
- [x] Implement Context/Snapshot and per-Source immutable diagnostic reuse, updating borrow/clone sites narrowly. Keep owned snapshots out of the convergence histories and separate from mapped/synthetic views.
- [x] Run focused semantic/metadata/pattern/directive/list/heading/reflow tests. Expected: existing outputs/findings/errors and original-source edit reconstruction remain unchanged.
- [x] Run the unchanged list debug gate and both `node tools/parser-oracle/reflow-scaling.mjs` / `node tools/parser-oracle/reflow-scaling.mjs --release`. Expected: all original input sizes pass within their original time/memory budgets, with exact expected output. If not, profile the remaining cost and continue this task; do not relax tests.
- [x] Run `mise exec -- cargo test --all-targets --jobs 2` and `mise exec -- cargo test --release --all-targets --jobs 2`, fmt/clippy/diff, 280-case metadata/82-case rule-pattern oracles and affected configuration/content/fragment/reflow/literal gates. Expected: complete debug and release receipts with no omitted failing targets.
- [x] Commit `perf: reuse immutable processing contexts`; record Task 1 completion with focused resource/consumer evidence.
- [x] Request one independent review of this batch with all Review Focus entries and ledger rulings. Fix Important/Critical findings once regression-first, rerun complete verification and retain decisions/costs before closing scratch. Continue Windows transport/corpus/artifact work without merge/push.

## Planning check and builder decisions

One task owns both reuse levels because semantic invalidation and the unchanged
resource budgets jointly define the deliverable. No public cache API or test-only
production cleanup methods are introduced. Existing resource failures are the RED
evidence; newly added semantic controls deliberately characterize the old results
instead of asserting private cache layout or exact helper-call counts.

Builder decisions: use one owned current snapshot and per-Source OnceLock/Arc.
Cost if wrong: conservative cache misses, explicit tree clones and lifetime
plumbing. Never trade that cost for stale metadata/configuration, unbounded retained
intermediates, relaxed budgets or shared mutable parser state. The profile can
justify a narrower optimization by a recorded ruling, not a silent plan rewrite.

The user already selected autonomous inline execution without routine approval
questions. This plan keeps that method and bounds the next deliverable; it does
not authorize publishing, moving configuration or claiming unrun platform evidence.

## Pre-review evidence

Baseline resource failures were reproduced unchanged before implementation. Initial
reuse passed 65 focused tests but still exceeded the prose budget; the second
profile identified disabled-module work and justified effective-rule gates.
All temporary instrumentation is removed. The same 500-paragraph invocation now
uses six parses/three directive contexts/two diagnostic recoveries instead of
169/42/10. Snapshot authorization reuse and the module/rule gate mapping are
explicit builder rulings: costs are one edit vector per current snapshot and a
mapping that must stay complete as new consumers are added.

The unchanged list resource test passes in 7.2 seconds, debug scaling at
100/500/1,000 paragraphs in 332/1,930/4,428 ms and optimized scaling in 37/206/495 ms.
Complete debug and optimized verification each pass 577 tests across 78 targets.
The 280 metadata/82 rule-pattern/56 property/110 config/554 content/1,144 fragment/
29 reflow/1,662 literal-style native comparisons pass. Fmt/clippy/diff are clean.
Commit/task-done then one independent review of this processing batch only;
the closed regex correction pass at 24b8500 does not receive a second review.

## Independent review closure

Committed at `283af75`. The single independent review found no Important or
Critical issues, so no correction pass or re-review is needed. It independently
passed 69 focused optimized tests, the original debug list gate in 6.84 seconds,
debug prose scaling in 306/1,800/4,184 ms and 15 ownership/inline-enable probes
(30 byte-preserving CLI checks). It inspected every changed production/test file,
exact invalidation, fixed config, authorization permission gates, immutable tree
consumers, metadata setters/mapped views and recovery initialization. No same-cell
reentrancy, retained cache cycle or unbounded parse history was found.

Retained reviewer-declined rulings and costs:

1. Closed regex fixes remain closed; only unchanged guard/fallibility interactions
   were inspected. Cost if wrong: any future independent regex defect needs its
   own evidence, not a disguised second review here.
2. Optional formatter execution remains Unix-only until the separate transport
   deliverable. Cost if wrong: other platforms still preserve code with warnings.
3. Windows runtime, remote CI, corpus/fuzzing and packaged artifacts remain open.
   Cost if wrong: these Linux results cannot establish roadmap completion.
4. Existing development Node advisories and reference pins remain unchanged.
   Cost if wrong: development-tool risk persists; this is not a native runtime
   dependency or permission to upgrade compatibility references.
5. Unrelated pre-existing parser/rule defects were not broadly audited.
   Cost if wrong: finite interaction coverage does not establish universal parity.
6. Throughput/memory/platform claims remain limited to measured gates and inspected
   ownership. Cost if wrong: other inputs may need additional measured hardening.
7. Minor checklist/style polish stays out of scope; none was reported as a
   consumer-impacting defect. Cost if wrong: cosmetic work remains, not hidden
   Important findings.

The two implementation rulings above retain their costs: one authorization vector
per current snapshot, and centralized ownership lists that must remain complete.
The full 577-test debug/release receipts and temporary profile counts are
implementer evidence. The reviewer did not claim to rebuild both complete suites
or concurrently initialize one private Source cache through the public thread test.
No deferred Minor findings. Keep branch/worktree; no merge, push or publication.
