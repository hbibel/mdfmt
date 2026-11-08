# Emphasis styles

MD049 reports configured emphasis-marker styles under markdownlint 0.42.0's
`lib/md049-md050.mjs`, with conditional paired marker changes. MD050 remains
planned; a source observation alone does not implement either rule.

## Diagnostics

- `consistent`, `asterisk` and `underscore` follow original token order. A disabled
  first token still anchors consistent style. Inline options and enabled states
  apply independently to each reported endpoint.
- Underscore requests exclude ASCII intraword markers under upstream's `\w`
  check. Unicode neighbors can still produce findings when a safe edit is absent.
- Native image alt text is flattened, so a read-only label reparse recovers its
  emphasis and consistency contribution. Images never gain edit authorization.
- Pinned micromark classifies adjacent UTF-16 units, not whole Unicode scalars.
  Underscores immediately next to an astral character such as emoji are literal;
  native AST equivalence alone can wrongly approve a swap that changes upstream
  emphasis. Exclude those native-only tokens and withhold such underscore repairs.
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
with positions removed, not whitespace-normalized content. A rejected batch of
at most 16 pairs may fall back to individual pair checks and one combined check;
larger rejected batches remain unchanged. This bounds whole-document reparse work
instead of repeating it for every marker. Marker changes precede prose reflow,
heading punctuation and heading-style stages.

## Evidence

`tests/md049.rs` covers nine focused tests and 540 pinned observations, original
byte edit application, stable second-call output/findings, multiline paired
permissions, protected regions, disabled anchors, astral-neighbor preservation
and an 800-pair CLI work regression.

The development oracle compares raw/final findings, token structure and content,
protected source, exit status and idempotence. Its low-level token snapshots use
the same BOM/newline preprocessing as upstream linting. Node is not a runtime
dependency.

```sh
mise exec -- cargo test --test md049
mise exec -- cargo build
node tools/parser-oracle/emphasis-styles.mjs --rule MD049
```
