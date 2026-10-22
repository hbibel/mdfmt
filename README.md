# mdfmt

mdfmt is a Markdown formatter being built in Rust. It aims to fix safely
actionable markdownlint violations using existing declarative
[markdownlint-cli2](https://github.com/DavidAnson/markdownlint-cli2)
configuration, including prose reflow that goes beyond `markdownlint-cli2 --fix`.

## Status

The Rust library and CLI foundation exist: source-aware parsing, validated
localized edits, UTF-16 coordinate accounting, and safe file replacement.

Declarative configuration loading, discovery/inheritance, inline rule permissions,
and native file selection are implemented; see the
[compatibility evidence and limits](docs/compatibility/configuration-completion.md).
No production rule fixes or rule diagnostics exist yet. The CLI preserves input
unchanged; a successful check does not mean the document is markdownlint-clean.

## Try the foundation

With the toolchain from `mise.toml`:

```sh
mise exec -- cargo run -- document.md
mise exec -- cargo run -- --check document.md
mise exec -- cargo run -- --write document.md other.md
mise exec -- cargo run -- --check "**/*.md" "#vendor/**"
mise exec -- cargo run -- --check --config rules.toml --config-pointer /markdown document.md
cat document.md | mise exec -- cargo run -- -
```

The default prints one document without changing its file. `--check` writes
nothing; `--write` accepts multiple literal files and updates changed files only.
Use `:` or `--` for literal filenames, including names containing glob characters.
Directories expand recursively; `.` alone selects Markdown files in the current
directory. Configuration globs append unless `--no-globs` is set. Mixed stdin/file
input is not supported. Stdout mode requires exactly one selected input.
Tested glob support includes numeric/alphabetic brace ranges, POSIX character
classes, UTF-16 matching and all five extglob operators with nesting. Enumeration
and configuration filters preserve their distinct upstream semantics, including
brace expansion, negation and directory pruning. See the compatibility evidence
for safety limits and intentional policy differences.

Automatic discovery supports the declarative `.markdownlint-cli2.*` and
`.markdownlint.*` names in the configuration contract. TOML requires `--config`.
An explicit config supplies a base and does not disable discovery. JavaScript
configuration, custom regexes, and package-based `extends` are rejected explicitly;
filesystem-based `extends` is supported. `fix: false` prevents edits in every mode.

Diagnostics go to stderr. Exit codes are 0 for successful processing, 1 for
needed changes in check mode or remaining error findings, and 2 for usage or
processing failures. No rules currently produce changes or findings.

Writes reject symlinks and, on Unix, multiply linked files. Replacement preserves
ordinary permissions and rejects observed changes since reading. It does not
promise adversarial race protection, preservation of extended metadata, or crash
durability. On other platforms, identity deduplication uses canonical paths and
does not detect hardlink aliases.

## Formatting goals

- Reflow prose to the configured character limit, even when markdownlint would
  exempt the original line from reporting.
- Keep inline links intact, wrapping before them when needed; never convert
  them into reference links.
- Preserve tables and avoid unsafe rewrites.
- Apply configured rule fixes, including MD026 trailing heading punctuation.
- Delegate fenced code to an available language formatter without losing code
  on failure.
- Produce identical output when formatting a document a second time.

The first release targets local declarative configuration and filesystem-based
`extends`. Later releases include custom regex compatibility and resolution of
already-installed shared configuration packages. Package resolution will not
download or install npm packages. JavaScript configuration execution is outside
scope.

## Project documents

- [Roadmap and requirements](docs/superpowers/specs/2026-10-21-mdfmt-design.md)
- [Compatibility baseline](docs/compatibility/baseline.md)
- [Configuration contract](docs/compatibility/configuration.md)
- [Rule inventory](docs/compatibility/rules.json)
- [Acceptance examples](docs/compatibility/acceptance.json)
- [Audit verification](docs/compatibility/verification.md)
- [Parser evaluation](docs/compatibility/parser-evaluation.md)

## Development

The Rust toolchain is declared in `mise.toml`; `Cargo.lock` is tracked.

```sh
mise exec -- cargo test --all-targets
mise exec -- cargo fmt --check
mise exec -- cargo clippy --all-targets -- -D warnings
```

Tests cover preservation, source positions, protected regions, localized edits,
idempotence, CLI streams, safe replacement, configuration and file selection.
Existing formatting acceptance examples remain future contracts, not passing rule tests.
The parser evaluation documents a separate pinned Node development oracle; Node
is not needed to build or run mdfmt.

Project-local OpenCode configuration allows tools without approval prompts,
prefers GPT-6.1 Sol, and excludes the `git-commit` skill. Use it only in a trusted
workspace; broader OpenCode policies and agent-specific restrictions still apply.
