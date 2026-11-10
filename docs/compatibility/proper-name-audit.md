# Proper-name audit observations

These are pinned markdownlint observations, not native implementation tests.
The separate [native evidence](literal-content-diagnostics.md) records MD044's
diagnostic-only implementation and explicit policy corrections. The reference
remains markdownlint 0.42.0.

## URL exclusions hide following names

With `names: ["GitHub"]`, plain `github` reports a capitalization finding.
However, inline/fenced code containing `https://example.com github` reports none,
despite the default `code_blocks: true`. In inline code containing
`github https://example.com github`, only the first name is reported.

`lib/md044.mjs` computes an autolink exclusion's end from the enclosing token's
end column rather than its start column, extending the exclusion past the URL.
Native diagnostics now select actual URL ranges rather than hide later names.
This does not authorize modifying literal code or identifiers.

## Empty names can fail to terminate

The schema permits empty strings in `names`. With `names: [""]`, the generated
global pattern can match zero characters; MD044's repeated `RegExp.exec` never
advances `lastIndex`. A bounded child-process probe on plain prose hit its timeout/
heap-abort guard, while the normal-name control exited successfully. The native
rule ignores empty entries before scanning and continues checking nonempty names.

`proper-name-unsafe-observation.json` retains the single observed guard outcome.
Do not replay this input in an unbounded development oracle or application test.

## Evidence

`tools/parser-oracle/proper-name-audit.mjs` retains six ordinary source-only
observations, including inline HTML as a working comparison. It does not rerun
the unsafe empty-name probe. Native results are recorded separately.

```sh
node tools/parser-oracle/proper-name-audit.mjs
```
