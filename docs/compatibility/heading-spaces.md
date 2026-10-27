# MD019 and MD021 heading separators

MD019/MD021 fixes and diagnostics are implemented against markdownlint 0.42.0's
`lib/md019-md021.mjs`. Coverage of other rules remains incomplete; see `rules.json`.

## Behavior

- MD019 applies to open ATX headings. MD021 applies to both opening and closing
  separators of closed ATX headings. Enablement and directives are independent.
- Each separator retains its first source space or tab; additional units are
  deleted. A single tab is not expanded or replaced with a space.
- Recognized setext headings, tables, code and front matter are not edited.
  Trailing whitespace after closing markers is outside these rules' repairs.
- Respects aliases/tags, severity, inline directives and API/config fix permissions.
  Disabled fixes still report configured findings.
- Empty closed headings can produce two upstream findings on the same separator;
  mdfmt retains both findings but deduplicates the actual deletion.
- Exact, rule-enabled separator edits are authorized independently of generic
  heading protection. A separator adjacent to an unrelated inline directive may
  be reduced without editing or moving that directive.
- All heading rules settle together before blank-line repairs, prose reflow and
  EOF handling. MD026 can expose closing markers and change the applicable
  spacing rule without requiring a second invocation.

## Evidence

`tests/heading_spaces.rs` checks literal outputs, per-rule permissions, protected
source, tabs, Unicode/BOM/CRLF, all-rule interaction, original edit coordinates
and stable second passes. It runs 190 recorded pinned observations, including
escaped marker endings and empty/hash-only headings. CLI tests cover
stdout/check/write and disabled-fix warning findings.

```sh
mise exec -- cargo build
node tools/parser-oracle/heading-spaces.mjs
```

The development oracle compares native output, positions, rule IDs and severity
with pinned upstream behavior. Node is not a runtime dependency. Existing parser
extension limitations remain; these tests do not establish full parser parity.
