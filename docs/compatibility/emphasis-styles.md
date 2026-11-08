# Emphasis styles

MD049/MD050 report configured emphasis/strong-marker styles under markdownlint
0.42.0's `lib/md049-md050.mjs`, with conditional paired marker changes.

## Diagnostics

- `consistent`, `asterisk` and `underscore` follow original token order. A disabled
  first token still anchors each rule's independent consistent style. Inline options and enabled states
  apply independently to each reported endpoint.
- Underscore requests exclude ASCII intraword markers under upstream's `\w`
  check. Unicode neighbors can still produce findings when a safe edit is absent.
- Native image alt text is flattened, so a read-only label reparse recovers its
  emphasis and consistency contribution. Images never gain edit authorization.
- A source-backed attention resolver follows pinned micromark's UTF-16 flanking,
  remaining delimiter runs and rule-of-three matching, independently of native
  emphasis nodes. This recovers alternate/surplus pairings and consistency anchors
  that native AST selection omits. Link labels, image labels and table cells keep
  separate inline scopes; code, destinations and HTML are not attention text.
- Pinned micromark classifies adjacent UTF-16 units, not whole Unicode scalars.
  Native AST equivalence alone can approve rendering changes beside emoji or
  ordinary letters. Repairs must retain every enclosing and adjacent attention
  pair, not just the converted node or an astral-only ancestry exception.
- Original byte spans identify each marker. Receipt columns exclude BOM as
  upstream does; native byte spans retain it. CLI presentation normalizes BOM.
- Metadata, literal code, math and HTML block ownership follow existing diagnostic
  views. Aliases, severity, disabled fixes and inline permissions apply normally.

## Repairs

Both delimiters must be enabled for the same target style and have matching native
ownership. Only their original source ranges are authorized. Links, images,
tables, code, HTML and metadata remain byte-exact even when they have findings.
Uncertain container ownership remains diagnostic-only.

Candidate edits are validated together against an exact native AST comparison
with positions removed, not whitespace-normalized content, and unchanged
source-backed attention topology. A rejected batch of
at most 16 pairs may fall back to individual pair checks and one combined check;
larger rejected batches remain unchanged. This bounds whole-document reparse work
instead of repeating it for every marker. Marker changes follow stable heading
punctuation, prefix and blank deletion, and precede prose reflow/heading styles.
MD026 can remove an intraword blocker; its approved original edits still apply.
Marker styles that would immediately manufacture another MD026 suffix violation
remain unresolved instead of rewriting the heading twice. Adjacent changes can remove
the other rule's intraword exclusion; resolve only this stage's dependencies with
at most 16 selection rounds and original-byte edit composition. If the stage does
not settle within that budget, withhold it atomically. No generic repeated-pass
formatter engine is introduced.

## Evidence

`tests/md049.rs` covers nine focused tests and 1,145 pinned observations, original
byte edit application, stable second-call output/findings, multiline paired
permissions, protected regions, disabled anchors, astral-neighbor preservation
and an 800-pair CLI work regression.

`tests/md050.rs` covers eight tests and 1,145 pinned observations, independent
styles on shared runs, astral ancestry, adjacent-rule dependencies, protected
source, endpoint permissions, CLI aliases/warnings and combined MD003/MD013/
MD026/separator behavior. The shared oracle also records 180 dual-rule cases.

`tests/emphasis_review.rs` reproduces all five independent review findings:
ordinary nested-run rendering changes in both directions, missing alternate
pairings/disabled anchors, reference-bearing image labels, nested quoted images
and MD026 scheduling/conflicts. Diagnostic image reparses include document
definitions and retain quote context through recursion; appended definitions do
not become label tokens. All remaining edits require matching native ownership.
Inline chunk boundaries are not quote or table-pipe punctuation; no-space quote
prefixes and dense table cells have independent raw-diagnostic regressions.

The development oracle compares raw/final findings, token structure and content,
protected source, exit status and idempotence. Its low-level token snapshots use
the same BOM/newline preprocessing as upstream linting. Node is not a runtime
dependency.

```sh
mise exec -- cargo test --test md049 --test md050 --test emphasis_review
mise exec -- cargo build
node tools/parser-oracle/emphasis-styles.mjs --rule MD049
node tools/parser-oracle/emphasis-styles.mjs --rule MD050
node tools/parser-oracle/emphasis-styles.mjs --combined
```
