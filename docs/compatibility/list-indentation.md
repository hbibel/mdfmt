# List indentation

MD005/MD007 report list indentation and conditionally repair only whitespace
before a marker. References are markdownlint 0.42.0's `md005.mjs` and `md007.mjs`.
Coverage is 52/53, not roadmap completion.

## MD005

The first sibling determines expected indentation even when reporting on that
line is disabled. Ordered lists accept aligned marker ends across digit-width
changes; after entering end-alignment mode, later markers must retain that end.
Diagnostic spans cover the physical line prefix, using original bytes and UTF-16
columns, including BOM, tabs, quotes, nested lists and recovered generic HTML.

Repairs insert or remove spaces immediately before the marker. They never change
numbers, delimiters, padding, links, continuation lines, code or tables. Each batch
must pass original-parser semantic/protected-source equivalence; diagnostic
recovery does not grant edit permission. Structural tabs or unmodeled syntax
withhold edits. Unsafe cases retain final-output findings.

For unordered lists MD005 delegates repairs whenever any sibling prefix enables
MD007, including partial inline permissions. This prevents a first-item sibling
anchor from competing with configured absolute nesting indentation. Diagnostics
remain independent. Padding and numbering settle before sibling indentation.
Aliases, severity, rule enablement, inline directives and both fix controls apply.

## MD007

Unordered nesting counts reset at a quote; ordered ancestry disables checking
until a nearer quote resets the scope. Footnotes add their pinned base indentation.
Physical quote-prefix adjustments exclude bracketed labels from marker scanning.
All three options apply, including `start_indent` defaulting to `indent`.
Expected arithmetic uses wider integers than configuration values, so huge
targets retain exact diagnostic details without overflow or padding allocation.
Structural changes, continuation/code rewrites and unsafe batches remain withheld.

The batch exposed a nontermination conflict: configured start indentation after a
quote can require spaces that MD027 removes. MD005 can create the same cycle when
its first sibling disables MD027 and later siblings do not. Bounded regressions
failed before the shared candidate gate rejected insertions overlapping enabled
MD027 findings. MD027 retains priority; final MD005/MD007 findings remain stable.
Disabled MD027 and `list_items: false` still permit safe indentation. This is a
narrow convergence policy, not general conflict reporting.

Interaction tests also exposed pre-existing MD013 prefix growth before a link/code
span on a continuation line. A native text token can end at the newline, excluding
existing indentation before the protected inline span. The reflow gap now consumes
prefix bytes within the enclosing paragraph; literal tokens remain unchanged.
Direct MD013-only regressions and list/heading/numbering/reflow composition pass.

## Evidence

Eight MD005 tests cover the regression-first sibling/alignment/range cases,
MD029/MD030 composition, protected-source rejection, disabled anchors/delegation,
CLI check/write behavior and a bounded 1,000-finding workload. The native oracle
checks 687 ordinary source observations, final diagnostics, upstream token
ownership/code preservation and idempotence. Its 156 upstream-fix differences
include safe unordered repairs upstream leaves to MD007 and conservative repair
rejections. No diagnostic mismatch is hidden by those differences.

Twelve MD007 tests cover options/scopes/ranges, five-second huge-target/conflict
termination, partial permissions, CLI controls, composition and a bounded
1,049-finding wide/deep workload. Its 1,680 ordinary source observations pass
native diagnostics, upstream ownership/value preservation and idempotence, with
261 explicitly counted upstream-fix differences. Neither rule uses Node at runtime.

The complete optimized suite passes 496 tests across 69 targets, with fmt/clippy
clean and unchanged assertions/time budgets. Prior native comparisons pass for
MD004 (296), MD027 (261), MD029 (483) and MD030 (235 plus 47 schema rejections).
All 288 MD013/440 MD026 and 29 reflow comparisons pass. The scaling harness now
isolates MD013 rather than treating later default MD041 findings as failures.
Its debug 500-paragraph run exceeded the unchanged ten-second limit; optimized
100/500/1,000-paragraph runs pass at 211/1,117/2,509 ms. No completed debug scaling
run is claimed. The list-indentation batch's independent review remains pending.

```sh
mise exec -- cargo test --release --all-targets --jobs 2
mise exec -- cargo build
node tools/parser-oracle/list-indentation.mjs --rule MD005
node tools/parser-oracle/list-indentation.mjs --rule MD007
node tools/parser-oracle/reflow-scaling.mjs --release
```
