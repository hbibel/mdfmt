# mdfmt

mdfmt is a Markdown formatter being built in Rust. It aims to fix safely
actionable markdownlint violations using existing declarative
[markdownlint-cli2](https://github.com/DavidAnson/markdownlint-cli2)
configuration, including prose reflow that goes beyond `markdownlint-cli2 --fix`.

## What `mdfmt` does

mdfmt reflows prose to your configured line width and applies safe Markdown fixes
using your existing declarative markdownlint configuration. It keeps inline
links intact and preserves table contents. Use `--check` to see what needs
attention without changing files.

mdfmt covers all 53 built-in markdownlint rules. Some rules only report problems,
and some fixes apply only when they can preserve the document’s meaning.
Coverage does not mean full markdownlint compatibility. See the
[compatibility baseline](docs/compatibility/baseline.md) and
[rule inventory](docs/compatibility/rules.json) for details.

## Usage

From an extracted native archive:

```sh
./mdfmt --help
./mdfmt document.md
./mdfmt --check document.md
./mdfmt --write document.md
```

On Windows, use `.\mdfmt.exe`. If an archive extractor drops executable permissions,
restore them with `chmod +x mdfmt` on Unix. Running the binary requires a compatible
OS/runtime, not Rust, Python or Node. Documentation links refer to the source
checkout; `--help` is available in the archive.

From source, with the toolchain from `mise.toml`:

```sh
mise exec -- cargo run -- document.md
mise exec -- cargo run -- --check document.md
mise exec -- cargo run -- --write document.md other.md
mise exec -- cargo run -- --format-code --check document.md
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
configuration is rejected explicitly. Native
[rule configuration patterns](docs/compatibility/config-patterns.md) are supported.
Custom [front-matter recognition](docs/compatibility/front-matter.md) preserves
matched metadata and rejects unsupported pattern forms explicitly.
Filesystem and locally
installed declarative package `extends` are supported without running Node or
installing anything; see [package lookup limits](docs/compatibility/config-packages.md).
`fix: false` prevents edits in every mode.

Diagnostics go to stderr. Exit codes are 0 for successful processing, 1 for
needed changes in check mode or remaining error findings, and 2 for usage or
processing failures. Warnings alone do not cause exit 1. `fix: false` prevents
edits but does not suppress configured diagnostics.
Optional `--format-code` uses installed formatters on Unix. If a formatter fails
or is unavailable, or the platform is unsupported, mdfmt leaves the code unchanged
and warns. See [supported formatters and limits](docs/compatibility/code-formatters.md).
Code formatting requires enabled MD013/code-block checking and full fence
permission. No formatter is downloaded, and ordinary library APIs do not
execute processes. The explicit process-I/O library entry point is
`format_with_code_formatters` with `CodeFormatOptions { enabled: true }`.

Reflow joins soft breaks and wraps at ordinary spaces, counting source widths in
UTF-16 units, including container prefixes. It preserves existing horizontal
spacing when it fits. Paragraphs containing tabs or multiline protected inline
syntax are left alone. Non-breaking spaces, hard breaks, tables, code blocks, and
protected inline content are not rewritten. If reparsing detects a structural or
protected-content change, the document's wrapping batch is discarded. Unavoidable
long tokens can remain MD013 findings according to the configured lint exceptions.
Documents with non-comment HTML, or paragraphs containing `$` or a colon followed
by an ASCII letter, are conservatively preserved until HTML, math, and directive
boundaries are modeled. This also leaves some ordinary prose unwrapped.

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
Offline [corpus/property checks](docs/compatibility/corpus-hardening.md) and
[local native packaging](docs/compatibility/native-artifacts.md) have separate
commands and evidence. Packaging tools are development-only, not formatter
runtime dependencies.

```sh
mise exec -- cargo test --all-targets
mise exec -- cargo fmt --check
mise exec -- cargo clippy --all-targets -- -D warnings
```

Tests cover preservation, source positions, protected regions, localized edits,
idempotence, CLI streams, safe replacement, configuration and file selection.
Implemented-rule suites exercise applicable formatting acceptance examples
and pinned upstream diagnostics. Other-rule and external-formatter examples remain future
contracts, not completed rule tests.
The parser evaluation documents a separate pinned Node development oracle; Node
is not needed to build or run mdfmt.

Project-local OpenCode configuration allows tools without approval prompts,
prefers GPT-6.1 Sol, and excludes the `git-commit` skill. Use it only in a trusted
workspace; broader OpenCode policies and agent-specific restrictions still apply.
