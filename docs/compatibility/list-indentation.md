# List indentation

MD005 reports inconsistent sibling indentation and conditionally repairs only
whitespace before a marker. The reference is markdownlint 0.42.0's `md005.mjs`.
MD007 remains pending; coverage is 51/53, not roadmap completion.

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

## Evidence

Seven native tests cover the regression-first sibling/alignment/range cases,
MD029/MD030 composition, protected-source rejection, disabled anchors/delegation,
CLI check/write behavior and a bounded 1,000-finding workload. The native oracle
checks 687 ordinary source observations, final diagnostics, upstream token
ownership/code preservation and idempotence. Its 156 upstream-fix differences
include safe unordered repairs upstream leaves to MD007 and conservative repair
rejections. No diagnostic mismatch is hidden by those differences.

The complete optimized suite passes 482 tests across 67 targets, with fmt/clippy
clean and unchanged assertions/time budgets. Prior native comparisons pass for
MD004 (296), MD027 (261), MD029 (483) and MD030 (235 plus 47 schema rejections).
The list-indentation batch's independent review follows MD007 implementation.

```sh
mise exec -- cargo test --release --all-targets --jobs 2
mise exec -- cargo build
node tools/parser-oracle/list-indentation.mjs --rule MD005
```
