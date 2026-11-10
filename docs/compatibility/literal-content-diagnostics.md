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
retains 1,518 pinned observations covering explicit/consistent styles, all newline
forms, BOM, tabs, nested containers, metadata, HTML recovery, empty/unclosed fences,
disabled anchors and severity. The native oracle checks exact source, findings,
exit status and repeated-call behavior with fixes enabled/disabled.

The first five tests failed on absent diagnostics. The first implementation then
passed four tests but exposed split indented-code records; the literal same-owner
regression failed before grouping and passes afterward, together with all receipt
rows. At MD046's initial commit the full suite passed 453 tests across 63 targets,
with fmt/clippy and affected code/MD013/MD026 oracles clean, reaching 48/53 rules.
Subsequent batch and final verification evidence appears below.

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
source. At MD034's commit the full suite passed 457 tests across 64 targets, with
fmt/clippy and affected HTML/reference/inline/MD013/MD026 oracles clean, reaching
49/53 rules. Subsequent MD044 and logical-code corrections are recorded below.

```sh
mise exec -- cargo test --test md034
node tools/parser-oracle/literal-content.mjs --rule MD034
```

## Proper names

MD044 reports configured proper-name capitalization without changing prose,
identifiers, code, HTML, links or definitions. Empty name entries are ignored;
nonempty entries still run. `code_blocks` and `html_elements` apply independently.
Converted Markdown inside HTML flow retains its Data/Code classification rather
than inheriting blanket HTML permissions. Continuation prefixes are not name data.

Source segments retain original byte ranges for Data, Code and Html, omitting
escapes/entities, attention markers, resources, reference labels, metadata and
math. Visible labels and recovered image descriptions remain checkable. Image
recovery shares the reference batch's selected compact-definition headers and
paragraph cache; it does not parse unrelated definition payloads per name/image.

The existing vendored `regress` engine matches escaped literal names using legacy
JavaScript UTF-16 case/boundary semantics. Longest names have priority; ICU root
collation resolves equal-length priorities as in the pinned runtime. A sigma
variant regression reproduced the failure of scalar/lowercase heuristics before
adding native `icu_collator` 2.1.1. Other environment-selected collation locales
are not claimed as additional reference contracts.

Accepted spellings and previously matched ranges prevent overlapping duplicate
findings. Actual URL exclusions are computed once per relevant source segment,
not extended to the token's end. Code indentation is removed only from the
diagnostic segment, with physical values mapped back to original source bytes.
Names after URLs remain reportable. No Node dependency exists at runtime.

`tests/md044.rs` has ten native tests. Its 1,911 receipts distinguish 1,904 ordinary
pinned cases from seven corrected-policy cases for URL bounds/empty names. Empty
names are never sent to upstream; the separate [audit](proper-name-audit.md)
retains its bounded resource-failure observation. Tests cover source ranges,
Unicode priority, flags, aliases, CLI modes, MD013/MD026 composition, repeated
calls and a bounded 1,100-finding dense-name/image workload with unrelated titles.

Development also reproduced a shared normalization defect: a synthetic space for
the BOM changed list continuation depth and exposed indented code as prose.
Zero-width BOM mapping fixes both false MD034 findings and incorrect MD044 code
flag handling. The literal regression and prior HTML/bare-URL comparisons pass.

All focused tests/native oracles pass with fmt/clippy clean. The isolated full
suite passes 467 tests across 65 targets. An earlier concurrent run hit MD018's
existing five-second math budget at 5.77s; the exact test and full suite pass
without competing oracle jobs. The limit was not weakened. Coverage is 50/53;
the independent batch review remains pending. The subsequent MD014 correction is
verified below.

```sh
mise exec -- cargo test --test md044
node tools/parser-oracle/proper-names.mjs
```

## Logical code correction

The same native parser split exposed an Important MD014 whole-block error:
`>     $ echo hi\n>\n>     output\n` reported a prompt despite following output.
The literal reproduction failed before the correction and now passes. MD014 and
MD046 share logical diagnostic code snapshots, joining only same-owner indented
nodes across actual blank/prefix-only gaps. Separate containers, adjacent parser
tokens and intervening empty containers remain independent; values retain actual
physical line distances. Original code bytes remain unchanged. The MD014 corpus
grows to 462 cases and MD046 to 1,518. The optimized full suite passes 471 tests
across 65 targets, with fmt/clippy clean and unchanged assertions/time budgets.
The prior debug run passed 470 tests before the additional barrier regression;
the final debug attempt exceeded its command limit after 387 passing tests, with
no failures. The complete optimized run is the final full-suite evidence, not a
claim that the timed-out debug run finished. The Important MD014 finding is fixed;
one independent batch review remains pending.
