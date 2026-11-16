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
- `Context::snapshot(&mut self, text: &str) -> Result<&Snapshot, ProcessingError>` compares complete source bytes. Matching bytes reuse the current snapshot; changed bytes construct configured Source, native parse and directives before replacement. Failed construction must not expose partial state.
- `parse::diagnostic_parse(&Source) -> Option<Arc<ParsedDocument>>` computes the existing recovered/mapped tree once and shares it read-only. Cache None as well as success. A caller that mutates/consumes a tree must clone explicitly.
- Source's diagnostic OnceLock starts empty in new/derive/mapped Sources. Internal metadata setters invalidate any existing recovered result; no cache can survive a context mutation.
- Existing pure public APIs and FormatOptions/ResolvedConfig fields stay unchanged. No process-I/O, dependency or regex-grammar changes belong to this task.

- [ ] Reproduce the unchanged list debug gate with `mise exec -- cargo test --test list_indentation_review admitted_intermediate_targets_keep_linear_memory_in_both_fix_modes`; capture its actual failure or pass. Expected from the prior baseline: deadline failure, not a compile error. Keep the log even if a later isolated run differs.
- [ ] Run `node tools/parser-oracle/reflow-scaling.mjs` against the current debug binary; preserve all timing/output assertions. Expected from the prior baseline: 500-paragraph deadline failure.
- [ ] Measure real native parse, diagnostic recovery and directive construction counts/times on those exact inputs using temporary local instrumentation. Record observations and remove throwaway instrumentation before implementation. If repeated work is not material, ledger a narrower measured optimization instead of implementing a speculative cache.
- [ ] Write `context_reuse_keeps_equal_length_changes_independent`: alternate same-length inputs whose list/heading/HTML/inline directives differ, use distinct configurations, check literal outputs/findings and second-call no edits. Include disabled/enabled MD026 and custom/no-match metadata to expose stale rule/edit authority.
- [ ] Write `diagnostic_reuse_keeps_metadata_views_independent`: alternate configured/default/nonmatching metadata with BOM/CRLF/astral/tab/HTML/list recovery, assert opaque prefix bytes, visible findings and real physical positions. Run interleaved threads to retain Source's thread-safety contract without sharing mutable permissions.
- [ ] Write `repeated_diagnostics_keep_original_positions`: one input activates multiple consumers of recovered headings/references/emphasis/HTML/list trees; repeated calls and reordered independent configs must return the same literal diagnostic codes/spans and preserve source bytes.
- [ ] Run new semantic controls before implementation. Expected: semantic controls pass against the baseline; the existing resource tests are the RED regressions proving the optimization is needed. Do not invent a missing API failure as performance evidence.
- [ ] Implement Context/Snapshot and per-Source immutable diagnostic reuse, updating borrow/clone sites narrowly. Keep owned snapshots out of the convergence histories and separate from mapped/synthetic views.
- [ ] Run focused semantic/metadata/pattern/directive/list/heading/reflow tests. Expected: existing outputs/findings/errors and original-source edit reconstruction remain unchanged.
- [ ] Run the unchanged list debug gate and both `node tools/parser-oracle/reflow-scaling.mjs` / `node tools/parser-oracle/reflow-scaling.mjs --release`. Expected: all original input sizes pass within their original time/memory budgets, with exact expected output. If not, profile the remaining cost and continue this task; do not relax tests.
- [ ] Run `mise exec -- cargo test --all-targets --jobs 2` and `mise exec -- cargo test --release --all-targets --jobs 2`, fmt/clippy/diff, 280-case metadata/82-case rule-pattern oracles and affected configuration/content/fragment/reflow/literal gates. Expected: complete debug and release receipts with no omitted failing targets.
- [ ] Commit `perf: reuse immutable processing contexts`; record Task 1 completion with focused resource/consumer evidence.
- [ ] Request one independent review of this batch with all Review Focus entries and ledger rulings. Fix Important/Critical findings once regression-first, rerun complete verification and retain decisions/costs before closing scratch. Continue Windows transport/corpus/artifact work without merge/push.

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
