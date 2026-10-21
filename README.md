# mdfmt

mdfmt is a Markdown formatter being built in Rust. It aims to fix safely
actionable markdownlint violations using existing declarative
[markdownlint-cli2](https://github.com/DavidAnson/markdownlint-cli2)
configuration, including prose reflow that goes beyond `markdownlint-cli2 --fix`.

## Status

The roadmap and initial compatibility audit are documented. There is no formatter
implementation, executable, or installation command yet.

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

## Development

The Rust toolchain is declared in `mise.toml`. There is no Cargo project or
application test suite yet. Audit verification instructions distinguish source
checks from runtime behavior that has not been tested.

Project-local OpenCode configuration allows tools without approval prompts,
prefers GPT-6.1 Sol, and excludes the `git-commit` skill. Use it only in a trusted
workspace; broader OpenCode policies and agent-specific restrictions still apply.
