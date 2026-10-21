# Rust Formatting Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build milestone 2's source-preserving Rust library and CLI without shipping production rule fixes.

**Architecture:** Keep original UTF-8 source authoritative, derive source regions through a private parser adapter, and validate complete localized-edit batches before applying them. Keep filesystem and terminal behavior outside the library. Production returns unchanged source until later milestones add configuration-aware rules.

**Tech Stack:** Rust 1.99, Cargo, one library/binary package, a fixture-selected Markdown parser, clap for CLI parsing, tempfile for safe replacement, and Node only as a development oracle.

**Spec:** `docs/superpowers/specs/2026-10-21-formatting-foundation-design.md`

## Global Constraints

- Use Rust 1.99 from `mise.toml`, one Cargo package named `mdfmt`, a library, a thin binary, and tracked `Cargo.lock`.
- No Node runtime dependency is permitted.
- No production rule fixes ship in this milestone.
- Original UTF-8 source is authoritative.
- Byte ranges are zero-based and end-exclusive.
- Human positions are one-based.
- All diagnostics/status go to stderr; stdout contains only document content.
- Never read `.env`; preserve unrelated user work; do not commit without authorization.
- Tests establish foundation behavior, not full markdownlint compatibility.

## Review Focus

- Non-BMP Unicode, BOM, tabs, and mixed line endings must not corrupt offsets or source; Tasks 1–3 pin these.
- Insertions at protected-region edges must not bypass protection or produce order-dependent edits; Task 3 pins these.
- Duplicate paths, symlinks, and hardlinks must not produce repeated or surprising writes; Task 5 pins these.
- Failed replacement and observed target changes must preserve original content; Task 5 pins these.
- No-op CLI success must not be presented as lint cleanliness; Tasks 4 and 6 pin streams, coverage, and documentation.

## File map

- `Cargo.toml`, `Cargo.lock`: package, production/dev dependencies.
- `src/lib.rs`: small public processing API and coverage types.
- `src/source.rs`: source preservation, line index, position conversions.
- `src/parse.rs`: private selected-parser adapter and region model.
- `src/edit.rs`: edit validation, protected-span policy, localized application.
- `src/diagnostic.rs`: structured findings and processing failures.
- `src/main.rs`, `src/cli.rs`: argument parsing, streams, exit aggregation.
- `src/file_io.rs`: literal input handling, deduplication, safe writes.
- `tests/source.rs`, `tests/preservation.rs`, `tests/cli.rs`: public integration coverage.
- `tests/fixtures/parser/`: inputs and expected classifications/source spans.
- `tests/parser_candidates.rs`: temporary dev-only candidate comparison harness.
- `tools/parser-oracle/`: pinned Node dev oracle and lockfile, never runtime.
- `docs/compatibility/parser-evaluation.md`: evidence and selected-parser decision.
- `README.md`: actual foundation status and usage, not future behavior claims.

The region/edit internals stay private; colocate their unit tests. Do not expose
parser-crate types. No crate-per-module workspace or plugin framework.

## Task 1: Original source and coordinate model

Files: create `Cargo.toml`, `Cargo.lock`, `src/lib.rs`, `src/source.rs`,
`tests/source.rs`; update `.gitignore` only for Cargo build artifacts if needed.

Interfaces:

- Public `Source::new(text: String) -> Source`, `Source::text(&self) -> &str`.
- Public `Position { line: usize, column_utf16: usize }`.
- Public `Source::position(&self, byte: usize) -> Result<Position, PositionError>`.
- Public `utf16_len(text: &str) -> usize` for source width accounting.
- Store terminators and BOM in the original string, never normalize them.

- [ ] Add minimum Cargo metadata (edition 2024, rust-version 1.99) and tests before implementation. `positions_use_utf16` asserts `Source::new("a😀b\r\nz".into()).position(5) == Ok(Position { line: 1, column_utf16: 4 })` and position 8 is line 2, column 1. `invalid_byte_boundary` rejects offset 2 in the emoji.
- [ ] Run `mise exec -- cargo test --test source`; confirm missing API fails compilation.
- [ ] Implement the source API. Add tests for empty input/EOF, BOM, LF/CRLF/lone CR, mixed endings, combining marks, and tabs; `utf16_len("a😀\t") == 4`. Reject positions inside CRLF rather than assign two spellings to one line boundary.
- [ ] Run `mise exec -- cargo test --test source`; require all tests pass and exact original text survives construction.
- [ ] Review the diff and record Task 1 complete. Commit only if the user has requested commits.

## Task 2: Parser selection with executable evidence

Files: create `tests/parser_candidates.rs`, `tests/fixtures/parser/`,
`tools/parser-oracle/package.json`, its lockfile and `oracle.mjs`,
`src/parse.rs`, `docs/compatibility/parser-evaluation.md`; modify Cargo files.

Interfaces:

- Private `parse(source: &Source) -> Result<ParsedDocument, ProcessingError>`.
- `ParsedDocument` stores source ranges and container relationships, not rendered text.
- `Region { span: Range<usize>, kind: RegionKind }`; kinds distinguish eligible
  prose, containers, protected blocks, and atomic inline spans from the design.
- Test-only normalized candidate summaries include kind, range, parent, and raw slice.

- [ ] Add fixture assertions before adapters: table spans slice exact original rows; links include original delimiters/destination/title; code and HTML are protected; nested prose retains parent relationships; reference definitions are represented even if a candidate omits them from normal traversal. Cover every comparison category listed in the design, including multiline links and lazy continuations.
- [ ] Run `mise exec -- cargo test --test parser_candidates`; confirm absent adapters fail.
- [ ] Add exact dev dependencies for `markdown =1.0.0`, `pulldown-cmark =0.13.4`, and `comrak =0.57.0`, with unnecessary default features disabled. Implement comparison adapters. Add a locked dev-only oracle using markdownlint 0.42.0 / cli2 0.24.0 and their pinned micromark dependency; use upstream parser/front-matter behavior, not regex guesses. Its install/run instructions belong in `parser-evaluation.md`.
- [ ] Execute the comparison and oracle. Record versions, commands, classifications, source slices, gaps, and mitigations in `parser-evaluation.md`; keep source-reviewed findings separate from runtime evidence. If the safety gate fails for all candidates, stop here with evidence.
- [ ] Select the preferred candidate only if evidence supports it. Promote only that parser to production dependencies; remove unused candidate dev dependencies/harness adapters after retaining normalized comparison evidence and regression fixtures. Keep the oracle development-only. Implement `parse` and test default front matter against upstream independently of crate metadata flags.
- [ ] Run `mise exec -- cargo test`; require selected-adapter fixture tests and source tests pass. Review dependency scope and mark Task 2 complete.

## Task 3: Validated localized edits

Files: create `src/edit.rs`, `src/diagnostic.rs`; modify `src/lib.rs`, `src/parse.rs`.

Interfaces:

- Private `Edit { span: Range<usize>, replacement: String, origin: String }`.
- Private `apply_edits(source: &Source, parsed: &ParsedDocument, edits: Vec<Edit>) -> Result<String, ProcessingError>`.
- `ProcessingError` distinguishes invalid ranges, edit conflicts, protected-region
  violations, and parser/position failures; it implements `Display` and `Error`.
- `Diagnostic { code: String, severity: Severity, message: String, rule_id: Option<String>, span: Option<Range<usize>> }`; `Severity::{Warning, Error}`.

- [ ] Write unit tests: replacing `world` in `"Hello world!\r\n"` yields `"Hello Rust!\r\n"`; unedited bytes are identical. Invalid UTF-8 boundaries/out-of-bounds/reversed ranges, overlapping replacements, identical-offset insertions, and insertion/replacement boundary collisions return errors with no partial output. Adjacent non-empty replacements succeed.
- [ ] Run `mise exec -- cargo test edit::`; confirm missing implementation fails.
- [ ] Implement deterministic validation and slice-copy application. Validate all edits before output construction. Test insertion at beginning/end/interior of protected spans; conservatively reject both edges unless an explicitly eligible boundary proves the insertion cannot change that region's interpretation. Reject partial changes to atomic inline spans. No current producer is authorized to rewrite whole atomic spans.
- [ ] Add tests protecting tables, front matter, code, HTML/comments, reference definitions, links/images, and inline code, including nested containers and Unicode. Permit eligible prose edits outside protected regions.
- [ ] Run `mise exec -- cargo test`; require all passing. Review error messages and mark Task 3 complete.

## Task 4: Processing results, permission, and meaningful idempotence

Files: modify `src/lib.rs`, `src/diagnostic.rs`; create `tests/preservation.rs`.

Interfaces:

- Public `FormatOptions { fix: bool }`, default `fix: true`.
- Public `format(source: &str, options: &FormatOptions) -> Result<FormatResult, ProcessingError>`.
- Public `FormatResult { output: String, edits: Vec<AppliedEdit>, diagnostics: Vec<Diagnostic>, coverage: Coverage }` and `changed(&self, original: &str) -> bool`.
- Public `AppliedEdit { span: Range<usize>, replacement: String, origin: String }`.
- Public `Coverage { implemented_rules: Vec<String>, complete: bool }`; foundation returns empty rules, `complete: false`.
- Private processing helper accepts an edit producer; production producer is empty,
  unit-test producer replaces a single designated eligible prose token once.

- [ ] Write public preservation tests asserting exact output equality for every parser fixture, `edits.is_empty()`, and explicit incomplete coverage. Write unit tests where an actual eligible edit changes output on the first pass and not the second; `fix: false` preserves original text and yields no applied edits.
- [ ] Run `mise exec -- cargo test`; confirm absent processing API fails.
- [ ] Implement the shared processing path, option gate, and diagnostic/result types. Rule findings refer to returned output; operational failures retain original-source locations. Do not expose test producers or a hidden CLI test flag.
- [ ] Add synthetic warning/error finding tests proving severity survives and disabled edits do not disable findings. Production still implements no rule diagnostics.
- [ ] Run `mise exec -- cargo test`; require exact preservation and non-vacuous idempotence tests pass. Mark Task 4 complete.

## Task 5: Literal files and safe replacement

Files: create `src/file_io.rs` and a minimal `src/main.rs` test host; modify Cargo
files for `tempfile`. Task 6 replaces the empty entry point with CLI dispatch.

Interfaces (binary-private):

- `read_input(path: &Path) -> Result<FileSnapshot, FileError>` validates regular-file UTF-8.
- `FileSnapshot` owns original bytes/text, target identity/metadata, and path.
- `write_if_changed(snapshot: &FileSnapshot, output: &str) -> Result<bool, FileError>` returns whether replacement occurred.
- `unique_inputs(paths: &[PathBuf]) -> Result<Vec<PathBuf>, FileError>` preserves first target order; define deterministic per-path errors at the CLI boundary rather than dropping valid later inputs.

- [ ] Write unit tests before implementation: unchanged content leaves timestamp unchanged; successful changed output preserves ordinary permissions; symlink and multiply linked write targets are rejected on Unix; path aliases deduplicate by target identity. Invalid UTF-8, directories, and special files fail safely. Do not open a FIFO and block before checking file type.
- [ ] Run `mise exec -- cargo test --bin mdfmt file_io::`; confirm absent binary/module fails.
- [ ] Implement file snapshots, regular-file checks, deterministic identity handling, and same-directory temporary replacement. Add the minimal binary test host declaring `file_io` so its unit tests compile. Recheck observed identity/content before replacement. Never fall back to truncating writes. For platforms without link-count/identity APIs, document conservative supported guarantees instead of assuming Unix behavior.
- [ ] Introduce a private injectable replacement operation for unit tests. Assert simulated replacement failure leaves original bytes and cleans temporary files. Mutate the target after reading and assert stale-snapshot rejection; test unchanged paths do not bypass write-target safety checks.
- [ ] Run `mise exec -- cargo test`; require safe-write tests exercise real changes, not just the production no-op. Mark Task 5 complete.

## Task 6: CLI contract and honest handoff

Files: create `src/main.rs`, `src/cli.rs`, `tests/cli.rs`; modify
`src/file_io.rs`, Cargo files, and `README.md`.

Interfaces:

- `main` delegates to `cli::run() -> u8`; use `std::process::ExitCode`.
- Private `Mode::{Stdout, Write, Check}` separates output destination from fix permission.
- Private outcome aggregation returns 2 for any operational failure, otherwise 1
  for check changes or error findings, otherwise 0. Unit-test synthetic outcomes
  because production has no changes/findings yet.

- [ ] Write integration tests using `CARGO_BIN_EXE_mdfmt`: single-file/stdin byte-preserving stdout; check/write empty stdout; no-argument exit 2; help/version exit 0; incompatible modes, repeated/mixed stdin, and multiple stdout inputs exit 2. `--` handles dash-prefixed literal paths. Invalid UTF-8 emits a stderr diagnostic and no content.
- [ ] Run `mise exec -- cargo test --test cli`; confirm absent CLI behavior fails.
- [ ] Implement clap parsing and argument-order processing. Continue after per-file errors, deduplicate targets in check/write, and aggregate failures. Supply `fix: true` through the API now; do not add unsupported config flags. Test internal fix-disabled mode mapping without inventing CLI configuration loading.
- [ ] Add unit tests pinning exits for changed checks, warnings, remaining error findings, and operational-failure precedence, plus multi-file continuation. Ensure stdout contains only document bytes and no output claims lint cleanliness.
- [ ] Update README status/usage: foundation exists but has no production fixes or config loading; clearly distinguish future goals. Link parser evidence and retain compatibility-document caveats. Do not mark existing formatting acceptance cases as passing.
- [ ] Run `mise exec -- cargo fmt --check`, `mise exec -- cargo clippy --all-targets -- -D warnings`, and `mise exec -- cargo test --all-targets`; require each exits 0. Run locked oracle commands from the parser evaluation and record their actual outcome separately from Rust test results.
- [ ] Review `git diff --check`, `git diff --stat`, and the full diff for unintended files, secrets, untracked lockfiles, and claims beyond tested behavior. Request final review, address findings, and report exact verification results and remaining milestone boundaries. Commit only if requested.

## Planning checks

- [x] Foundation/CLI scope approved in conversation; implementation details captured in the linked design.
- [x] Fresh read-only builder check of the design: nothing important left.
- [x] Spec requirements mapped to tasks; later milestones explicitly excluded.
- [x] Interfaces checked across source, parser, edit, processing, file I/O, and CLI tasks.
- [x] Review-focus failure modes assigned concrete tests.
- [x] User waived further planning checkpoints and authorized native execution.
