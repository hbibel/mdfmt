# Installed Config Packages Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve already-installed declarative shared configs through native local package lookup without executing or installing anything.

**Architecture:** Keep exact relative-path precedence in the current loader, then use a private CommonJS resolver adapter for fallback. Existing parsers, inheritance cycle checks and ordered merges consume the selected file unchanged.

**Tech Stack:** Existing Rust/mise toolchain and pinned `oxc_resolver = 11.24.3`, default-features false. Pinned Node tooling is development-only differential evidence.

**Spec:** `docs/superpowers/specs/2026-11-15-installed-config-packages-design.md`, binding roadmap and existing configuration/precedence contracts.

## Global Constraints

- Native Rust; no Node runtime, package download/install, registry/.npmrc access or JavaScript/native-module execution.
- Exact existing filesystem target shadows package lookup; preserve home/absolute paths, referring-file relative extends and canonical128-level cycle protection.
- Only JSON/JSONC/YAML/YML/TOML target formats are supported after selection; unsupported targets must error, not fall through to another candidate.
- Preserve ordered standalone/cli2/override shallow merges, aliases/tags, discovery and file selection. Unsupported regexes/executable modulePaths stay explicit separate boundaries.
- Local ancestor node_modules only; explicitly disable NODE_PATH/PnP/browser/TS aliases/custom roots. Default Node24 metadata conditions include node/require/module-sync/node-addons/default with metadata order significant.
- Regression-first inline execution, one independent review and Important/Critical fix pass; no second review, no merge/push or routine prompts. Close adapter review before implementation.

## Review Focus

- Local shadows, nested ancestor packages and symlinked configs must resolve relative to the correct lexical/real referring file without changing merge order; owning test `package_targets_preserve_referring_scope_and_key_order`.
- Exports arrays/conditions/null/patterns and invalid metadata must not choose a convenient existing file that Node would reject; owning differential and literal metadata tests.
- Selected JS/native/unsupported targets must never execute, get reselected or permit writes on error; owning marker/error CLI tests.
- Package cycles, aliases and malformed targets must terminate under existing limits with provenance-bearing errors, not silently substitute defaults; owning cycle/failure tests.
- Resolver options must not consult global/NODE_PATH/PnP/TS/browser configuration or contaminate later resolver instances with stale caches; owning isolated child-environment and fresh-resolver tests.

## Task 1: Native installed-package configuration resolution

Files: create `src/config/packages.rs`, `tests/config_packages.rs`, `tests/fixtures/config-packages/cases.json`, `tools/parser-oracle/config-packages.mjs`, `docs/compatibility/config-packages.md`; modify `src/config/mod.rs`, `src/config/load.rs`, Cargo.toml/lock, README, configuration contract/evidence and roadmap. Extend old unsupported-package assertion to the new actual missing-package failure without removing provenance checks.

Interfaces:

- `packages::resolve(config_path: &Path, reference: &str) -> Result<PathBuf, ConfigError>` consumes an expanded reference, returns the exact existing path first, otherwise the locally resolved native CommonJS target. Errors carry `extends` plus referring config/reference provenance. No public API expansion is required.
- Configure Resolver options explicitly: extensions `[.js,.json,.node]`, main_fields `[main]`, main_files `[index]`, modules `[node_modules]`, conditions `[node,require,module-sync,node-addons,default]`, symlinks true, NODE_PATH false, PnP false, no aliases/tsconfig/browser/custom roots. Keep metadata's own branch order.
- The existing load_rules parser/active-canonical set owns format rejection, recursion and ordered merge. Do not add a parallel config parser or special package merge path.
- Source-only oracle rows record actual pinned upstream effective configuration or selected failure class for isolated installed-package fixtures; native consumers assert effective MD013/other rule states, ordered behavior, CLI output/status/source preservation and repeated identity. Never execute a target merely to resolve it.

- [ ] Write `installed_main_index_scoped_and_subpath_configs_drive_native_formatting` with literal configs: inherited defaultfalse/MD013width24 plus local MD026true formats prose/headings correctly and settles; nested package overrides select nearest ancestor, YAML/JSONC/TOML explicit targets parse natively.
- [ ] Write `package_targets_preserve_referring_scope_and_key_order` for package→relative→package chains, standalone inherited-first versus cli2/override in-place positions, direct local file/directory shadows and conditional Unix symlinks. Assert effective alias/tag behavior and exact formatting, not a mirrored resolver expectation.
- [ ] Write metadata controls for exports string/default-before-require/require-before-default/module-sync/node-addons/pattern/null/array first-valid-missing behavior, imports/self-reference, malformed metadata/targets and invalid-main index fallback. Pin literal selected widths or explicit failures against actual Node24/pinned sources.
- [ ] Write missing/cyclic/alias/depth-limit and executable-marker CLI tests: exit2, path/property/reference named, original file exact in --check/--write, no JS marker or install side effects. Test NODE_PATH/PnP/browser/TS settings in isolated child environments and fresh resolver objects, never mutate process-global environment during parallel tests.
- [ ] Run `mise exec -- cargo test --release --test config_packages`. Expected: package-dependent tests fail with current unsupported-config-resolution, not malformed fixtures. Generate source-only oracle receipts, then observe native consumer RED before implementation.
- [ ] Add the pinned dependency/private adapter and replace only the loader's fallback target selection. Explicit options and existing parser/cycle/merge consumption must satisfy literal and source-backed tests; add only source-justified adapter corrections if resolver-library defaults differ.
- [ ] Run focused config/CLI tests, `node tools/parser-oracle/config-packages.mjs`, existing configuration/selection oracles, complete optimized suite, fmt/clippy/diff. Expected: ordinary config/formatting unchanged, source-backed package cases green, no downloads/execution by the application. Unsupported parser/JS formats stay explicit intentional differences.
- [ ] Update README/contracts/evidence/roadmap without claiming custom-regex/hardening completion; commit `feat: resolve installed declarative configuration packages`, task-done with focused tests and full-suite evidence.
- [ ] Request one fresh independent review with every Review Focus and chronological ruling. Reproduce/fix Important/Critical findings once regression-first, full verification; retain all rulings/costs before only completed scratch cleanup.
- [ ] Continue custom JavaScript regex compatibility and release hardening autonomously, retaining branch/worktree without merge/push.

## Planning check

One task carries one coherent loader feature; no disconnected infrastructure-only
task is needed. The adapter returns the same PathBuf the existing loader already
consumes. Existing direct-path/canonical-cycle/ordered-merge policies stay in
their current owners. Every Review Focus names owning tests. Actual pinned
probes establish modern conditions, first-array-target failures and direct shadow
behavior; no native package support or resolver-library parity is claimed yet.
The prior adapter review closed at `de5d61e` with both Important findings fixed
regression-first, complete optimized554/554 across74targets and all19 decisions
retained before completed scratch removal. This plan may begin in the same
existing sibling worktree without a configuration/session move.
