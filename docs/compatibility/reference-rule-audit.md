# Reference-rule audit observations

These are pinned upstream observations for MD052/MD053, not native implementation
tests. MD052's separate [native evidence](reference-diagnostics.md) now exists;
MD053 remains planned. The reference remains markdownlint 0.42.0.

## Parser-less cache dependency

Both rules declare `parser: "none"`, but consume the shared micromark-token cache.
With `default: false` and only MD052/MD053 enabled, the pinned runtime leaves that
cache empty and reports no findings for any of these inputs:

- `[text][missing]` with `MD052.shortcut_syntax: true`.
- `[unused]: url` with MD053 enabled.
- Two identical `[a]: one` definitions with MD053 enabled.

Enabling an otherwise inert custom rule with `parser: "micromark"` produces the
expected missing-reference, unused-definition and duplicate-definition findings.
The comparison used the same inputs and configuration apart from that no-op rule.
Keep direct runtime results distinct from parser-forced rule observations in the
future oracle. Native rules must not depend on another rule being enabled merely
to obtain their diagnostic source model.

## Label boundaries

Source review of `helpers/helpers.cjs::getReferenceLinkImageData` and forced-parser
runtime observations establish the following cases for the planned native model:

- Labels are lowercased, trimmed and collapsed using JavaScript whitespace rules;
  ignored-label/definition arrays are not independently normalized by the helper.
- Undefined-reference shims retain only data and line-ending token text.
  `[text][m\*issing]` and `[text][m&amp;issing]` both report the label `missing`,
  not the raw escaped/entity spelling or the rendered `m*issing`/`m&issing`.
- Undefined quoted multiline `[text][a` / `> b]` reports label `a > b`. This differs
  from the helper's explicit quote-prefix exclusion for resolved reference tokens.
  Undefined shortcuts such as `[a` / `> b]` instead report `a b`: failed full-label
  parsing includes prefixes in its line-ending tokens, while shortcut data uses
  separate prefix tokens. Hard-break escapes omit their backslash from the key.
- `[ß]` with `[SS]: url` produces a missing-reference finding and an unused `ss`
  definition under the forced-parser rule observation. Do not substitute the
  native parser's reference identity normalization for this helper's lowercase key.
- Footnote definitions/calls contribute `^`-prefixed keys to MD053's shared model.
- Code and HTML-block contents remain excluded. MD053's context and range cover
  the physical definition line, including list/quote prefixes.

The initial 24-input, parser-off/parser-on probe and its 48 observations are
retained at `/tmp/opencode/mdfmt-reference-rule-audit.mjs` and
`/tmp/opencode/mdfmt-reference-rule-audit.jsonl` for the implementation handoff.
