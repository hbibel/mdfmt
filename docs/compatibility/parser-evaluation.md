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
non-plain-prose nodes remain protected. The strict foundation edit path still
rejects multiline text ranges because they can contain quote/list prefixes. The
reflow path now authorizes only explicitly modeled whitespace/container gaps and
requires semantic-tree/protected-source equivalence before expanded edits apply.

Limitations: upstream supports additional syntax (including math/directives) not
proven equivalent here. Initial reflow proposes only whitespace edits inside
paragraphs, including multiline/container text, subject to the guard described below. Later
transformations must expand oracle coverage before admitting more syntax. Documents
containing Unicode line/paragraph separators are protected as a whole because
JavaScript multiline anchors and the Rust fixed-pattern port differ there.
Do not use this fixture set to claim complete GFM or lint compatibility.

## Initial reflow evidence

MD013 retains the foundation edit guard and adds exact modeled whitespace gaps
for soft joins and structural continuation prefixes. Links, images, inline code,
and emphasis remain indivisible source units. After packing, a whole-document
reparse must retain a position-independent mdast tree, normalized text, and exact
protected source. A mismatch discards the wrapping batch. This is a conservative
guard against introducing headings, lists, HTML blocks, or other syntax; it is
not proof of parser parity. See [current MD013 evidence](md013.md).

`tests/fixtures/reflow.json` contains 27 input/output cases checked by the Rust
suite and a development-only pinned upstream oracle. The oracle executes the CLI,
checks second-pass idempotence, compares selected micromark block/inline structure,
and asserts upstream MD013 line findings before and after formatting. It covers
UTF-16 widths, inline units, CRLF/BOM, reference syntax, structural hazards, and
strict versus default lint exceptions, math/directive hazards, and raw HTML.
Runtime comparisons passed locally on
2026-10-27. Long links can remain lint errors under strict MD013; reflow does not
convert or split them. Native remaining-violation reporting is now implemented
and compared against the pinned rule, separately from the reflow policy.

Reproduce after installing the pinned development oracle dependencies:

```sh
mise exec -- cargo build
npm run test:reflow --prefix tools/parser-oracle
npm run test:reflow-scaling --prefix tools/parser-oracle
```

Because native GFM cannot validate upstream extensions, the formatter currently
preserves an entire document if any paragraph contains `$` or a colon immediately
followed by an ASCII letter. It also preserves documents containing non-comment
HTML, including inline tags whose content may be whitespace-sensitive. These
deliberately broad guards can withhold wrapping for ordinary prose too; precise
extension and HTML boundaries remain future work.

Permission checks reuse source coordinates and indexed text/protected intervals.
The development scaling smoke test checks exact output for 100, 500, and 1,000
paragraphs with a generous per-process time budget. The local debug CLI took about
0.06, 0.34, and 0.8 seconds respectively after replacing whole-document scans per
edit; this is bounded performance evidence, not a release benchmark.

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
