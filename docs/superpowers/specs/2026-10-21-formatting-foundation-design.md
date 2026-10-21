# Rust formatting foundation

## Intent and scope

Implement roadmap milestone 2: a native Rust library and CLI that can identify
Markdown structure and apply validated localized edits without rewriting unrelated
source. This is infrastructure, not the first useful formatter release.

The user approved the foundation scope and CLI contract discussed below. Parser
selection remains conditional on implementation-time comparison. Detailed internal
interfaces and conservative filesystem policies are builder recommendations.

No production rule fixes ship in this milestone. Configuration discovery,
resolution, inline directive semantics, file enumeration, prose reflow, MD026,
remaining built-in rule diagnostics, and external formatters remain later work.
The approved MD013 and MD026 policies in the compatibility contract are unchanged.

## Package and parser

Use Rust 1.99 from `mise.toml`, one Cargo package named `mdfmt`, a library, a thin
binary, and tracked `Cargo.lock`. No Node runtime dependency is permitted. Node
may run the pinned upstream oracle in development.

Compare these exact candidates before selecting the production dependency:

- `markdown` 1.0.0 (`markdown-rs`): positioned AST and micromark sibling; preferred
  candidate, but public AST positions are not a lossless concrete token stream.
- `pulldown-cmark` 0.13.4: source-range event iterator; container reconstruction
  and extension differences need investigation.
- `comrak` 0.57.0: positioned CommonMark/GFM tree; byte-based line/column positions
  need conversion. Do not use its Markdown renderer for output.

Compare nested lists/quotes, lazy continuation, links/images, reference definitions,
tables, HTML/comments, fenced/indented code, escaped punctuation/entities, hard
and soft breaks, default front matter, BOM, CRLF, tabs, and non-BMP Unicode.
Use markdownlint 0.42.0 / cli2 0.24.0 pinned references from `baseline.md` to
establish syntax and default front-matter behavior. Compare classifications and
source slices, not equal AST shapes. Record observed differences and limitations.

Selection requires usable original-source ranges and no unmitigated mismatch
that would expose protected content to later edits in this fixture set. If no
candidate meets that gate, stop and report evidence rather than invent a parser.
Do not claim complete parser equivalence from this bounded comparison.

Hide the chosen dependency behind a private adapter. Expose only the small
source-region model needed by the foundation, not a universal parser interface.

## Source and edit model

Original UTF-8 source is authoritative. Keep BOM, line terminators, and final
newline presence intact; no implicit normalization. Invalid UTF-8 is an input
error, not an invitation to replace characters. A line index accounts for LF,
CRLF, and lone CR. Byte ranges are zero-based and end-exclusive.

Use byte offsets for edits and explicit UTF-16 code-unit accounting for upstream
widths and diagnostic columns. Human positions are one-based. Tabs count as one
UTF-16 unit for width; parser indentation rules are a separate concern. Test
non-BMP characters and reject byte positions within UTF-8 characters.

An edit contains its original-source range, replacement text, and origin. Validate
the complete batch before applying any edit: ranges must be ordered internally,
in bounds, on character boundaries, non-overlapping, and allowed by the region
policy. Sort deterministically; reject multiple insertions at the same offset,
and insertions touching a replacement boundary rather than guess precedence.
Adjacent non-empty replacements are allowed. Apply by copying untouched slices.

Protected blocks include tables (whole rows/delimiters), code, raw HTML/comments,
default front matter, and reference definitions. Indivisible inline spans include
links/images and inline code. Policy distinguishes forbidden edits from atomic
spans that a later authorized transformation may move intact. No production rule
may edit through these boundaries by default. The foundation's test-only edit
producer operates solely in explicitly eligible plain prose.
The foundation conservatively withholds multiline text-node ranges because their
source slices can include continuation-line container prefixes. Reflow will need
container-aware segmentation in its own milestone.

Until directive semantics exist, the production formatter proposes no edits.
Future rule implementation must supply effective per-rule/per-line permissions;
an incomplete directive implementation must never silently enable fixes.

## Results and errors

Separate successful processing from lint cleanliness. A successful library result
contains output, applied edits, diagnostics, and explicit coverage metadata stating
that no built-in rules are implemented. Derive `changed` from output versus input.
Diagnostics carry a stable code, warning/error severity, message, optional rule
ID, and optional byte span. Keep operational errors separate from rule findings.

Diagnostics describe the returned output; errors that prevent processing use the
original input coordinates. Empty diagnostics do not imply markdownlint compliance.
The library must perform no filesystem or terminal I/O.

Edit permission is independent of output mode. With effective `fix: false`, do
not apply edits in any mode. Future diagnostics still run on original content.
The foundation accepts this permission through its typed API; it does not pretend
to load configuration files or implement rule enablement.

## CLI contract

- `mdfmt file.md` prints the single file's output without modifying it.
- `mdfmt -` reads stdin and prints output.
- `mdfmt --write file.md …` updates files in place.
- `mdfmt --check file.md …` checks files without writing or printing content.
- `mdfmt --check -` checks stdin without printing content.
- No arguments prints usage and exits 2; `--help` and `--version` exit 0.
- `--write` and `--check` conflict. Reject write-with-stdin, repeated stdin,
  mixed stdin/files, and multiple inputs in stdout mode.
- Accept literal files only; support `--` for names starting with a dash. No
  directory recursion, glob expansion, ignores, or config options yet. Shell
  expansion simply supplies literal paths subject to the selected mode's limits.
- All diagnostics/status go to stderr; stdout contains only document content.
  No banner. Check success means no implemented changes/errors, not lint-clean.
- Exit 0 on success, 1 for needed changes in check mode or remaining error-severity
  findings, and 2 for usage/I/O/processing failures. Warnings alone do not fail.
  Operational failures take precedence across multiple files; otherwise aggregate
  deterministically in argument order. Process remaining files after a per-file
  failure; there is no multi-file transaction.

`--write` requests writes but never overrides future effective `fix: false`.
With fixes disabled, stdout emits original source, write leaves files untouched,
and check still fails for remaining error-severity findings. It does not perform
a secret fix-enabled preview. CLI config integration belongs to milestone 3.

## Filesystem safety

Read regular files only. Read/check may follow symlinks; write must reject
symlinks and, where link counts are available, multiply linked files. Deduplicate
identical target files in write/check processing; preserve first argument order.
Do not rewrite unchanged files or alter their modification times.

For changed files, create a temporary file in the same directory, write/flush the
complete output, preserve ordinary permissions, and replace the destination only
after success. Reject observed target changes between read and replacement. On
failure, retain the original file and clean up the temporary file. Never truncate
the destination as a fallback. Platform-specific metadata, adversarial race-free
filesystem guarantees, and crash durability remain release-hardening work; do not
claim them here. Report unsupported replacement safely instead of weakening it.

## Evidence and acceptance

Production is a byte-preserving no-op, exercised through library and CLI tests.
Test-only transformations prove actual localized edits, rejection behavior,
protected-region enforcement, changed output, and second-pass idempotence. These
must call the same edit validation and processing path as production; do not ship
a hidden testing flag or hard-coded public formatting behavior.

Add regression fixtures for all parser comparison categories, Unicode coordinate
conversion, insertion/replacement conflicts, preservation outside edits, invalid
UTF-8, CLI stream/exit behavior, unchanged-file timestamps, symlink/hardlink write
rejection, replacement failure, and observed concurrent modification.

Existing `acceptance.json` remains a future behavioral contract, not a suite of
passing formatter tests. New tests may reuse its inputs for preservation but must
not substitute no-op output for its expected formatted output. Full lint oracle,
corpus, fuzzing, benchmarks, and cross-platform release CI remain later work.

## Sources reviewed during planning

- Repository roadmap and `docs/compatibility/{baseline,configuration,progress}.md`.
- <https://docs.rs/markdown/1.0.0/markdown/> and the tagged markdown-rs README.
- <https://docs.rs/pulldown-cmark/0.13.4/pulldown_cmark/struct.OffsetIter.html>.
- <https://docs.rs/comrak/0.57.0/comrak/nodes/struct.LineColumn.html>.

No candidate or formatter runtime was executed during this design review.
