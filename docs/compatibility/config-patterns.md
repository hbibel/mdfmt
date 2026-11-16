# Native rule configuration patterns

Custom rule patterns execute natively without JavaScript or process I/O:

- MD001/MD025/MD041 `front_matter_title` uses case-insensitive, non-Unicode
  JavaScript matching against recognized metadata lines; empty disables the title
  effect. Defaults still follow the pinned source fallback, including quoted keys.
- MD043 `%`-prefixed headings match regex text at the required level with Unicode
  semantics and optional case-insensitivity. Mixed `%#` prefixes retain the pinned
  level grammar. Literal headings, loops and wildcards keep their existing behavior.
- MD051 `ignored_pattern` is unflagged and tests the unescaped fragment without
  its leading `#`. Ignored destinations receive neither findings nor case repairs.

The existing vendored ECMAScript engine supports lookaround and backreferences;
Rust's regex engine is not substituted for these consumers. Unicode/non-Unicode
mode preserves astral/surrogate and case-fold distinctions. Aliases, inline
configuration, enablement, severities and edit permissions remain authoritative.

## Limits and failures

Pattern length is limited to 65,536 UTF-16 units and group nesting to 64. Each
document configuration cache holds at most 256 compiled patterns and 1 MiB of
source patterns. Compiled handles share immutable state; no global cache exists.
Matching consumes deterministic engine fuel and performs one search, not an
unbounded loop over zero-length matches.

Invalid configured expressions fail with named `invalid-config` properties.
Execution/cache exhaustion propagates as `ProcessingError::WorkLimit`, preventing
output or writes rather than becoming a nonmatch. Disabled rules do not execute
their patterns. A later independent safe document is unaffected.

## Evidence and remaining scope

Four public tests cover actual repairs/findings, aliases/inline controls, bounded
cache/fuel failures, CLI check/write preservation, repeat stability and UTF-16
flag distinctions. A development-only oracle compares 42 actual pinned
markdownlint0.42.0 cases with native diagnostics and unchanged source. Existing
554 content-diagnostic, 1,144 fragment, 110 config/selection and 53 schema
comparisons pass. The former explicit-deferred assertions now test real behavior;
invalid expressions still fail explicitly. Complete optimized verification passes
565 tests across 76 targets, with fmt/clippy/diff checks clean.

Custom cli2 [frontMatter recognition](front-matter.md) is now implemented with a
separate pinned RE2-style contract. Complete combined verification passes 570 tests
across 77 targets. Independent batch review is pending; complete ECMAScript/all-input/platform parity and
release-hardening completion are not claimed.

```sh
mise exec -- cargo test --release --test config_patterns
mise exec -- cargo build
node tools/parser-oracle/config-patterns.mjs
```
