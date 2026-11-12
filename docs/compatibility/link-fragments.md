# Link fragments

MD051 reports missing or incorrectly capitalized local fragment targets using
the pinned markdownlint 0.42.0 rule. It conditionally repairs unambiguous ASCII
inline-destination capitalization. The independent batch review and its single
regression-first fix pass are complete.

## Target and destination identity

Heading targets follow the rule's source projection: visible data, escapes, inline
code and math contribute; image descriptions, HTML tokens, entities and resource/
reference syntax do not. Footnote-call labels do contribute. Lowercasing, Unicode
Letter/Mark/Number/Connector_Punctuation filtering, ASCII spaces/hyphens and URI
component encoding follow the pinned transformations. Duplicate counters, custom
`{#anchor}` strings, `#top` and explicit HTML id/a-name precedence retain insertion
order. Disabled heading lines still supply targets.
Automatic-link payloads do not contribute caption data; explicit Markdown labels
do. Lexical ATX caption boundaries preserve literal hashes and significant spaces
that native child spans can omit. Multiline code/math drops line endings and actual
container prefixes, not following data whitespace or partial/lazy-prefix content.

Inline links and definition destinations use existing owned resource ranges,
including links recovered inside images. Images themselves are not links. The new
definition-destination field is requested only by this consumer; ordinary reference
checks do not extract unused fields. Resource scanning is shared rather than
duplicated. Metadata, code and nonlocal URLs do not create local fragment findings.

Encoded exact identity and raw case-insensitive identity remain separate, matching
the pinned source rather than blindly decoding URLs. `ignore_case` suppresses only
the pinned case-only finding, not missing targets. Nonempty `ignored_pattern`
retains the explicit unsupported-setting error until later JavaScript-regex work.

Diagnostics cover single-line links/definitions and retain exact source context.
Multiline owners have no context and use a zero-width span at their physical line
start. Original byte ranges and UTF-16 columns survive BOM, CRLF/CR, tabs, quoted
definitions and recovered scopes. Table links are diagnosed without editing tables.
Aliases, warnings, inline permissions and both fix modes are tested.

## Bounded analysis

Exact targets and first case-folded targets use indexed lookups. Docfx heading
ranges use binary lookup, not a full target scan per link. Recognized-entity
validation caches small native parses rather than reparsing documents per link.

The new consumer exposed a shared HTML-image path that reparsed all unrelated
definition payloads for every image. A deterministic real-parser regression failed
with four parses for four equal images. HTML recovery now uses the existing
compact selected-definition/paragraph cache; the test requires exactly one parse
and fewer than 4,096 input bytes while varying images and unrelated definitions.
Existing reference identity, HTML ownership and protection contracts remain intact.

## Safe case corrections

A repair requires exactly one case-folded target, raw `#[A-Za-z0-9_-]+` syntax in
both source and target, a single physical line, original native-link ownership and
MD051 edit permission. Definitions, tables, code, images, HTML-owned/recovered-only
links, escaped/entity/percent/non-ASCII destinations and ambiguous case collisions
remain diagnostic. `ignore_case` suppresses both the case-only finding and repair.

Repairs run after heading normalization, so MD026/MD003 and MD013 composition uses
current targets and source ranges. The private validator compares position-free
original/candidate trees with only admitted ASCII fragment URL case normalized;
every other AST field remains exact. It does not relax generic protected-link
validation. Titles, labels, nonlocal URLs and all bytes outside selected inline
destinations remain unchanged. Repeated formatting settles without another edit.
Already-valid line and line-range fragments remain exact even when a heading or
HTML target collides by case; diagnosis and repair use the same validity exemption.

## Evidence and scope

Thirteen original tests and five review regressions cover literal target/range
cases, 1,144 ordinary pinned source
observations, narrow repair/rejection/permission/composition cases, CLI controls and
bounded dense/repeated-image workloads with exact finding counts. The native oracle
uses independent upstream destination spans to admit every changed code unit and
checks final diagnostics/idempotence. Its 30 upstream-fix differences retain
ambiguous/protected cases rather than guessing targets or editing tables/definitions.
The complete optimized suite passes 522 tests across 72 targets, with fmt/clippy
clean. Prior native comparisons pass for MD052/MD053/MD054
(6,305 total), MD024 (224), MD033 (602), MD045 (207), MD013 (288) and MD026 (440).
MD003's 1,206 comparisons and 29 reflow outputs also pass after repair integration.
The post-review source-prefix gates also pass 516 MD014 and 1,662 MD046 comparisons.

The single independent review found four Important findings, no Critical findings
and no minors. Five primary public tests failed before correction, covering line
navigation, automatic links, lossy ATX captions, multiline literal spaces and CLI
byte preservation. Expanded lazy-prefix controls also failed before the partial
prefix correction. All now pass across LF/BOM+CRLF/CR and root/list/quote contexts.
Native edit admission additionally requires an original upstream wrong-case
finding, not merely a case-folded inline span; already-valid destinations cannot
pass that preservation check. No second review was dispatched.

All 53 built-ins now have native coverage, including conditional and diagnostic-only
rules. `Coverage.complete` remains false: that count does not establish all safe
repairs, all option/parser parity or roadmap completion. General conflicts,
external formatters, later declarative
compatibility and release hardening remain explicit work.

```sh
mise exec -- cargo test --release --all-targets --jobs 2
mise exec -- cargo build
node tools/parser-oracle/link-fragments.mjs
```
