# Fenced Code Formatters Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add explicit optional fenced-code formatting with bounded process execution and byte-preserving fallback.

**Architecture:** A fixed adapter registry and bounded process runner produce stable logical code. A private source-owned body stage validates candidate Markdown and a separate process-I/O API/CLI path composes it with ordinary formatting.

**Tech Stack:** Existing Rust toolchain and tempfile; pinned synchronous command-group 5.0.1 for process groups, standard threads/channels for bounded pipe I/O. Installed formatters remain optional.

**Spec:** `docs/superpowers/specs/2026-11-12-code-formatters-design.md`, binding roadmap and formatter acceptance scenarios in `docs/compatibility/acceptance.json`.

## Global Constraints

- Native Rust, no mandatory Node runtime, no shell commands/package fetching from application code, no JavaScript configuration execution.
- Keep existing pure formatting APIs and `FormatOptions { fix }`/ResolvedConfig call sites compatible; add a separate process-I/O entry point and opt-in `--format-code`.
- Respect both fix controls and MD013 code-block/inline permissions before spawning. Preserve failed bodies exactly and retain final ordinary diagnostics.
- Only original native closed fences with unambiguous prefixes qualify; never promote recovered ownership into edit authority or relax generic protected-byte checks.
- Private output validation may ignore only selected Code values; all other AST fields and bytes outside selected spans remain exact. Require formatter and public-call idempotence.
- Use 1MiB input/stdout,64KiB stderr,5s/invocation,30s/document,64bodies/document and width1..=4096; kill/reap process groups and drain/join bounded I/O workers on all outcomes.
- Inline regression-first batches, one independent review/Important-Critical fix pass; no merge/push or routine approval questions. Close the prior convergence review before implementation.

## Review Focus

- Pipe pressure, leader exit with live descendants and cleanup failures must not hang formatting or retain partial code; Task 1 owns real-process deadline/cleanup tests.
- Container prefixes, tabs, empty lines and CR/BOM boundaries must not leak into logical input or let formatter output escape its fence; Task 2 owns original-source/AST/unsafe-output controls.
- Disabled code/rules/lines and non-fix modes must not spawn processes, while failed independent bodies preserve their bytes and final MD013 findings; Task 2 owns invocation counters and mixed outcomes.
- Adapter isolation/argv and unavailable tools must not load project plugins/configuration, run shell syntax or falsely claim width control; Task 1 owns exact argv and actual-tool smoke tests.
- Repeated formatter and Markdown stages must settle with exact original-coordinate edit reconstruction and final anchored warnings, including adjacent changed/failed fences; Task 2 owns composition/idempotence tests.

## Task 1: Bounded formatter runner and fixed adapters

Files: create `src/code_formatters/mod.rs`, `src/code_formatters/runner.rs`, `src/code_formatters/adapters.rs`; add module declaration and tracked Cargo dependency/lock update. Test fixture executables live in `tests/support/` only if shared by integration tests.

Interfaces:

- `Adapter::for_language(language: &str) -> Option<Adapter>` selects rust/rs→rustfmt, go/golang→gofmt, python/py→black, json→prettier JSON only.
- `Adapter::command(width: usize, isolated: &Path) -> Result<Command, FormatterFailure>` produces fixed argv and isolated cwd/config; rustfmt edition2024/emitstdout, gofmt no width arg, Black isolated line-length, Prettier noconfig/noeditorconfig/parserjson/printwidth.
- `RunLimits` privately holds timeout/stdout/stderr bounds; `run(command: Command, input: &[u8], limits: &RunLimits) -> Result<Vec<u8>, FormatterFailure>` is the real bounded process path.
- `FormatterFailure` supplies diagnostic code and concise message for unavailable/failed/timeout/output-limit/unstable/unsafe/skipped outcomes.
- `format_stable(adapter: Adapter, input: &str, width: usize, deadline: Instant) -> Result<String, FormatterFailure>` runs twice within remaining document budget, validates UTF-8/nonempty/NUL and compares normalized LF output.

- [x] Write `runner_preserves_partial_failure_and_bounds_both_pipes`, `runner_deadline_covers_blocked_stdin_and_descendant_held_output`, and `stable_format_rejects_invalid_empty_and_alternating_results` using real controlled subprocesses. Assert exact bytes/status/failure codes and completion within lowered private deadlines; no mock process claims.
- [x] Write `adapter_commands_are_isolated_and_width_specific` with exact program/argv/cwd/config assertions for all aliases. Unknown/extra-info languages select no adapter. Gofmt has no invented width option. Controlled PATH fixtures must not mutate the parent process's environment in parallel tests.
- [x] Run `mise exec -- cargo test --release --lib code_formatters`. Expected: missing adapter/runner APIs before implementation.
- [x] Implement process-group spawn, concurrent bounded stream readers/writer, common monotonic deadline, cleanup on every exit, fixed commands and two-run stability validation. Do not call unbounded wait_with_output or join live pipes before group cleanup.
- [x] Run focused tests, available real rustfmt/gofmt smoke tests, complete optimized suite, fmt/clippy/diff. Expected: exact outputs, no hangs/leaked groups, prior pure Markdown behavior unchanged; Black/Prettier smoke coverage marked unavailable if absent rather than faked.
- [ ] Commit `feat: add bounded optional code formatter adapters`; task-done with the focused module and complete suite evidence.

## Task 2: Source-owned fenced bodies and opt-in API/CLI

Files: create `src/code_formatters/fences.rs`, `tests/code_formatters.rs`, `docs/compatibility/code-formatters.md`; modify `src/code_formatters/mod.rs`, `src/lib.rs`, `src/cli.rs`, README and roadmap. Keep generic edit/parse preservation unchanged.

Interfaces:

- Consume Task1 `Adapter`, `format_stable` and `FormatterFailure` as specified; no second process runner or test-only production shortcut.
- `CodeFormatOptions` is a separate public defaultable type with `enabled: bool`; existing APIs never invoke external processes.
- `format_with_code_formatters(source: &str, options: &FormatOptions, config: &ResolvedConfig, code: &CodeFormatOptions) -> Result<FormatResult, ProcessingError>` exposes explicit process I/O; disabled/fixfalse/configfixfalse returns the ordinary path without discovery/spawn.
- Private `FenceBody` captures original Code ordinal/body span/language/restored prefix/available width; `bodies(&Source,&ParsedDocument,&DocumentConfig) -> Vec<FenceBody>` rejects uncertain ownership, unsupported info and incomplete fences before process invocation.
- Private stage returns body edits plus failures anchored by owning Code ordinal. Reidentify those ordinals after final Markdown settlement before rendering warnings; do not carry stale physical offsets through changing prose.

- [ ] Write `successful_formatter_restores_quote_and_list_prefixes`, including literal quoted JSON acceptance output, root/list/nested fences, blank lines and independent code widths. Assert exact source outside bodies and exact original-coordinate edit reconstruction.
- [ ] Write `disabled_or_unowned_fences_never_spawn` using controlled real fixture invocation counts: both fix controls, MD013 disabled/code_blocksfalse, partial inline permissions, tables/HTML recovered-only fences, unclosed/unlabeled/indented/unknown/extra-info cases and uncertain tabs.
- [ ] Write `failures_preserve_original_bodies_and_final_findings` for unavailable/nonzero partial/timeout/outputlimit/invalidUTF8/unstable/fencebreaking output; include one successful adjacent body, MD013 original/final findings, BOM/CRLF/CR and literal markdownlint comments in code.
- [ ] Write CLI check/write/stdout/alias/severity tests with --format-code on/off, repeated calls, changed/unchanged statuses, warning anchors after wrapping prose and both fix controls. Require a second formatting call to export no edits.
- [ ] Run `mise exec -- cargo test --release --test code_formatters`. Expected: missing opt-in entry point/behavior, not incorrect fixtures or unsupported existing settings.
- [ ] Implement source-backed closed-fence extraction/reconstruction, exact permissions and width subtraction; run bounded stable adapters and privately validate only selected Code values before composing any replacements. Preserve unsuccessful independent bodies and warnings.
- [ ] Wire the explicit API and CLI option: ordinary Markdown settlement, code stage once, final Markdown settlement/diagnostics, original-edit composition and final Code-ordinal warning anchors. Add no formatter settings to markdownlint config discovery.
- [ ] Run focused and full optimized tests plus prior heading/list/fragment/reference/reflow/code native gates, fmt/clippy/diff. Expected: same preservation limits, exact final warnings/findings, real local tool evidence distinguished from controlled fixtures and unavailable tools.
- [ ] Update evidence/README/help/roadmap honestly and commit `feat: format owned fenced code through opt-in adapters`.
- [ ] Request one independent batch review with every Review Focus/ruling. Reproduce Important/Critical findings, fix once, run complete verification; retain every ruling/cost before removing only completed scratch.
- [ ] Continue later regex/package declarative compatibility and release hardening without merge/push or routine questions.

## Planning check

The existing pure API and generic protected-code policy remain untouched unless
the new opt-in entry point is called. Task1's failure/stability/process interfaces
are exactly Task2's consumers. Every review input class has owning tests. Local
discovery found rustfmt and gofmt, not Black/Prettier. Isolated temporary-directory
probes actually ran rustfmt edition2024/emitstdout/explicit40-column config and
gofmt against complete tiny programs; both exit0, produce expected formatted
output and settle on a second invocation. This is direct-tool smoke evidence,
not a native adapter implementation or process-limit test. command-group
documentation confirms process-group kill/wait on Unix
and Windows, but warns about sequential wait_with_output pipes, which this plan
does not use. Runtime platform guarantees remain hardening work.

## Builder decisions and assumptions

1. Make execution opt-in and preserve pure APIs. Rationale: installing mdfmt must
   not silently grant new process-execution behavior. Cost if wrong: users need
   `--format-code` rather than getting available-tool formatting automatically.
2. Start with four fixed language/tool adapters and no project plugin discovery.
   Rationale: explicit commands are auditable and do not introduce a runtime
   package manager. Cost if wrong: other languages/project formatter preferences
   need later explicit adapters instead of arbitrary shell configuration.
3. Tie code formatting to enabled MD013/code_blocks and all-body permissions.
   Rationale: reuse the configured code-width authority without a second rule
   system. Cost if wrong: users wanting style-only code edits with MD013 disabled
   need a later separately authorized policy.
4. Verify tool idempotence and targeted Markdown ownership, not language semantic
   equivalence. Rationale: trusted external formatters own syntax semantics.
   Cost if wrong: a stable but buggy installed tool can still change code meaning;
   successful output is not a proof of equivalence.
5. Use process groups and bounded concurrent I/O, not a sandbox. Rationale:
   deadlines must cover ordinary descendants and full pipes on supported systems.
   Cost if wrong: deliberately escaped descendants/resource abuse remain outside
   the trust boundary; untested platform behavior is a hardening obligation.

## Execution evidence and rulings

Task1's tests fail on missing runner/adapter/stability APIs before implementation.
Five focused tests now pass with real controlled subprocesses, bounded concurrent
pipes, blocked stdin and descendant-held output cleanup, invalid/partial/empty/
NUL/unstable output rejection, exact isolated argv and real rustfmt/gofmt smoke
coverage. Black/Prettier are absent locally and have argv coverage, not actual
formatter runtime evidence. Complete optimized verification passes544/544 across
73 targets, fmt/clippy clean. Production code-body/API/CLI integration remains
Task2; this is infrastructure, not implemented fenced-body formatting.

6. Compile the private module under cfg(test) until Task2 supplies its production
   consumer. Rationale: avoid dead-code suppression, public internal APIs or a
   placeholder runtime path. Cost if wrong: infrastructure is not included in a
   standalone Task1 release; tests still execute its real process implementation.
