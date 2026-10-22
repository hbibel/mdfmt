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
- [ ] Complete advanced glob support and broader escaping/Unicode/case/platform parity before declaring milestone 3 complete.

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
