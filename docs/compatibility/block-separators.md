# Block separators

MD022 reports missing blank lines around headings under markdownlint 0.42.0's
`lib/md022.mjs`, adding them only when reparsing preserves meaning. MD031 adds
conditional fence separators under `lib/md031.mjs`. MD032 adds conditional list
separators under `lib/md032.mjs`.

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

## MD031 behavior

- Opening and ending physical fence lines are checked separately. Both fence
  styles, longer closing markers, unclosed fences and empty blocks follow the
  pinned parser's boundaries. Indented code, math and HTML-owned fences are not
  promoted into eligible fenced blocks.
- `list_items: false` excludes list-contained fences, including quoted/nested
  lists. Recovered code snapshots retain that ownership independently of edit
  authorization, covering fences after recovered HTML boundaries.
- Comments, metadata, BOM and mixed line endings retain the existing source
  model. Contexts are comment-masked and truncated under upstream's MD031 helper,
  unlike MD022's untruncated contexts.
- Code bytes and language/info strings remain exact. Adding separators must not
  loosen a list or retarget an active next-line directive for another rule.
- Fence-style repairs and prose reflow run before this separator stage; remaining
  diagnostics refer to their final output. Aliases, severity and fix controls apply.

## MD032 behavior

- Only outermost lists are checked, including those inside quotes. Nested child
  lists do not independently require separators. Ordered/unordered markers,
  empty items, references and definitions retain their source meanings.
- The visual end follows the final content block rather than a container's last
  prefix-only line. Inline closing markers and literal code remain content;
  trailing quote prefixes do not create false boundary findings.
- Tab expansion and invalid-interruption correction determine diagnostic list
  ownership, mapping byte positions back to original source.
- Lazy generic HTML is modeled as a block inside surviving containers, not an
  endlessly continued paragraph. Synthetic boundaries map back to the original
  tag line without inventing edit ranges. Later text, closing tags and markers follow pinned
  scope; contexts and all edits still use original source.
- HTML-owned lists, code and math are excluded where upstream excludes them.
  Aliases, warnings, disabled fixes and physical next-line targets are respected.

## Conditional repairs

Insertions use the document's preferred newline and a verified continuation
prefix. Compatible requests sharing a blank run merge by the largest missing
count rather than adding duplicate padding. Each insertion and the combined
batch must preserve the native tree and exact protected source.

Root and quote-contained headings/fences/lists can gain separators without changing
captions or code. List-tightness changes, ambiguous recovered ownership, metadata-region
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

`tests/md031.rs` covers 264 pinned observations, root repairs, list exclusions,
code preservation, active directive targets, MD048 interactions and final warning
positions after MD013 reflow.

`tests/md032.rs` covers 345 pinned observations, outer-list fixes, visual ends,
lazy HTML scope, original/final coordinates, active directive targets and CLI
warning-only outcomes without writes.

```sh
mise exec -- cargo test --test md022 --test md031 --test md032
mise exec -- cargo build
node tools/parser-oracle/block-separators.mjs --rule MD022
node tools/parser-oracle/block-separators.mjs --rule MD031
node tools/parser-oracle/block-separators.mjs --rule MD032
```

The oracle checks raw findings, final-output findings, retained nonblank lines
and second-call stability. It does not require unsafe upstream fixes. Node is
development-only; overall built-in rule coverage remains incomplete.
