# Literal content diagnostics

MD046 reports code-block style mismatches without converting fences, indentation,
info strings or code. Explicit `fenced`/`indented` styles and the first-block
`consistent` anchor follow the pinned rule; disabled first blocks still anchor
classification. Inline permissions control findings, not ownership.

Diagnostics use an empty span on the opening physical line, retaining valid BOM,
CRLF and original-byte positions. Aliases, warnings, CLI check/write, disabled
fixes and final-output MD013/MD026 composition are covered. Tables, metadata,
literal code and links remain unchanged.

Tab-expanded diagnostic ownership determines style but never supplies output
bytes. Indented code records split by the native parser across blank container
lines are treated as one logical block only when their owner is identical and
the gap contains only whitespace/container prefixes. Different list items or
blockquotes remain separate.

## Evidence

`tests/md046.rs` contains six native tests; `literal-content.mjs --rule MD046`
retains 1,464 pinned observations covering explicit/consistent styles, all newline
forms, BOM, tabs, nested containers, metadata, HTML recovery, empty/unclosed fences,
disabled anchors and severity. The native oracle checks exact source, findings,
exit status and repeated-call behavior with fixes enabled/disabled.

The first five tests failed on absent diagnostics. The first implementation then
passed four tests but exposed split indented-code records; the literal same-owner
regression failed before grouping and passes afterward, together with all receipt
rows. Full-suite verification passes 453 tests across 63 targets; fmt/clippy and
the affected MD014/MD040/MD048/MD013/MD026 native oracles pass. Coverage is 48/53;
the remaining batch tasks and its independent review are still pending.

```sh
mise exec -- cargo test --test md046
node tools/parser-oracle/literal-content.mjs --rule MD046
```

MD034 and MD044 remain unimplemented. The separate
[MD044 audit](proper-name-audit.md) is source-only evidence, not native coverage.

## Related open finding

The same native parser split still affects existing MD014 whole-block command
classification. For `>     $ echo hi\n>\n>     output\n`, pinned MD014 reports
nothing, but native MD014 reports a prompt finding at line 1 despite following
output in the same logical code block. Source bytes remain unchanged. MD046's
local grouping does not fix other snapshot consumers; the roadmap retains this
Important gap for a shared logical-code ownership correction.
