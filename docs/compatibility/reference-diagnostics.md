# Reference diagnostics

MD052 reports undefined reference labels without inventing destinations. It works
independently when no other rule is enabled. Configured shortcut handling, exact
ignored-label matching, aliases, warnings and inline directives are respected.

The read-only model derives original-byte spans and helper-compatible keys from
source-backed inline scopes, definitions and unresolved-label candidates. It does
not substitute native parser identifiers: for example, native case folding can
resolve `[ß]` against `[SS]: /url`, while the pinned helper still reports `ß`.
Definitions on disabled lines remain resolution anchors.

Undefined labels follow the pinned shim's adjacent full/collapsed/shortcut grouping
and data/line-ending projection. Escapes, entities, attention markers and code
tokens are not ordinary data. Shortcut quote prefixes are excluded, while failed
full-reference line-ending tokens can retain raw quote prefixes. Hard-break
escapes omit the backslash but retain their line ending. Image-label recovery
keeps logical definition identities and nested quote context while preserving raw
helper keys separately. Structural exclusions do not erase nested label data.
Bare GFM URL `*`/`_` bytes remain data; escapes/entities and resource destinations
do not. Explicit inner links also retain the shim's inactive outer-label ranges.

Contexts use the first physical comment-masked display line; diagnostic spans
retain valid original-byte ranges. BOM/CRLF, UTF-16 lengths, metadata, HTML, math,
tables, nested images and escaped image markers are covered. No source edit is
proposed; fixes enabled/disabled and second-call behavior remain identical.

## Evidence

`tests/md052.rs` contains four tests and consumes 2,429 paired pinned observations.
Each receipt records direct runtime and parser-forced outcomes separately. Pinned
MD052 declares `parser: "none"` but otherwise reads an empty token cache; an inert
micromark rule supplies the source-reference observations used for comparison.
Native diagnostics intentionally do not inherit that false-clean dependency.

The native development oracle checks diagnostics, exact source, exit status and
idempotence with fixes enabled/disabled. A CLI budget test covers 1,000 missing
references plus a 256 KiB trailing segment, without reparsing the document per use.
Balanced rejected outer labels are skipped rather than repeatedly projecting
nested closing brackets.

Task 1's full native suite passes 430 tests across 59 targets; fmt/clippy are
clean. Prior spacing/style diagnostic and preservation comparisons also pass.

```sh
mise exec -- cargo test --test md052
mise exec -- cargo build
node tools/parser-oracle/reference-diagnostics.mjs --rule MD052
```

## Unused and duplicate definitions

MD053 reports unused first definitions and later duplicates independently,
including footnotes. Definitions and uses on disabled lines still contribute to
identity/usage bookkeeping; only reporting permissions are filtered by line.
Ignored-definition arrays match normalized helper keys exactly, without silently
normalizing the option values.

Findings cover the original whole physical definition line, including container
prefixes. Contexts use masked display text and UTF-16 head ellipses; BOM columns
exclude the leading marker. Unused/duplicate definition content is never deleted,
including when destinations or titles differ.

`tests/md053.rs` contains five tests and consumes 2,105 paired direct/parser-forced
observations. It verifies disabled anchors/uses, quote/list and multiline records,
Unicode/comment contexts, CLI aliases/warnings, MD052/MD013/MD026 composition and
second-call stability. MD053 alone works despite the pinned parser-less cache bug.
Task 2's full suite passes 435 tests across 60 targets; fmt/clippy are clean.

```sh
mise exec -- cargo test --test md053
node tools/parser-oracle/reference-diagnostics.mjs --rule MD053
```

## Link and image styles

MD054 reports disallowed autolink, inline, full, collapsed and shortcut syntax,
plus redundant URL-inline forms. It never converts links or images. Literal
autolinks remain distinct from bare GFM URLs, and title/image exceptions apply to
`url_inline`. Native `url` validation supplies absolute-URL checks; raw source
label/destination equality and space/angle exclusions follow the pinned rule.

Style records preserve original label, destination and title ranges. Explicit
`ReferenceFields` selection prevents style-only calls from scanning undefined
uses and reference-only calls from extracting unused styles. Image-label recovery
retains definitions/container context. Multiline findings use an empty span on
the opening physical line; contexts use only the first raw line and UTF-16 head
ellipses. Aliases, severity, disabled fixes and line permissions apply normally.

The audited upstream false-clean cases have explicit native policy expectations:
actual resolved-reference kinds are reported despite differently cased/entity/
whitespace labels or empty definition destinations, and empty inline resources
remain inline syntax. These corrections use native ownership rather than the
upstream helper's raw lookup/truthiness shortcuts. MD052/MD053 retain their separate
helper-compatible key contract; no rule gains edit authority.

`tests/md054.rs` contains seven tests. Its 1,771 receipt rows distinguish 1,760
ordinary pinned comparisons from eleven explicit corrected-policy cases, retaining
both source outcomes alongside the native expectations. It covers labels/titles,
URL schemes/escapes/IDNA/IPv6, nested images, metadata/code/math/HTML/tables, BOM/
CRLF, original edit composition after reflow, all-three-rule CLI check/write/error/
warning/no-fix behavior and second-call stability. A quoted 1,000-link plus 256 KiB
tail work-budget test covers style-only extraction separately from reference-only
work. Node remains a development oracle, not a runtime dependency.
Resource-field escape scanning advances two bytes only for ASCII punctuation;
backslashes before emoji/accented/CJK characters retain valid UTF-8 boundaries.
The explicit regression reproduced a native panic before this correction.
Optimized probes for 500/1,000 quoted style-only links plus a 256 KiB tail take
2.289s/2.926s, preserve exact output, emit precisely 500/1,000 warnings and exit 0.
The pre-review full native suite passes 442 tests across 61 targets after the
UTF-8 correction; fmt/clippy and affected prior comparisons pass.

## Independent review corrections

The batch's one independent review found four Important issues and no Critical
issues. Three public-API regressions in `tests/reference_review.rs` reproduced
lost nested resource data/grouping, math-internal bracket ownership loss and
quoted multiline image-reference identity loss. They now pass independently and
in combination, with fixes enabled/disabled and unchanged output.

Owned labels use a math-aware boundary mode without changing unrelated callers.
Image recovery indexes compact logical definition headers using native identifier
normalization, selects only headers mentioned by bracket candidates and caches
only the first recovered paragraph per logical label. Raw helper keys and original
source ranges remain separate. Plain alternate text needs no recovery parse.

Two deterministic work tests count actual recovery parser calls and bytes, varying
image count, definition count, title size and repeated/distinct labels. Repeated
labels need at most one parse; eight distinct labels consume under 2 KiB even with
512 unrelated definitions. Instrumentation exists only in test builds.

Fresh optimized plain-image probes with 1 KiB definition titles preserve exact
source, status 0 and expected warning counts. For 100 images/200 definitions,
disabled/MD052/MD054 times are 5.628s/5.598s/5.820s; at 100/400 they are
11.423s/11.602s/11.461s. Added reference-analysis cost no longer scales as images
times definition payload. Pre-existing disabled-rule image latency remains a
later hardening issue, not a performance guarantee.

Post-review verification passes 447 tests across 62 targets, fmt/clippy and all
6,305 reference observations, plus prior whitespace/style/MD013/MD026 comparisons.
No second review was dispatched; coverage remains 47/53 built-ins.

```sh
mise exec -- cargo test --test md052 --test md053 --test md054
node tools/parser-oracle/reference-diagnostics.mjs --rule MD054
```
