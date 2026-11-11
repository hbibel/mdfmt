# Link fragments

MD051 reports missing or incorrectly capitalized local fragment targets using
the pinned markdownlint 0.42.0 rule. This first implementation is diagnostic-only;
the narrow safe ASCII case-correction task and independent review remain pending.

## Target and destination identity

Heading targets follow the rule's source projection: visible data, escapes, inline
code and math contribute; image descriptions, HTML tokens, entities and resource/
reference syntax do not. Footnote-call labels do contribute. Lowercasing, Unicode
Letter/Mark/Number/Connector_Punctuation filtering, ASCII spaces/hyphens and URI
component encoding follow the pinned transformations. Duplicate counters, custom
`{#anchor}` strings, `#top` and explicit HTML id/a-name precedence retain insertion
order. Disabled heading lines still supply targets.

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

## Evidence and scope

Eight public tests cover literal target/range cases, 664 ordinary pinned source
observations, CLI preservation and bounded dense/repeated-image workloads with
exact finding counts. The complete optimized suite passes 512 tests across 71
targets, with fmt/clippy clean. Prior native comparisons pass for MD052/MD053/MD054
(6,305 total), MD024 (224), MD033 (602), MD045 (207), MD013 (288) and MD026 (440).

All 53 built-ins now have native coverage, including conditional and diagnostic-only
rules. `Coverage.complete` remains false: that count does not establish all safe
repairs, all option/parser parity or roadmap completion. MD051 safe case repairs,
the batch review, general conflicts, external formatters, later declarative
compatibility and release hardening remain explicit work.

```sh
mise exec -- cargo test --release --all-targets --jobs 2
mise exec -- cargo build
node tools/parser-oracle/link-fragments.mjs
```
