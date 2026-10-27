# Compatibility baseline

mdfmt targets declarative configuration and built-in rule semantics from the
following exact published versions. This audit does not implement the formatter.

## References

Retrieved on 2026-10-21:

- markdownlint-cli2 0.24.0, commit
  `341e2a331ba578029e66c2e01183e7a6b5ca9ab4`.
  [Published metadata][cli-npm] identifies this Git commit and declares an exact
  `markdownlint: 0.42.0` dependency, not a floating version range.
- markdownlint 0.42.0, commit
  `b67636d47f335cf124990faae4a72c5e9fadd613`.
  [Published metadata][lint-npm] identifies the same commit as its `v0.42.0` tag.
- The `v0.24.0` and `v0.42.0` Git tags resolved to those commits. GitHub Release
  endpoints returned 404; published-version evidence comes from npm metadata,
  not an assumed GitHub Release or an unpinned README on `main`.

Published tarball integrity values:

```text
markdownlint-cli2@0.24.0
sha512-c5hU2MDoTlS230MLzdNlkIyxImXq5acxGKz4nYz4aDhH/2BgFtpsA7iK9/OoTHMgZ4FuR0wedTQA31K6Lp9I/Q==

markdownlint@0.42.0
sha512-kzsCV/rPWRFn3jUhcB919YF4Qgeh0g9tcogoXFadGSo6Zxj6662YOqWXv0jDn8nv5KCCxDfMKJZv6N4HkNkvTg==
```

These values are metadata receipts; the audit downloaded commit source archives,
not npm tarballs, and therefore does not claim independent tarball verification.

## Artifacts and findings

- `rules.json`: all 53 built-in rules, aliases, tags, option names/types/schema
  defaults, fix-emission evidence, and proposed safe-repair classifications.
  Current implementation coverage is tracked in its `implementationCoverage`
  field and per-rule compatibility evidence, not the original audit counts.
- 33 rules can emit upstream fix information. That does not mean every
  violation they report is fixable, nor that every fix preserves meaning.
- Proposed classification: 5 safe, 32 conditional, 16 diagnostic-only. These are
  safety-policy judgments, not upstream labels or completed implementation.
- `configuration.md`: all 16 cli2 schema properties classified, plus loading,
  inheritance, inline directives, regex concerns, and file-selection semantics.
- `acceptance.json`: 43 formatting/resolution examples and 8 explicit error
  examples. Expected formatting is proposed mdfmt behavior, not upstream output.
- `verification.md`: reproducible consistency checks and narrow source experiments.
- `progress.md`: execution record and decisions taken during this audit.

### Source/documentation discrepancies

The pinned [MD013 implementation][md013] matters more than its schema or prose:

- Omitted `heading_line_length` and `code_block_line_length` inherit
  `line_length`; they are not independently fixed at 80. MD007 `start_indent`
  likewise inherits `indent`, and MD033 `table_allowed_elements` inherits
  `allowed_elements` when omitted. `defaultFrom` records these differences.
- Omitted `front_matter_title` in MD001/MD025/MD041 accepts quoted title keys,
  case-insensitively, using `^\s*"?title"?\s*[:=]`; the schema default omits the
  optional quotes. `sourceDefault` / `sourceFlags` record the source fallback.
- Strict MD013 checks standalone links. Its documentation says they are always
  exempt, but the source's `strict || ...` condition bypasses that exemption.
  Reference definitions remain exempt even in strict mode. This corrects the
  earlier conversational claim that standalone links are always exempt.
- MD013 measures JavaScript string length (UTF-16 code units), not Rust bytes,
  Unicode scalar values, grapheme clusters, or terminal display columns.
- Source metadata includes rule severity and `enabled` options. Rule/tag/alias
  application depends on declaration order, not an assumed fixed specificity.
- 50 built-in rules select the micromark parser; MD047, MD052, and MD053 declare
  no parser requirement. A Rust CommonMark/GFM parser is not automatically
  equivalent; syntax extensions and source locations need explicit compatibility
  evaluation.

### Limits imposed by preservation

Table rows and delimiters stay byte-for-byte unchanged; adjacent blank lines may
be adjusted if that preserves block meaning. This conservatively interprets the
agreed table-preservation requirement, including fixes for non-table-tagged rules
inside cells.

Inline links remain inline. MD054 settings that prohibit the existing form may
therefore leave a diagnostic. Overlong links and inline code remain indivisible.
ATX headings cannot be wrapped into paragraphs just to satisfy a width.

MD026 must be fixed when enabled, including when enabled by default. The user
explicitly approved its prescribed trailing-punctuation removal; generic content
preservation must not veto that edit. Respect the configured punctuation set,
rule disabling, and upstream exceptions. Other rule classifications remain
subject to review; this decision does not authorize arbitrary content rewrites.

## Verification evidence

Run both shell blocks in `verification.md` from the project root. In this audit,
the source cache was `/tmp/opencode/mdfmt-audit-2026-10-21`.

Observed results:

```text
Rule inventory: 53 rules; 33 source fix emitters
Proposed fixability: {'diagnostic-only': 16, 'conditional': 32, 'safe': 5}
CLI configuration: 16 properties classified
Acceptance: 43 formatting/resolution cases; 8 error cases
PASS: audit consistency only; full formatter and upstream oracle tests pending
PASS: shallow merging, declaration order/severity, explicit-base merge, and title-default source experiments
```

The inventory check compares against both the schema and rule registry, then
checks aliases/tags and option types/defaults against source. Fixture checks
cover shape, literal link/table preservation, fallback code preservation, and
selected width boundaries. They found and corrected an obsolete alias and a
nested-list example that initially exceeded its width.

The narrow Node experiments execute upstream's actual dependency-free merge
module and extracted pure effective-config and front-matter-title functions.
They establish shallow replacement, declaration-order/severity behavior,
explicit-base merging, and title defaults, not full file discovery.

Full upstream linting, parser equivalence, semantic preservation, fixture
idempotence, config-file resolution, and external formatter execution have not
been run. Every acceptance example still needs the complete upstream oracle
where applicable, and eventually an mdfmt implementation. No dependencies were
installed and no application tests or build exist yet.

## Agreed release staging

- Custom front-matter and rule regular expressions are later-release roadmap
  items. Reject unsupported custom patterns explicitly in the first release.
- Installed-package `extends` resolution is also a later-release roadmap item.
  Resolve already-installed declarative configs locally; no npm fetching,
  package installation, registry credentials, or `.npmrc` handling is required.
  Filesystem-based `extends` belongs to the first release.
- These staged limitations do not narrow the project's long-term declarative
  compatibility goal or authorize JavaScript configuration execution.

## Agreed width-based reflow

Enabled MD013 supplies a canonical prose wrapping target even for eligible
lines upstream exempts from reporting. Exact soft-break joining and paragraph
packing belong to the reflow design, not the lint oracle. The user approved this
policy: safely staying within the configured character limit is a formatting
goal, even when the original line is lint-clean. Lines at the limit are allowed;
indivisible links/code, preserved tables, and unsafe transformations remain
subject to the agreed preservation boundaries. Diagnostic exceptions still
determine whether an unavoidable overrun is reported as an MD013 violation.

No parser crate or external formatter has been selected. Before the foundation
plan, compare source-aware parser candidates against CommonMark/GFM fixtures,
micromark behavior, protected regions, UTF-16 location accounting, and localized
edits. Define the result/diagnostic model and CLI write/check/stdin behavior,
including `fix: false`, before implementing configuration resolution.

[cli-npm]: https://registry.npmjs.org/markdownlint-cli2/0.24.0
[lint-npm]: https://registry.npmjs.org/markdownlint/0.42.0
[md013]: https://github.com/DavidAnson/markdownlint/blob/b67636d47f335cf124990faae4a72c5e9fadd613/lib/md013.mjs
