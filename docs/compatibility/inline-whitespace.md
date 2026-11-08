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

MD038/MD039 are planned, not implemented by this evidence.
