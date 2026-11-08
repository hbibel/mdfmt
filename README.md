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
MD013 prose reflow and remaining-violation diagnostics are implemented, including
multiline paragraphs, nested lists, and blockquotes. Reflow respects configured
widths and inline permissions, preserves hard breaks and indivisible inline
syntax, and checks the parsed structure before applying changes. MD026 removes
configured trailing heading punctuation, respecting rule permissions and upstream
entity/emoji exceptions; see [MD026 evidence](docs/compatibility/md026.md).
MD047 adds a missing final newline where it is safe, preserving literal EOF
boundaries with a remaining diagnostic; see [MD047 evidence](docs/compatibility/md047.md).
MD012 reduces excess blank lines where reparsing confirms preservation;
see [MD012 evidence](docs/compatibility/md012.md).
MD019/MD021 normalize ATX heading separators without changing heading text;
see [heading-spacing evidence](docs/compatibility/heading-spaces.md).
MD035 normalizes thematic-break styles when contextual reparsing confirms safety;
see [MD035 evidence](docs/compatibility/md035.md).
MD004 normalizes unordered-list markers without merging parsed lists or changing
nesting; see [MD004 evidence](docs/compatibility/md004.md).
MD009 removes incidental trailing whitespace while preserving hard breaks and
protected source; see [MD009 evidence](docs/compatibility/md009.md).
MD010 replaces hard tabs where configured substitution preserves meaning;
see [MD010 evidence](docs/compatibility/md010.md).
MD023 removes heading indentation where container ownership is unchanged;
see [MD023 evidence](docs/compatibility/md023.md).
MD040 reports missing/disallowed fence languages and extra info without guessing
replacements; see [MD040 evidence](docs/compatibility/md040.md).
MD042 reports empty links without inventing destinations;
see [MD042 evidence](docs/compatibility/md042.md).
MD045 reports images missing alternate text without fabricating descriptions;
see [MD045 evidence](docs/compatibility/md045.md).
MD028 reports blank gaps between sibling blockquotes without merging quotations;
see [MD028 evidence](docs/compatibility/md028.md).
MD024 reports duplicate headings without renaming content or changing anchors;
see [MD024 evidence](docs/compatibility/md024.md).
MD048 converts fence styles only when code and info strings remain unchanged;
see [MD048 evidence](docs/compatibility/md048.md).
MD027 removes incidental quote spacing only when container ownership and protected
source remain intact; see [MD027 evidence](docs/compatibility/md027.md).
MD029 normalizes later ordered-list markers while retaining rendered start values;
see [MD029 evidence](docs/compatibility/md029.md).
MD030 normalizes list-marker spacing where continuation and code ownership remain
unchanged; see [MD030 evidence](docs/compatibility/md030.md).
MD018 reports missing ATX separators without turning literal text into headings;
see [MD018 evidence](docs/compatibility/md018.md).
MD020 reports closed-ATX separator gaps without reinterpreting literal hashes;
see [MD020 evidence](docs/compatibility/md020.md).
MD011 reports reversed link syntax without creating links from literal text;
see [MD011 evidence](docs/compatibility/md011.md).
MD014 reports command-only prompt prefixes while retaining literal code;
see [MD014 evidence](docs/compatibility/md014.md).
MD056 reports table cell-count mismatches without adding or discarding cells;
see [MD056 evidence](docs/compatibility/md056.md).
MD055 reports configured table edge-pipe styles without editing rows;
see [MD055 evidence](docs/compatibility/md055.md).
MD058 adds table separators when block structure remains unchanged and reports
unsafe cases; see [MD058 evidence](docs/compatibility/md058.md).
MD001, MD025, MD036, MD041, MD043 and MD059 report heading hierarchy, title,
required-structure, emphasis-as-heading and descriptive-link findings without
inventing content; see [content diagnostic evidence](docs/compatibility/content-diagnostics.md).
MD003 conditionally converts heading styles without changing levels or inline
content; see [heading-style evidence](docs/compatibility/md003.md).
MD033 reports disallowed HTML opening tags without deleting or escaping source;
see [HTML diagnostic evidence](docs/compatibility/md033.md).
MD060 reports table-column alignment and spacing without changing table bytes;
see [table-column-style evidence](docs/compatibility/md060.md).
MD022 adds heading separators where block structure, protected source and
directive targets remain unchanged; see [separator evidence](docs/compatibility/block-separators.md).
Documents with uncertain tabbed container prefixes are preserved to avoid editing
code misclassified by the native parser; see [parser limits](docs/compatibility/parser-evaluation.md).
Other rules are not implemented yet; a successful check does not mean the document is
markdownlint-clean. See [MD013 evidence and limits](docs/compatibility/md013.md).

## Usage

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
processing failures. Warnings alone do not cause exit 1. `fix: false` prevents
edits but does not suppress configured diagnostics.

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
Implemented-rule suites exercise applicable formatting acceptance examples
and pinned upstream diagnostics. Other-rule and external-formatter examples remain future
contracts, not completed rule tests.
The parser evaluation documents a separate pinned Node development oracle; Node
is not needed to build or run mdfmt.

Project-local OpenCode configuration allows tools without approval prompts,
prefers GPT-6.1 Sol, and excludes the `git-commit` skill. Use it only in a trusted
workspace; broader OpenCode policies and agent-specific restrictions still apply.
