# Native custom configuration patterns

## Intent

Complete the roadmap's declarative pattern support without running JavaScript.
Custom metadata must remain byte-exact while ordinary Markdown rules, directives,
reflow and optional code formatters operate on the remaining document.

The reference is pinned markdownlint0.42.0/cli2 0.24.0, not an assumption that every
pattern executes through the same JavaScript engine. Package resolution's separate
review/fix pass closed at `89c0c36`, with 561 complete optimized tests passing.

## Source-backed pattern contracts

- cli2 `frontMatter` constructs a JavaScript regex with `u` when its string is
  truthy. Markdownlint translates it and executes RE2JS, accepting only a match
  starting at offset zero after BOM/newline normalization. Missing/empty cli2
  values retain default metadata recognition. RE2JS rejects lookahead, lookbehind
  and backreferences in actual probes; its whitespace classes are ASCII, unlike
  ECMAScript whitespace. Do not silently replace this execution with generic JS
  matching or change the existing default front-matter recognizer.
- MD001/MD025/MD041 `front_matter_title` executes JavaScript regex with `i`, not
  `u`, against individual recognized metadata lines. Empty strings disable the
  title effect. Preserve the pinned default and rule-specific interactions.
- MD043 `%`-prefixed required headings execute regex text against heading text,
  with `u` and with `i` unless `match_case` is true. Prefix length still enforces
  heading level; existing loops/wildcards remain unchanged.
- MD051 `ignored_pattern` executes an unflagged JavaScript regex against the
  fragment at its pinned comparison point. Empty means the existing `^$` behavior.
  An ignored destination must not receive the conditional case-correction edit.
- MD036 punctuation builds a raw JavaScript character class. Keep the already
  supported class semantics and explicit invalid-pattern handling; do not broaden
  it into an unrelated regex-injection feature.

Actual probes establish accepted frontMatter metadata, zero-width matches,
non-prefix matches and unsupported constructs. A partial-line frontMatter match
exposes an upstream coordinate quirk: its line offset counts the removed text as
one full line. Native diagnostics must retain actual original-source coordinates,
not report a nonexistent extra line. Record that intentional difference.

## Native engines and bounded execution

Use the existing vendored regress engine for JavaScript rule patterns, preserving
per-consumer flags, UTF-16/non-Unicode behavior and its deterministic execution
fuel. Do not use Rust regex for backreferences or lookaround. Validation failures
must name the rule/property; execution exhaustion must reach ProcessingError::WorkLimit
and prevent output/write, never become an ordinary nonmatch.

For frontMatter, first validate ECMAScript `u` syntax, then compile the accepted
RE2-compatible form using the existing Rust regex dependency. Translate RE2JS's
ASCII shorthand/word-boundary behavior deliberately, with consuming differential
cases for classes, Unicode properties, escapes, anchors and normalization. Reject
constructs the selected native backend cannot represent with an explicit
configuration diagnostic rather than substituting a different pattern. Keep the
default recognizer unchanged when no custom pattern is configured.

Bound source patterns to 65,536 UTF-16 units, structural nesting to 64 and compiled
frontMatter regex size to 1 MiB. Bound each document cache to 256 compiled entries
and 1 MiB of source patterns, not a global cache.
Single-match consumers must never use an unbounded global iterator or loop on a
zero-length match. Existing glob compilation and its exact legacy behavior are
not part of this change.

## Metadata context through the pipeline

Add optional `front_matter: Option<FrontMatterPattern>` to ResolvedConfig and resolve it using
the pinned invocation-level cli2 option precedence. It is not a rule key and must
not be merged into RuleConfig or supplied through inline configure-file rules.
Keep FormatOptions and the pure/process-I/O API split unchanged. The public
FrontMatterPattern constructor validates/compiles once; cloning shares immutable
compiled state rather than recompiling at every formatting phase.

Create a configured Source at processing entry, normalizing only for matching and
mapping the selected prefix boundary back to original UTF-8 bytes. Source stores
an explicit prefix override distinct from default recognition, including explicit
no-match/zero-width results. Ordinary Source::new keeps existing default behavior.

All complete-document candidate reparses must inherit that prefix boundary.
Derived byte-preserving views retain it; tab-expanded views map it through their
offset tables. Truly synthetic isolated probes remain fresh Sources. Audit every
Source::new call rather than relying on one top-level parse. No thread-local or
global parser state is allowed.

The matched prefix is a protected FrontMatter region, not Markdown content.
Recognized HTML-style markdownlint directives retain pinned semantics even inside
metadata strings: an actual default-metadata probe confirms a disable directive
there suppresses a later body finding. Do not invent a new directive exclusion.
Native AST source bases, final diagnostics and composed edits continue to address
original bytes. Candidate validators must compare protected metadata exactly.
Formatting may change the body but cannot move or shorten that prefix. All
Markdown phases, final checks, conflict rollback, reflow and code formatter
ownership/warning anchors consume the same context.

Rule pattern work is fallible. Propagate errors from heading/fragment diagnostics
and MD051 proposals through their existing callers; do not stash errors in global
state, emit a warning for a nonmatch or silently suppress unrelated findings.

## Verification

Public tests and a development-only pinned oracle must cover:

- Custom/missing/empty/nonmatching/zero-width/partial-line metadata patterns; BOM,
  LF/CRLF/CR, astral content, ASCII versus Unicode whitespace and default fallback.
- Metadata containing apparent headings, tables, links, HTML directives and fences;
  enabled rules and code-format opt-in must not edit or execute its content, while
  recognized directive permissions retain existing pinned behavior.
- Body headings/reflow/links/code still work after metadata; exact original edits,
  final warning positions and a second no-edit invocation must hold.
- Title patterns for all three rules; MD043 case/level/loop/wildcard behavior;
  MD051 ignored destinations, aliases and inline configured/disabled states.
- Lookaround/backreferences for actual JavaScript rule consumers, invalid patterns,
  Unicode/non-Unicode distinctions, bounded adversarial execution, error provenance,
  unchanged input on failure and recovery in later independent calls.

Retain all existing configuration, metadata, heading, fragment, reflow, formatter
and preservation tests. Run the complete optimized suite and affected native
oracles. One independent batch review follows, with one regression-first
Important/Critical fix pass and no re-review. Do not merge or push.

## Builder decisions and costs

1. Reuse regress for JS consumers and Rust regex for the pinned RE2-style metadata
   consumer. Cost if wrong: explicit unsupported-pattern errors or source-backed
   translation corrections, not a hidden Node runtime or semantic substitution.
2. Add the optional ResolvedConfig field rather than hide document options in a
   rule map or global state. Cost if wrong: prerelease callers using exhaustive
   struct literals must add the field or use `..Default::default()`.
3. Carry an explicit protected-prefix boundary through full-document Sources and
   derived views. Cost if wrong: more internal plumbing and conservative rejected
   edits, rather than rules or subprocesses reading metadata as Markdown.
4. Preserve actual-source coordinates for partial-line prefix matches. Cost if
   wrong: those diagnostic positions intentionally differ from upstream's phantom
   removed-line offset; source edits remain unambiguous and reconstructable.
5. Reject unsupported native pattern forms and propagate resource exhaustion.
   Cost if wrong: some valid upstream patterns remain explicit boundaries rather
   than being silently approximated. No all-ECMAScript-input proof is claimed.
6. Retain pinned HTML directive scanning within metadata, as within literal code.
   Cost if wrong: metadata strings can suppress later rule edits, not silently
   become a newly permission-isolated syntax category.
