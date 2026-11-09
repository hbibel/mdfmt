# Inline whitespace diagnostics

MD037 reports spaces beside paired bare emphasis-marker data tokens under pinned
markdownlint's `lib/md037.mjs`. It does not create emphasis from literal prose.

The shared read-only inline snapshot supplies attention pairs and bare marker
tokens separately from edit authorization. Pairing uses identical one-to-three
character runs within each direct token parent. Root paragraph residual markers
stay separate; matched attention and link/image labels coalesce adjacent data.
Code, escapes, character references and structural prefixes are non-data boundaries.

MD037 contexts use comment-masked display lines; findings retain original byte
ranges. Contexts selected across an astral character follow upstream UTF-16
selection, replacing an isolated surrogate for UTF-8 display. BOM, CRLF, aliases,
warnings and inline permissions are covered.

`tests/md037.rs` contains five tests and consumes 915 pinned observations. The
development oracle checks native findings with fixes enabled/disabled, unchanged
source, exit status and idempotence. The shared-model move also passes the 1,145
MD049, 1,145 MD050 and 180 combined style comparisons.

```sh
mise exec -- cargo test --test md037 --test md049 --test md050 --test emphasis_review
mise exec -- cargo build
node tools/parser-oracle/inline-whitespace.mjs --rule MD037
```

## Code spans

MD038 reports `lib/md038.mjs`'s extra code-data whitespace, balanced syntax
padding and backtick exceptions. It never trims code. Code records exclude
mandatory container prefixes but retain additional indentation as literal data.
Contexts use raw source, newline normalization and UTF-16 head/tail ellipses.

`tests/md038.rs` contains five tests and consumes 1,203 pinned observations. It
also verifies quoted multiline positions, reflow/marker composition and valid
data-space ranges where upstream throws. `md038-invalid-ranges.json` separately
retains three upstream exceptions and raw rule outputs with negative columns;
these are not counted as matching diagnostic observations. Native diagnostics
on those inputs select real horizontal spaces rather than another line's padding.

```sh
mise exec -- cargo test --test md038
node tools/parser-oracle/inline-whitespace.mjs --rule MD038
```

## Link labels

MD039 reports `lib/md039.mjs`'s leading/trailing link-label whitespace without
trimming indivisible links. Actual link labels, including links recovered inside
image descriptions, qualify; image-only alternate-text labels do not. Raw label
contexts retain container prefixes, collapse JS whitespace and use endpoint
ellipses. Newline-only endpoints have empty byte spans anchored to the adjacent
physical content line rather than invented horizontal ranges.

`tests/md039.rs` contains seven tests and consumes 957 successful pinned
observations. It covers original-coordinate edit composition after MD013 reflow,
combined heading punctuation/style/separator edits, preserved link/code bytes,
aliases, warnings, check/write modes, disabled fixes and second-call stability.

Upstream mixes Unicode-aware JS trimming with ASCII-only RE2JS range matching.
The separate `md039-invalid-lines.json` receipt records 42 exceptions over 21
inputs; `md039-unicode-policy.json` records six one-sided outcomes, including
findings incorrectly moved to the next blank line. Native diagnostics instead
select valid horizontal spaces on the actual label line. Literal tests cover
NBSP, U+2003 and BOM. These intentional differences are not counted as ordinary
matching observations.

```sh
mise exec -- cargo test --test md039
node tools/parser-oracle/inline-whitespace.mjs --rule MD039
```

## Independent review regressions

`tests/inline_whitespace_review.rs` reproduces two Important findings before
their fixes: code padding extended across physical lines (also consulting the
wrong line's permissions), and repeated whole-line prefix expansion for each
code span. Padding now extends a range only when adjacent to the selected data.
The tests cover both directions, CRLF, quotes/lists and disabled opening lines.

The shared snapshot accepts an `InlineFields` selection. Style-only consumers
request attention without whitespace records; spacing consumers request only
enabled fields. Container paths and prefix endpoints are cached per owner/line,
empty paths bypass tab expansion, and line-number lookup does not count UTF-16
columns unnecessarily. A dense-line CLI regression tests style-only and MD038
configurations, including quoted code.

For 8,000 code spans plus a 256 KiB tail, the optimized style-only run takes
5.321s versus 5.458s on the pre-batch baseline; review's uncorrected batch took
41.483s. Root/quoted MD038 runs take 5.226s/5.293s. Every probe preserves exact
source, emits no findings and exits 0; baseline parser costs remain.
