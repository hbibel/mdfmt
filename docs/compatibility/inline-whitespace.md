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

MD039 remains planned, not implemented by this evidence.
