# Corpus and Native Artifact Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add offline corpus/property/benchmark evidence and reproducible local packages tested through the extracted native executable.

**Architecture:** Checked-in, licensed inputs and pinned upstream protected-range receipts feed Rust consumer tests. Native seeded subprocess cases and a benchmark example exercise the pure library. A development-only Python packager builds and validates curated archives; bounded CI runs these gates without publishing.

**Tech Stack:** Declared Rust/mise toolchain, existing serde_json/markdown dependencies, Python standard library, existing pinned development-only markdownlint/micromark.

**Spec:** `docs/superpowers/specs/2026-11-16-release-hardening-design.md`, corpus/artifact deliverable. Processing review is closed; Windows transport/runtime remains separate.

## Global Constraints

- Preserve branch/worktree/shared configuration, unrelated node_modules link and all prior approved rule/edit/regex/package decisions. No merge/push/publication or `.env` reads.
- Ordinary tests are offline and invoke no Node runtime from the Rust formatter. Fixture refresh is an explicit development action with pinned URLs/hashes/licenses.
- Keep current 4,000-item/128 MiB/30-second and prose 100/500/1,000-input/10-second gates unchanged.
- Require repeat stability, valid original edit reconstruction and exact code/table/metadata/link preservation outside approved MD026/MD051/external-formatter edits.
- Seeded native properties use 256 cases and a 30-second subprocess budget. Oracle child invocations get ten-second limits; CI jobs get explicit overall limits.
- Full debug/release suites, fmt/clippy/diff and relevant native gates precede completion claims. Record Important/Critical discoveries, no nitpicking or silent corpus skips.
- Execute inline; one independent batch review and one Important/Critical regression-first correction pass. No second reviews of closed batches.

## Review Focus

- Newlines, tabs, astral text and recovered containers cannot shift protected-range authority; owning `licensed_corpus_preserves_upstream_atomic_ranges`.
- Seeded hostile syntax must terminate, remain stable and retain typed configuration/work-limit failures without poisoning later calls; owning `seeded_native_cases_are_bounded_and_repeatable`.
- Generic preservation must not suppress approved MD026 punctuation or admitted MD051 case edits; owning `approved_repairs_remain_available_under_hardening`.
- Archives must not include arbitrary worktree files, machine project paths or missing notices/provenance; owning `test_release_archive_is_reproducible_and_curated`.
- Check/write/config failures and unavailable formatters must preserve inputs when running the extracted executable, not merely the build-tree binary; owning `test_packaged_binary_modes_are_transactional`.

## Task 1: Offline corpus, seeded properties and benchmark distributions

Files: create `tests/hardening_corpus.rs`, `tests/fixtures/corpus/` with manifest/raw data/licenses/observations, `tools/refresh-corpus.py`, `tools/parser-oracle/corpus.mjs`, `examples/benchmark.rs`, `.gitattributes`; update compatibility docs/roadmap. Product changes only for demonstrated Important/Critical regressions.

Interfaces:

- Corpus manifest records each source URL/version/SHA/license and relative local path. CommonMark JSON remains unmodified; real-document bytes retain pinned hashes. Raw fixture files use Git `-text` attributes.
- Pinned `corpus.mjs --record --source-only` writes case IDs, input and original UTF-8 atomic ranges for code/inline code/tables/links/images/definitions/HTML/hard breaks. Normal mode runs the native CLI twice, enforces subprocess limits, compares selected source-parser structure/atomic text and unchanged files. No hostile JS-regex execution loop.
- Rust tests consume checked-in observations using existing public formatting APIs, with MD013 width 12, no_inline_config true and explicit custom no-match metadata for the corpus. Seeded cases use fixed local PRNG state, never randomness/network/process formatters.
- `examples/benchmark.rs` emits JSON records for prose/list/reference/code/table samples, elapsed times and median/p95; no timing assertions beyond existing resource gates.

- [x] Write `licensed_corpus_preserves_upstream_atomic_ranges`: require 652 standard cases plus two real documents, no-fix exact bytes, MD013-only prefix/range preservation, composed edit reconstruction, second-call no edits and at least one real corpus transformation. Run RED on missing corpus observations, not a fabricated formatter bug.
- [x] Write `seeded_native_cases_are_bounded_and_repeatable` and `approved_repairs_remain_available_under_hardening`. Pin 256 seeded cases and literal authorized output `# Target\n\n[case](#target)\n` from `# Target!\n\n[case](#TARGET)\n`. A parent test bounds the native property child; hostile-pattern controls require typed failures and later safe recovery.
- [x] Implement explicit fixture refresh with the pinned CommonMark digest and README hashes from source audit, retaining source licenses. Record actual micromark atomic ranges before running Rust/native comparisons. Expected: complete corpus results or concrete failures with case IDs, never skipped failing cases.
- [x] Reproduce any Important/Critical result in a focused RED consumer regression, fix narrowly and record the roadmap finding. Preserve reference pins and existing fix authority. No formatter defect was established; source-probe corrections have separate RED controls.
- [x] Implement/run the five-sample native benchmark in debug and release. Expected: five categories with actual finite sample distributions, not invented performance numbers.
- [x] Run `mise exec -- cargo test --test hardening_corpus`, `mise exec -- cargo test --release --test hardening_corpus`, `node tools/parser-oracle/corpus.mjs` and affected metadata/reflow/literal gates. Expected: exact protected ranges, output reconstruction/idempotence and bounded native properties pass.
- [x] Run complete debug/release suites, fmt/clippy/diff; commit `test: add licensed corpus and bounded native properties`, task-done with actual consumer evidence.

## Task 2: Curated reproducible archives and extracted-binary checks

Files: create `tools/release.py`, `tools/test_release.py`, root `LICENSE`, release compatibility/usage docs; modify `.github/workflows/ci.yml` and `.gitignore` only for local output. Retain current file-write implementation unless a real regression is demonstrated.

Consumes: Task 1 offline corpus/property gate and declared Rust build baseline, not its private test helpers.

Interfaces:

- `python3 tools/release.py --output DIR --build-dir DIR` builds the host CLI through `mise exec -- cargo build --release --locked`, using explicit path remapping and no script installs. Use bounded subprocesses and fixed argv, never shell execution. Record version, host, toolchain, source identity and normalized flags without machine paths.
- Emit `mdfmt-VERSION-HOST.zip` with fixed/sorted member metadata and `.sha256` sidecar. Curate executable, README, project license, Cargo.lock, BUILD-INFO.json and combined dependency/data license notices. Resolve notices from locked Cargo metadata and known generated-data licenses; never scan arbitrary worktree contents or read `.env` files.
- Smoke-test an extracted executable in a private directory: stdin/stdout, check, write/idempotence, declarative/package/custom-metadata config, invalid config/no write and absent formatter/original-code retention. Unix checks retain symlink/hardlink rejection and ordinary modes. Do not claim race-proof writes or unrun Windows semantics.

- [ ] Write Python consumer tests `test_release_archive_is_reproducible_and_curated` and `test_packaged_binary_modes_are_transactional`; run RED because the release command is absent. Assert exact allowed archive members, actual sidecar hashes, repeat archive bytes and extracted-binary results, not script source text.
- [ ] Implement the packager and notices/provenance collection, honoring Cargo's existing MIT declaration with a root MIT license and explicit dependency/data license retention. Build only locally; reject unsafe/unrepresentable packaging state explicitly.
- [ ] Run `python3 -m unittest discover -s tools -p test_release.py` against actual native builds/archives. Expected: both archive integrity/reproducibility and extracted CLI transactional checks pass with exact code/metadata bytes.
- [ ] Add job timeouts, offline corpus/property gates, bounded oracle comparison and host-native package smoke checks to CI. Do not add release/tag/upload actions; document remote receipts as unavailable until executed.
- [ ] Produce and smoke-test a local Linux archive/checksum under `/tmp/opencode`, run full debug/release and affected configuration/formatter gates, fmt/clippy/diff and Python tests. Expected: real extracted native binary works, curated artifacts have no forbidden members/project paths, no application runtime dependency was introduced.
- [ ] Commit `build: package and verify native release artifacts`; task-done. Request one independent review of this two-task batch, apply one regression-first Important/Critical pass if needed, retain all rulings/costs and continue the remaining Windows implementation/runtime obligation without merge/push.

## Planning check and decisions

Task 2 consumes Task 1 only as a CI gate; it does not depend on corpus internals.
All five Review Focus risks have consuming tests. Explicit refresh permits a
maintainer download, not a formatter downloader. Python is development tooling,
not a Node/Python wrapper around the Rust CLI. Local reproducible ZIP metadata is
not proof of reproducible binaries across hosts; compare real build/archive
receipts and state the narrower evidence. Corpus licensing stays in test data,
not in binary contents; project/dependency/generated-data licenses remain curated.

The builder owns these reversible choices under the user's autonomous inline
instruction. Costs: larger licensed fixtures, finite seeded coverage rather than
coverage-guided proof, host-specific benchmark distributions, explicit notice
maintenance and still-missing Windows/remote runtime receipts. None authorizes
publication, reference-pin upgrades or declaring the whole roadmap complete.

## Task 1 evidence and retained decisions

The missing corpus observations and benchmark APIs failed before implementation.
All 654 corpus cases now pass native protected-range/edit/idempotence checks and
bounded source/native comparison. Probe regressions separately exposed lost entity
payload/word spacing, nonsemantic micromark content grouping and artificial
undefined-reference overlays; source/runtime checks justified representation
corrections rather than application edits or skipped cases. Compare selected
semantic blocks/inline events, not internal chunks/diagnostic copies. Cost if
wrong: this remains a finite selected-structure proof; literal source ranges and
probe invariants are retained, not replaced by a universal HTML-parity claim.

All 256 seeded native cases pass; 29 perform real transformations. Approved MD026/
MD051 controls pass. Debug and release benchmark distributions retain five actual
samples per prose/list/reference/code/table category. Both complete suites pass
581 tests across 80 targets; 280 metadata/82 patterns/29 reflow/1,662 literal-style/
220 fence-style native gates and fmt/clippy/diff pass. Task 2 and the independent
combined review remain; no new formatter production correction was required.

Retain the ordering ruling: run available Linux corpus/artifact work before Windows
transport while no native Windows runtime is available. Cost if wrong: the Windows
implementation/runtime obligation remains open, never waived by these results.
