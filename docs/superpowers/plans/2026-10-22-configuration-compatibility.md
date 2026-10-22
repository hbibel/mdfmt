# Configuration Compatibility Implementation Plan

> **For agentic workers:** Use native task-by-task execution with test-first changes and an independent final review. Steps use checkbox syntax for tracking.

**Goal:** Implement roadmap milestone 3 against markdownlint-cli2 0.24.0 and markdownlint 0.42.0 without adding rule fixes.

**Architecture:** Preserve ordered declarative rule objects, resolve filesystem configuration separately from document processing, and compute effective per-line rule permissions before any future transformation. Enumerate files natively and retain the foundation's safe I/O and clean output streams.

**Tech Stack:** Rust 1.99, ordered serde_json values, JSONC/YAML/TOML parsers, native glob matching and gitignore handling; pinned Node packages remain development-only oracles.

**Spec:** `docs/compatibility/configuration.md` and milestone 3 of `docs/superpowers/specs/2026-10-21-mdfmt-design.md`.

## Global constraints

- Never read `.env`; never execute configuration or install shared npm packages.
- Preserve declaration order and shallow rule-object replacement.
- Standalone rule configuration and cli2 option inheritance are distinct.
- Explicit configuration is a base; discovery still applies.
- TOML is explicitly loaded only. Unsupported executable features, custom regexes, and package resolution produce named errors.
- `fix: false` gates edits independently of stdout/write/check modes.
- No production rule fixes or lint findings ship in this milestone.
- Keep document bytes on stdout and all status/diagnostics on stderr.
- Retain safe file replacement and tracked Cargo.lock. Do not commit without a request.

## Review focus

- Malformed/unknown settings must not silently fall back to defaults (Tasks 1–2).
- Child cli2 configuration must not accidentally inherit a standalone parent (Task 2).
- Inline directives inside code/front matter follow upstream's raw-source scan, not AST comment detection (Task 3).
- Literal paths, symlink cycles, outside-tree paths, and ancestor gitignore files need explicit tests (Tasks 2 and 4).
- Config errors must prevent output/writes for affected documents, even though no rules run yet (Task 5).

## File map and interfaces

- `src/config/mod.rs`: `ConfigError`, ordered `RuleConfig`, effective `RuleState`, `ResolvedConfig`, and public exports.
- `src/config/rules.rs`: built-in registry, validation, aliases/tags, enablement/severity/parameters.
- `src/config/load.rs`: JSONC/YAML/TOML parsing, JSON Pointer, filesystem extends and unsupported-feature checks.
- `src/config/resolve.rs`: `ConfigResolver::new(invocation: &Path, explicit: Option<&Path>, pointer: Option<&str>)`, `base_options()` and `resolve(path: &Path)`.
- `src/config/directives.rs`: `ResolvedConfig::document(source: &str) -> Result<DocumentConfig, ConfigError>`, including `state(rule, line)` and `allows(rule, byte_range, source)`.
- `src/config/glob.rs`, `src/config/select.rs`: ordered glob filtering, native enumeration and `select_inputs(resolver, patterns, no_globs)`.
- `src/lib.rs`, `src/diagnostic.rs`: filesystem-free `format_with_config` and configuration processing errors; existing `format` remains usable.
- `src/cli.rs`: config/config-pointer/no-globs flags, enumeration and effective per-input configuration.
- `tests/config*.rs`: rule, resolution, directive, selection, and CLI regressions.
- `tools/parser-oracle/configuration.mjs`, recorded fixtures: actual pinned-upstream comparisons.
- `README.md`, `docs/compatibility/configuration.md`: tested coverage and limits.

## Task 1: Ordered rules and explicit validation

- [x] Add failing tests for case-insensitive aliases/tags, declaration order, shallow replacement, default severity, unknown rules/options and deferred regexes. `{"default":"warning","MD013":false,"line_length":{"line_length":42}}` enables MD013 at error severity and width 42.
- [x] Run `mise exec -- cargo test --test config_rules`; require the missing API to fail.
- [x] Implement ordered rule configuration, built-in metadata and effective states without injecting schema defaults. Reject unknown properties and invalid types explicitly.
- [x] Re-run the task tests; require all passing.

## Task 2: Declarative loading and discovery

- [x] Add failing tests for JSONC comments/trailing commas, YAML, explicit TOML/JSON Pointer, extends cycles and local-path precedence over packages.
- [x] Pin precedence examples from the contract: explicit base below discovered cli2, standalone above cli2, child standalone replaces parent, cli2 shallow inheritance, first matching override.
- [x] Implement discovery from invocation directory downward, separate inheritance families, outside-tree common-ancestor injection, and directory-relative override filters. Reject selected JS files and unsupported settings with path/property context.
- [x] Run `mise exec -- cargo test --test config_resolution`; require all passing, including no upward discovery and no automatic TOML.

## Task 3: Inline permissions

- [x] Add failing tests for configure-file, enable/disable-file, enable/disable, capture/restore, disable-line/next-line, noInlineConfig, multiple comments, multiline configure-file, BOM/CRLF, and directives in code/front matter.
- [x] Implement upstream's four-pass raw-source scan, preserving original bytes and severity. Compute per-line state and deny edits spanning any disabled line or a directive.
- [x] Run `mise exec -- cargo test --test config_directives`; require all passing and unchanged second-pass output.

## Task 4: File selection

- [x] Add failing tests for dot-only patterns, dotfiles, brace/extglob patterns, negative-only input, `!`/`#` exclusions, `:`/`--` literals, config globs/no-globs, nested ignores, gitignore ancestors, escaping, symlinks and outside-tree targets.
- [x] Implement deterministic native enumeration, directory expansion, literal-path handling, ordered filters and gitignore application. Never read document contents during enumeration. Diagnose unsupported pattern semantics rather than silently approximating them.
- [x] Run `mise exec -- cargo test --test config_selection`; require all passing.
- [x] Complete the native advanced-glob compiler, directed/generated escaping/Unicode/case comparisons and platform CI definition. Exhaustive platform/corpus hardening remains milestone 7; remote CI is not claimed executed locally.

## Task 5: Library/CLI integration and upstream evidence

- [x] Add failing integration tests for config flags, discovered failures preventing stdout/write, stdin configuration, showFound on stderr/noProgress suppression, and unchanged file safety.
- [x] Integrate effective fix permission and directive validation into the filesystem-free processing API and CLI. Keep all existing modes and exit aggregation.
- [x] Add a pinned Node development oracle comparing resolution, rule parameters/severity, directive enablement and selected paths with checked-in Rust fixture assertions. Record actual observations, not invented expected output.
- [x] Update documentation to distinguish implemented configuration behavior from unimplemented rule diagnostics/reflow and bounded compatibility evidence.
- [x] Run `mise exec -- cargo test --all-targets`, `mise exec -- cargo fmt --check`, `mise exec -- cargo clippy --all-targets -- -D warnings`, the pinned oracle, and `git diff --check`.
- [x] Obtain independent whole-change review, address findings, repeat relevant verification, and report worktree location and remaining boundaries. Leave changes uncommitted unless requested.

## Execution decisions

The user delegated all decisions and waived questions/checkpoints. Use native execution and independent final review; implementation may proceed without a separate approval prompt. The repository starts clean; all 27 baseline Rust tests pass. The isolated worktree is `configuration-compatibility`.

## Execution record

- Tasks 1–3 are implemented with tests: ordered rules and pinned schema constraints,
  declarative loading/inheritance, and effective inline permissions.
- Task 4 implements the documented tested subset, not complete globby/micromatch
  parity. Unsupported advanced forms are diagnosed; broader parity remains part
  of milestone 3 and is not silently marked complete.
- Task 5 integrates the library/CLI and adds 30 recorded upstream scenarios plus
  a 53-rule option-schema snapshot. No formatting rule is marked implemented.
- Independent review identified ordinary-pattern, ordering, parser-priority,
  whitespace and validation differences. Each finding gained an upstream-backed
  or direct regression before its fix. Final targeted follow-up found no remaining
  blockers in those fixes; it did not establish full advanced-syntax parity.
- The pinned Node development oracle has existing npm audit advisories, documented
  separately from Rust runtime dependencies. Reference versions were not changed.
- The initial implementation was left uncommitted. The user subsequently
  authorized committing and merging this partial milestone into `main`.
- The preserved worktree is now a sibling of `main` at
  `/home/hannes/projects/mdfmt/configuration-compatibility`, on
  `feat/configuration-compatibility`. Shared OpenCode configuration is inherited
  from the workspace parent.
- Relocation left cached integration-test executables pointing at the old path.
  A scoped `cargo clean -p mdfmt` and rebuild resolved the cache issue without
  source changes; all 54 Rust tests passed again before integration.
- Final verification: 54 Rust tests, rustfmt, Clippy with warnings denied, 22
  parser observations, 30 configuration observations, 53 pinned option schemas,
  and diff whitespace checks pass. Npm audit advisories remain as documented.

### Task 4 continuation

- Removed the clean, already-integrated `configuration-compatibility` worktree
  and branch after user authorization. Continued in the sibling worktree
  `/home/hannes/projects/mdfmt/configuration-globs`, on `feat/configuration-globs`.
- Added bounded native brace-range expansion, ASCII POSIX classes, repeated
  negation, and UTF-16 wildcard matching, preserving enumeration/filter differences.
- Expanded runtime evidence to 50 configuration scenarios and 41 direct pattern
  comparisons without changing the original 30 observations.
- Extglobs, some range-filter/backtracking forms, and broader platform parity
  remain unresolved. Task 4's final checkbox and milestone 3 remain open.
- The continuation is left uncommitted pending user direction.
- Independent review found three silent selection differences. Pinned upstream
  regressions reproduced each before fixing literal class operators, enclosing
  nested braces, and ancestor pruning for complemented exclusions.

### Task 4 exact-one extglob continuation

- Continued in the existing dirty `configuration-globs` worktree, preserving the
  earlier continuation. Baseline: 60 passing Rust tests.
- Implemented exact-one `@(...)` alternatives, including nesting, empty branches,
  ordinary classes, comma braces, Unicode and escaped literals. Repetition and
  negative extglobs remain for subsequent batches; milestone 3 stays open.
- Extended the pinned oracle to 67 filesystem scenarios and 83 direct patterns.
  Five filesystem scenarios and named direct contexts record upstream behavior
  while requiring a native unsupported diagnostic instead of silent divergence.
- Upstream comparisons exposed incompatible POSIX backtracking, globstars inside
  groups, cross-segment enumeration and range/filter interactions. Diagnose those
  combinations explicitly; do not claim full extglob parity.
- Verification before review: 61 Rust tests, rustfmt, Clippy with warnings denied,
  and the complete pinned Node suite pass. Leave changes uncommitted.
- Independent review found four Important selection issues. Regressions failed
  before preserving grouped negation and wildcard boundaries and diagnosing
  post-group quantifiers and leading question marks. All 65 Rust tests and the
  expanded pinned suite pass after the fix pass. Milestone 3 remains incomplete.

### Task 4 optional and repeated literal alternatives

The user requested committing the current state, then continuing. Commit
`48e2925` records the verified brace/class/exact-one batches. Continue natively
in the same worktree; do not merge, push, or automatically commit later changes.

Goal: support `?(a|b)`, `+(a|b)` and `*(a|b)` with literal alternatives, without
claiming complete positive/negative extglob compatibility. Reuse the existing
regex fragment mechanism; no runtime Node dependency or new Rust crate.

Files and interfaces:

- `src/config/extglob.rs`: change `translate`'s fragment callback to
  `FnMut(String) -> String`; exact-one boundaries insert an empty regex string,
  while optional/repeated literal branches insert an escaped grouped regex.
  `src/config/repeat.rs` parses literal alternatives and diagnoses upstream safety
  rewrites, unsupported syntax and excessive alternative counts.
- `src/config/glob.rs`: retain that assertion as a matcher predicate evaluated
  inside outer negation using `needs_initial_character(pattern: &str) -> bool`.
  Continue UTF-16 conversion exactly once per fragment. Expanded fixtures also
  require that guard for leading POSIX classes.
- `tests/fixtures/configuration/{glob-cases,cases}.json` and observations: append
  pinned runtime evidence, including unsupported nested/magic repeated bodies.
- `tests/config_selection.rs`: keep negative and unimplemented forms diagnosed.
- `README.md`, configuration evidence: document supported literal bodies and limits.

Review focus:

- Empty and repeated-character prefix-overlap alternatives: upstream may replace
  repetition with a literal pattern for safety; diagnose instead of approximating.
- Leading empty/newline inputs: upstream's initial assertion is not a dotall regex.
- Unicode repetition and escaped glob characters: preserve UTF-16 units and literals.
- Adjacent groups/wildcards and grouped bangs: do not regress exact-one boundaries.
- Nested/magic bodies and resource growth: explicit errors, not partial selection.

Checklist:

- [x] Append runtime fixtures for literal optional/repeated alternatives, Unicode,
  escaped punctuation, empty groups, ordered exclusions, overrides and ignores.
  Run `node tools/parser-oracle/globs.mjs --record` and the configuration oracle;
  watch native tests fail with `unsupported-glob` before writing implementation.
- [x] Implement literal-body parsing and regex fragments. Reject nested groups,
  unescaped wildcards/classes/braces/quotes, path separators, unsafe escapes and
  upstream-literalized repeated alternatives. Preserve prior unsupported boundaries.
- [x] Pin and implement the initial-character assertion, outer negation behavior,
  malformed bodies and bounded total extglob groups, with RED→GREEN tests.
- [x] Run all Rust tests, the complete pinned oracle, rustfmt, Clippy and diff
  checks. Obtain one independent final review; fix Important findings test-first.
- [x] Update evidence and ledger; leave the next batch uncommitted and milestone
  3 open while negative extglobs and remaining parity work are outstanding.

Execution outcome: 69 Rust tests pass, along with 84 pinned configuration scenarios,
126 direct pattern observations, rustfmt, Clippy and whitespace checks. Independent
review found six Important context interactions. Each gained a failing regression
before explicit diagnostics for unsupported surrounding star/brace/regex syntax.
All 67 configuration observations committed at `48e2925` remain unchanged.

### Milestone 3 completion

The replacement compiler supersedes the incremental unsupported-syntax batches.
All former native-error scenarios now match actual upstream behavior. Final
evidence includes 110 configuration scenarios, 126 directed glob cases, 755
generated patterns and 755 actual cli2 filesystem selections. Independent review's
four Important findings gained failing regressions and fixes, including a local
regex-engine state-restoration correction. The 69 Rust tests and 537 vendored
engine tests pass. See `docs/compatibility/configuration-completion.md` for the
current contract, deliberate policies and unexecuted remote-platform boundaries.
