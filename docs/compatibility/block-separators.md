# Block separators

MD022 reports missing blank lines around headings under markdownlint 0.42.0's
`lib/md022.mjs`, adding them only when reparsing preserves meaning. MD031 and
MD032 are planned, not implemented by this evidence.

## MD022 behavior

- Scalar and per-level `lines_above`/`lines_below` options are supported. Omitted
  array entries default to 1; `-1` disables checking on that side, and 0 requires
  no minimum blanks. Extra existing blanks are MD012's concern.
- ATX, closed-ATX and multiline setext headings use original physical boundaries.
  File boundaries supply virtual blanks; `include_front_matter` controls whether
  metadata lines count as visible neighbors above the first heading.
- Comment-only and quote-prefix-only neighbors count as blank under upstream's
  helper. Contexts use comment-masked lines, without MD022-specific truncation;
  original byte ranges never come from the masked UTF-8 view.
- Aliases, severity, inline permissions and disabled fixes apply normally.
  Remaining findings use empty spans at final-output line starts, excluding BOM.

## Conditional repairs

Insertions use the document's preferred newline and a verified continuation
prefix. Compatible requests sharing a blank run merge by the largest missing
count rather than adding duplicate padding. Each insertion and the combined
batch must preserve the native tree and exact protected source.

Root and quote-contained headings can gain separators without changing their
captions. List-tightness changes, ambiguous recovered ownership, metadata-region
changes, code/table edits and uncertain syntax are preserved with findings.
An unsafe combined batch may withhold otherwise safe individual insertions.

Insertions must not retarget any physical next-line override, even for another
rule. Permission checks apply at both the proposed boundary and a shared run's
canonical boundary. An enabled MD012 maximum limits interior plain blank runs;
unresolved MD022 requirements remain findings instead of spacing ping-pong.

Virtual-boundary counting is arithmetic, not proportional to a configured count.
Separator insertions are capped at 1 MiB per stage, checking the whole batch
before allocating replacements. Oversized requirements remain findings; the
formatter does not attempt unbounded output or gradually fill an oversized batch.

## Evidence

`tests/md022.rs` covers 657 pinned observations and literal safe repairs, quote
prefixes, arrays, metadata, CRLF, directive targets, MD012 conflicts, shared gaps
and idempotence. Native-only process regressions bound huge-count checking and,
on Linux, verify allocation safety under a 128 MiB virtual-memory limit. These
resource tests do not claim parity with running huge counts upstream.

```sh
mise exec -- cargo test --test md022
mise exec -- cargo build
node tools/parser-oracle/block-separators.mjs --rule MD022
```

The oracle checks raw findings, final-output findings, retained nonblank lines
and second-call stability. It does not require unsafe upstream fixes. Node is
development-only; overall built-in rule coverage remains incomplete.
