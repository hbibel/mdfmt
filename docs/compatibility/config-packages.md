# Installed declarative configurations

`extends` can name an already-installed shared config, including scoped packages
and exported subpaths. The Rust application does not run Node, install packages,
download from a registry or read npm credentials.

## Lookup and parsing

The exact filesystem path relative to the containing config wins first, even if
it is a directory or an unsupported config format. Home-relative and absolute
references retain their existing behavior. Otherwise native CommonJS-style
lookup searches ancestor `node_modules`, with `.js`, `.json`, `.node` extension
order and package main/index fallback. Package exports respect metadata key order,
Node24 default conditions, subpath patterns and blocked/invalid targets.

Only selected JSON/JSONC/YAML/YML/TOML configs are parsed. Selecting JavaScript,
native code or an extensionless file produces `unsupported-config-format`, not
execution or reselection of a more convenient candidate. Missing or invalid
references produce `invalid-config` naming `extends`, the referring file and the
reference. Errors leave Markdown inputs untouched, including in write mode.

Package-selected symlinks use the real target's scope. Direct existing paths keep
their existing lexical relative scope. Canonical identities and the 128-file
inheritance bound still detect cyclic or excessive chains. Standalone inheritance
keeps inherited-before-local order; cli2 config/overrides retain local key positions.
The package adapter does not own parsing, cycle detection or merges.

## Boundaries

- No global module folders, `NODE_PATH`, PnP, custom module roots, browser fields,
  TS aliases, registry access or executable configuration.
- No config-local package imports/self-reference. The pinned markdownlint loader
  captures its library `require.resolve`; extra lookup paths do not change that
  parent-module scope. This is not a generic caller-scoped Node loader.
- No arbitrary custom Node conditions or `NODE_OPTIONS` interpretation.
- No all-input or cross-platform filesystem parity claim. Runtime evidence is Linux.

## Evidence and implementation

Seven native public tests pass, including literal output/idempotence, standalone/
cli2/override key order, package-relative/nested-package chains, exact shadows,
symlinks, cycles/depth limits, missing references, marker-bearing executable
rejection and isolated-environment/fresh-resolver controls.

The development-only oracle records 102 actual markdownlint0.42.0/cli2 0.24.0
observations and consumes them in Rust and through the CLI. It checks effective
configs/key order, formatting, pinned lint results, failures and repeat stability.
Selected unsupported formats are explicit intentional differences. Initial native
tests and CLI probes failed before implementation.

The pinned native resolver is vendored with narrow source-backed corrections;
see `vendor/oxc_resolver/MDFMT-PATCHES.md` for provenance. The initial dependency
incorrectly blocked top-level null exports, accepted numeric conditional keys,
fell through terminal conditional null and invented config-local imports/self
success. Expanded cases failed before correction and now pass. Unselected invalid
branches remain unvisited, numeric hard errors do not fall through arrays, and
all-undefined arrays retain enclosing conditional fallback.
No on-disk package metadata is rewritten.

The independent review found six Important selection defects, with no Critical
findings. All were reproduced before the single fix pass. Exports now require an
exact file rather than legacy extension/directory fallback; removed folder maps
cannot expose blocked subpaths. Raw and encoded invalid segments are checked before
normalization, with array-fallback error classes preserved. Exports URL filenames
are decoded and query/fragment components are not appended to native paths. Literal
CommonJS references keep literal percent/query/hash bytes. Both metadata backends
require object roots, and legacy file lookup precedes directory fallback. Public
check/write tests pin exact BOM/CRLF bytes and exit codes across every review row,
including conflicting configurations that previously caused unwanted edits.

Complete post-review optimized verification passes 561 tests across 75 targets.
Focused debug configuration tests pass 54 tests across seven targets; 110 existing config/
selection observations, 53 rule schemas and 755 actual cli2-selection cases pass.
Root fmt/clippy/diff and the patched vendor files' upstream-style rustfmt check
are clean after the fixes. The independent review and its single regression-first
fix pass are complete; no second review was dispatched. Custom regex
compatibility and release hardening remain separate unfinished roadmap items.

Known conservative mismatches remain for uncommon unknown `#` condition names,
malformed metadata in otherwise unused nested scopes and overflowing numbers in
unused metadata fields. These fail closed rather than selecting different rules.
The review did not elevate them under the Important/Critical-only scope; their
explicit boundary is not an all-input compatibility endorsement.

```sh
mise exec -- cargo test --release --test config_packages
mise exec -- cargo build
node tools/parser-oracle/config-packages.mjs
```
