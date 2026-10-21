# Parser evaluation

Milestone 2 selects `markdown` (markdown-rs) 1.0.0 behind a private adapter.
This is bounded runtime evidence, not a claim of full micromark equivalence.

## Experiment

Executed on Linux with Rust 1.99.1 and Node 24.19.0. Compared markdown 1.0.0,
pulldown-cmark 0.13.4, and comrak 0.57.0 against the same 18 inputs in
`tests/fixtures/parser/cases.json`. Enabled GFM syntax explicitly. Recorded each
candidate's positioned nodes/events in `parser-candidate-observations.json` before
removing comparison-only dependencies. The retained Rust adapter tests check
literal source slices and parent containment against independent fixture values.

The upstream oracle invokes actual markdownlint 0.42.0 with a capturing micromark
custom rule (development only), through markdownlint-cli2 0.24.0's pinned dependency
baseline. Inline directives are disabled for capture so the HTML directive fixture
does not disable the observation rule. `tools/parser-oracle/observations.json`
retains token types, source text, coordinates, and default front-matter lines.

## Findings

- markdown-rs retains complete original ranges for inline/multiline links, image
  syntax, inline code, reference definitions, tables, hard breaks, containers, and
  fenced/indented code. Offsets sliced UTF-8 correctly including non-BMP characters,
  BOM, tabs, CRLF, and lone CR in this fixture set.
- pulldown's event ranges are useful, but tight-list paragraph structure needs
  reconstruction. Reference definitions are not ordinary events; a separate API
  is needed. BOM handling differed, and literal autolinks remained text with the
  tested extension flags. Its indented-code range excluded indentation.
- comrak exposes rich structure, but its source positions need inclusive-end
  conversion and definitions were absent from ordinary descendant traversal.
- All three require separate upstream default front-matter handling. Upstream
  recognizes YAML `---`, TOML `+++` or `...`, and JSON-style `{` / `}` delimiters.
  YAML `...` is not recognized. Trailing whitespace/blank lines may be included.
  The adapter ports the fixed built-in pattern as executed by RE2JS, whose ASCII
  whitespace differs from JavaScript RegExp whitespace. It does not support custom
  regex. Four additional whitespace regressions extend the selected-parser/oracle
  suite to 22 inputs; the original three-candidate receipt remains 18 inputs.
- Upstream normalizes line endings and strips a leading BOM for parsing. The
  adapter preserves original text, maps front-matter detection back to original
  bytes, then offsets markdown-rs ranges into the untouched source.

No unsafe protected-region mismatch remained in this bounded set after adapting
front matter. Atomic/protected regions are never editable by the foundation.
Unknown constructs are conservative; headings, emphasis, hard breaks, and other
non-plain-prose nodes are protected pending their transformation designs.
Multiline text ranges are not eligible for foundation edits: those ranges can
contain source-level quote/list prefixes. Later reflow needs a container-aware
segmentation design rather than permission to replace arbitrary text-node slices.

Limitations: upstream supports additional syntax (including math/directives) not
proven equivalent here. Production proposes no edits. Later transformations must
expand oracle coverage before admitting that syntax as editable prose. Documents
containing Unicode line/paragraph separators are protected as a whole because
JavaScript multiline anchors and the Rust fixed-pattern port differ there.
Do not use this fixture set to claim complete GFM or lint compatibility.

## Reproduction

From the repository root:

```sh
mise exec -- cargo test --lib parse::
npm ci --prefix tools/parser-oracle --ignore-scripts --no-audit --no-fund
npm test --prefix tools/parser-oracle
```

Observed: 22 upstream observations reproduced; selected-parser fixture and
front-matter comparison tests passed. The original three-candidate traversal
harness was intentionally removed with unused dependencies; its raw observations
remain an evidence receipt, not a continuously rerun three-parser test.

To intentionally refresh upstream observations after reviewing fixture changes:

```sh
node tools/parser-oracle/oracle.mjs --record
```
