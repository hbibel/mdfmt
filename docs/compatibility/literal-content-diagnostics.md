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

## Bare URLs

MD034 reports exposed bare HTTP(S), WWW and email links without wrapping or
rewriting them. Native literal-link ownership selects source ranges; explicit
links, autolinks and image labels are not bare URLs. Paired inline HTML content is
excluded using case-sensitive sibling matching, not a DOM or a document-wide URL
regex. Unmatched and void tags retain the pinned rule's behavior.

The content snapshot requests only bare URLs. It traverses each sibling list once
to match HTML tags, then visits exposed content; it does not reparse images or
definition payloads. Existing diagnostic-view normalization supplies math-aware
ownership and original-byte mappings through BOM, CR/CRLF, metadata, tabs and
generic HTML recovery. Escaped angle brackets are not ordinary neighboring data,
even when the native text node's range omits its escape byte.

`tests/md034.rs` has four tests consuming 1,740 pinned observations, with aliases,
permissions, warnings, CLI check/write/no-fix, literal source preservation and
second-call stability. A quoted 1,000-URL plus 256 KiB tail and 100-image budget
case completes within 30 seconds, emits exactly 1,000 warnings and retains exact
source. Full-suite verification passes 457 tests across 64 targets; fmt/clippy and
all affected HTML/reference/inline/MD013/MD026 oracles pass. Coverage is 49/53;
MD044, the batch review and the related MD014 correction remain pending.

```sh
mise exec -- cargo test --test md034
node tools/parser-oracle/literal-content.mjs --rule MD034
```

MD044 remains unimplemented. Its separate [audit](proper-name-audit.md) is
source-only evidence, not native coverage.

## Related open finding

The same native parser split still affects existing MD014 whole-block command
classification. For `>     $ echo hi\n>\n>     output\n`, pinned MD014 reports
nothing, but native MD014 reports a prompt finding at line 1 despite following
output in the same logical code block. Source bytes remain unchanged. MD046's
local grouping does not fix other snapshot consumers; the roadmap retains this
Important gap for a shared logical-code ownership correction.
