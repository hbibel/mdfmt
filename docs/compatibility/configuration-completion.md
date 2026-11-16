# Native configuration compatibility evidence

This records milestone 3 evidence for the first-release declarative
configuration contract. The earlier [batch record](configuration-evidence.md)
is historical; its unsupported-extglob lists no longer describe the implementation.
Subsequent rule/reflow work is documented in the README; later installed-package
support has [separate evidence](config-packages.md).

Milestone 3 is implemented for that contract. Final local verification passes
69 application tests and 537 vendored-engine tests, plus every upstream comparison
below. Remote CI execution and exhaustive platform hardening are not claimed.

## Implemented contract

- JSON/JSONC/YAML discovery and explicit TOML/JSON Pointer loading; filesystem
  `extends`, separate cli2/standalone inheritance and ordered shallow replacement.
- Built-in IDs, aliases/tags, options, severities, first-match overrides, all
  supported inline directives and independent `fix: false` permission.
- Native file selection, dotfiles, directory expansion, ordered exclusions,
  literal paths, config globs and outside-tree inputs.
- All five extglob operators with nesting, ordinary groups/quantifiers, Unicode,
  quotes and escaping; numeric/alphabetic brace expansion; POSIX classes and
  pinned legacy class/range backtracking. Enumeration and filtering remain distinct.
- Gitignore true/string discovery, ancestor scope, per-file anchored rules,
  case-insensitive matching, ordered whitelists and excluded ignore-search trees.
  User Git configuration excludes are not read: pinned cli2 does not enable
  globby's separate `globalGitignore` option.
- Deterministic enumeration with directory pruning, cycle termination and clean
  CLI streams. Regex work-limit failures propagate as processing errors before
  output or writes; they are not treated as nonmatches.

## Reference and implementation

The compatibility reference remains markdownlint-cli2 0.24.0, markdownlint 0.42.0,
micromatch 4.0.8 and picomatch 2.3.2 from the locked development installation.
The picomatch parser was byte-compared with its integrity-verified locked npm
tarball; its SHA-256 is
`3a91656f440069aa8f8e15c9d695b9f6b2209df1eab2be8127c72cbcca45ff5f`.

`src/config/picomatch/` ports the token grammar for the matching options used by
cli2. Separate token values and output fragments preserve upstream's backtracking
and repetition-safety rewrites. In particular, source quirks are not approximated
with isolated brace/extglob substitutions. Generated invalid regexes become dead
matchers, matching upstream; malformed configuration still receives named errors.

Execution uses the native `regress` package vendored from the Picomatch Mortis
repository at `d0684188b1c39275f3caaab578bd94a9c0c5bc25`. It provides deterministic
execution fuel and prohibits unsafe code. A local, regression-tested correction
preserves loop state when nested repetitions backtrack. Normal builds do not
fetch a Git dependency. Only its regex engine is used: the
separately evaluated full glob port differed on six recorded cases and is not
the compatibility reference. Raw UTF-16 parser input and decoded surrogate escapes
preserve code units in both literals and classes, including range validation.
The vendored engine's provenance, patch and independent test command are recorded
in `vendor/regress/UPSTREAM.md`.

`src/config/brace.rs` expands enumeration alternatives before matching.
`src/config/glob.rs` and `select.rs` reproduce traversal-root selection and partial
directory pruning separately from full-path matching. Glob errors prevent the
invocation from processing documents.

## Reproducible evidence

- 110 filesystem/configuration/directive scenarios compare actual selected paths
  and effective rule state. Former `nativeError` cases now compare actual results;
  none are skipped as unsupported syntax.
- 126 directed patterns compare native brace expansion, enumeration regex matching
  and micromatch filtering over the recorded paths.
- 755 deterministic generated patterns compare expansion and matching over 38
  paths, including empty/newline strings, Unicode, escaping and nested groups.
- The same 755 patterns are executed by actual cli2 over a 22-file tree and
  compared with native selection. Sixteen explicitly marked missing-named-input
  cases assert the foundation's operational-error policy rather than upstream's
  silent empty selection. The other 739 compare selected paths exactly.
- Existing 22 parser observations, 53 rule-option schemas, configuration tests,
  preservation/idempotence tests and safe-write regressions remain in the suite.
- Hostile repeated expressions test explicit, recoverable regex exhaustion;
  excessive pattern length, nesting and brace allocation are regression-tested.
- Final review regressions cover nested repetition engine state, escaped surrogate
  pairs, embedded quote tokenization and partial pruning during custom ignore-file
  discovery. All four findings were reproduced before their fixes.

Run from the repository root:

```sh
mise exec -- cargo test --all-targets
mise exec -- cargo fmt --check
mise exec -- cargo clippy --all-targets -- -D warnings
npm test --prefix tools/parser-oracle
git diff --check
```

The Node suite is a development oracle only. Normal runs reproduce checked-in
observations; `--record` is an explicit fixture-maintenance operation. Existing
development dependency advisories remain as documented in the historical evidence;
reference versions were not changed.

## Deliberate boundaries

- JavaScript configuration and executable extensions remain unsupported. Custom
  front-matter recognition remains later-release work; native rule regexes now
  have [separate evidence](config-patterns.md). Installed-package `extends`
  is implemented in a subsequent batch with explicit local-lookup/selected-format
  boundaries. No npm download is performed by mdfmt.
- Invalid/unknown configuration is rejected rather than reproducing every parser
  coercion. Missing JSON Pointers are errors. Configuration is cached per invocation,
  not watched for changes.
- Missing named inputs retain operational-error behavior. `--` keeps the
  foundation's literal-filename contract. Enumeration order is deterministic,
  not upstream's parallel scheduling order.
- On POSIX, native paths preserve literal backslashes in discovered filenames
  rather than aliasing them to different paths. Such filenames are not part of
  the generated filesystem parity corpus; backslash pattern semantics are tested
  separately over strings.
- Pattern length is limited to 65,536 UTF-16 units and structural nesting to 64.
  Native compiler output is bounded at 1 MiB and its token arena at 65,536 entries.
  Brace ranges have safe-integer endpoints, at most 1,000 values each, 4,096
  alternatives per expansion and a 1 MiB conservative allocation budget.
  Exceeding limits is an error, never partial selection.
- Runtime evidence here is Linux. CI defines full Linux/macOS Rust jobs, focused
  Windows library/configuration jobs and a Linux upstream oracle job. These remote
  jobs have not been executed from this local session. Exhaustive multi-platform
  filesystem/corpus/fuzz validation remains release-hardening work in milestone 7.

Finite differential evidence establishes the tested contract, not an all-input
proof or a claim that documents are markdownlint-clean.
