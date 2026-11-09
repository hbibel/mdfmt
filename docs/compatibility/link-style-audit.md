# Link-style audit observations

These are pinned markdownlint observations for planned MD054 work, not native
implementation or coverage. The reference remains markdownlint 0.42.0.

## False-clean reference styles

`lib/md054.mjs` looks up raw reference-string/label text in the shared definition
map, whose keys the helper lowercases and whitespace-normalizes. With
`MD054.full: false`, `[text][id]` against `[id]: /url` reports a finding, but
`[text][ID]` does not. Uppercase collapsed/shortcut labels and entity-bearing
labels likewise miss disallowed-style findings despite resolving as links.

The source also distinguishes inline resources by a truthy destination string.
`[text]()` is falsely clean under `MD054.inline: false` even if `[text]: /url`
exists; the rule falls into its reference branch instead of classifying the empty
inline resource. Future native classification must decide from syntax, not
destination truthiness.

## Upstream edits can discard content

Under `MD054.inline: false`, upstream proposes converting
`[label](https://example.com "title")` to `<https://example.com>`, discarding both
the visible label and the title. Do not reuse upstream fix proposals as native
edit authority. Preserve inline links and their content; report styles when no
meaning-preserving allowed conversion exists.

## Evidence

`tools/parser-oracle/link-style-audit.mjs` and its tracked receipt retain nine
source-only observations, including baseline lowercase findings and emitted fix
text. These are runtime observations backed by source review, not application
tests or coverage claims.

```sh
node tools/parser-oracle/link-style-audit.mjs
```
