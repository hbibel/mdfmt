# Markdown formatter roadmap

## Intent

Build a native Rust CLI that formats Markdown according to existing declarative
markdownlint configuration. Go beyond `markdownlint-cli2 --fix`, starting with
MD013 prose reflow. Users should not need a second set of formatting rules.

## Requirements

- Must respect the effective supported markdownlint configuration.
- Must reflow eligible prose to the configured MD013 width when the rule is
  enabled, even if upstream's lint exceptions leave the original line unflagged.
  The width is a formatting target, not just a diagnostic threshold. Preserve
  indivisible links/code and the other agreed preservation boundaries; a line
  exactly at the configured width is allowed.
- Must preserve content and Markdown meaning rather than force an unsafe fix.
- Must fix MD026 when enabled, removing the configured trailing punctuation
  according to upstream semantics. Its prescribed edit is authorized even though
  it changes heading text; generic preservation must not block this fix. Respect
  disabled rules, configured punctuation, and upstream exceptions.
- Must preserve inline links without conversion to reference links. Treat each
  inline link as indivisible: wrap before it when it does not fit on the current
  line but fits on its own, accounting for container indentation.
- Must preserve tables. Violations involving tables may remain unresolved.
- Should delegate fenced code formatting to an available language formatter.
  Formatter failures must not replace code with partial or failed output.
- Must report remaining violations when the effective configuration flags them.
- Must be idempotent: formatting the result again produces identical output.
- Must support declarative configuration, not JavaScript configuration execution.

Full coverage means all safely actionable built-in violations, including those
upstream does not automatically fix. It does not mean every document can become
lint-clean without author input. Missing content, unavoidable long tokens, and
conflicting rule requirements may prevent that.

## Compatibility boundary

Pin exact upstream reference versions during the compatibility audit; do not
claim compatibility with an unversioned moving target. Match rule options and
configuration semantics, not necessarily upstream's exact choice of edits.

Audit JSON/JSONC, YAML, and explicitly loaded TOML, with upstream discovery,
precedence, inheritance, `extends`, overrides, aliases, tags, inline directives,
and file selection semantics. TOML discovery must not be invented where upstream
supports only explicit loading.

JavaScript configs and Node-based custom rules, parser plugins, and output
formatters are outside the initial compatibility boundary. Unsupported settings
must receive explicit diagnostics rather than silently being ignored. Audit
settings such as JavaScript regular expressions separately: declarative syntax
does not guarantee native Rust semantic compatibility.

Custom regular expressions and installed-package `extends` resolution are planned
for later releases, not excluded from the project's compatibility goal. Until
implemented, the first release must diagnose these settings explicitly.
Package resolution locates an already-installed declarative config on the local
filesystem, following applicable package metadata and ancestor lookup rules.
It must not fetch from npm, install packages, or require registry authentication
or `.npmrc` handling. JavaScript configuration execution remains outside scope.

For MD013 diagnostics, respect all options and upstream exceptions; an overlong
line is not necessarily a violation. Those exceptions must not prevent safe
width-based prose reflow. Preserve hard breaks and container structure during
reflow. Code and table handling must not silently disable configured checking.

## Milestones

1. Compatibility contract: pinned references, built-in rule/option inventory,
   fixability classification, supported configuration boundary, acceptance cases.
2. Formatting foundation: Rust library and CLI, source-aware parsing, localized
   edits, files and stdin/stdout, write/check modes, preservation and idempotence
   tests.
3. Configuration compatibility: discovery and resolution, directives, file
   selection, and upstream comparison tests.
4. First useful release: configuration-aware MD013 prose reflow, including lists
   and blockquotes, preservation guarantees, and remaining-violation reporting.
5. Built-in rule coverage: tested batches of safe fixes, interaction tests,
   conflict detection, and progress tracked against the inventory.
6. Code formatter integration: explicit language adapters, widths where supported,
   indentation, unavailable tools, timeouts, and failures.
7. Release hardening: differential tests, real-world corpus, fuzzing,
   cross-platform CI, benchmarks, safe writes, binaries, and usage documentation.

## Release scope

Implementation status (2026-10-28): milestones 1–3 are implemented for the
first-release contract. [Configuration completion evidence](../../compatibility/configuration-completion.md)
records native/upstream comparisons, intentional policy differences and remote
platform validation not executed locally. Milestone 4's core is implemented:
configuration-aware MD013 reflow handles multiline paragraphs, nested lists and
quotes, preserves hard breaks and inline source, and reports remaining MD013
violations on the formatted output. [MD013 evidence and limits](../../compatibility/md013.md)
records tested behavior and conservative parser/preservation boundaries; full
upstream parser equivalence is not claimed. Milestone 5 has started with
[MD026's approved fix and diagnostics](../../compatibility/md026.md) and
[safe MD047 final newlines](../../compatibility/md047.md), followed by
[conditional MD012 blank-line repairs](../../compatibility/md012.md) and
[MD019/MD021 heading separators](../../compatibility/heading-spaces.md) and
[context-validated MD035 thematic-break styles](../../compatibility/md035.md), then
[context-safe MD004 unordered-list markers](../../compatibility/md004.md) and
[conditional MD009 trailing whitespace](../../compatibility/md009.md) and
[conditional MD010 hard tabs](../../compatibility/md010.md) and
[conditional MD023 heading indentation](../../compatibility/md023.md) and
[MD040 fenced-language diagnostics](../../compatibility/md040.md) and
[MD042 empty-link diagnostics](../../compatibility/md042.md) and
[MD045 missing-alt-text diagnostics](../../compatibility/md045.md) and
[MD028 blockquote gap diagnostics](../../compatibility/md028.md) and
[MD024 duplicate-heading diagnostics](../../compatibility/md024.md) and
[conditional MD048 code fence styles](../../compatibility/md048.md) and
[conditional MD027 blockquote spacing](../../compatibility/md027.md) and
[conditional MD029 ordered-list numbering](../../compatibility/md029.md) and
[conditional MD030 list-marker spacing](../../compatibility/md030.md) and
[MD018 missing-ATX-separator diagnostics](../../compatibility/md018.md) and
[MD020 closed-ATX-separator diagnostics](../../compatibility/md020.md) and
[MD011 reversed-link diagnostics](../../compatibility/md011.md) and
[MD014 protected-code prompt diagnostics](../../compatibility/md014.md) and
[MD056 table-column diagnostics](../../compatibility/md056.md) and
[MD055 table-pipe-style diagnostics](../../compatibility/md055.md) and
[conditional MD058 table separators](../../compatibility/md058.md).
The next batch adds [MD001/MD025/MD036/MD041/MD043/MD059 content diagnostics](../../compatibility/content-diagnostics.md).
This is followed by [conditional MD003 heading styles](../../compatibility/md003.md).
The literal-diagnostic batch adds [MD033 HTML findings](../../compatibility/md033.md)
and [MD060 table-column styles](../../compatibility/md060.md).
The separator batch adds [conditional MD022/MD031/MD032 heading/fence/list blanks](../../compatibility/block-separators.md).
The inline-style batch adds [conditional MD049/MD050 emphasis/strong styles](../../compatibility/emphasis-styles.md).
The inline-whitespace batch adds [MD037/MD038/MD039 bare-marker, code-span and link-label diagnostics](../../compatibility/inline-whitespace.md).
The reference batch adds [MD052/MD053/MD054 reference, definition and link-style diagnostics](../../compatibility/reference-diagnostics.md).
The literal-content batch adds [MD034 bare-URL, MD044 proper-name and MD046 code-block style diagnostics](../../compatibility/literal-content-diagnostics.md).
The list-indentation batch adds [conditional MD005/MD007 sibling/nesting alignment](../../compatibility/list-indentation.md).
MD051 adds [local-fragment diagnostics and safe ASCII case corrections](../../compatibility/link-fragments.md).
All 53 built-ins now have native coverage, including diagnostic-only rules and
conditional repairs. MD051's independent review and regression-first corrections
are complete. [Convergence and document-specific conflict reporting](../../compatibility/formatting-conflicts.md)
now add tested interactions, exact bounded histories and retained-priority
explanations. Their independent batch review found no Important/Critical defects;
95 reviewer tests and539 complete optimized implementer tests pass. Optional
[fenced-code formatter adapters](../../compatibility/code-formatters.md) now have
tested runner/API/CLI integration; their independent batch review remains pending.
Later declarative compatibility and release hardening remain unfinished. The rule
count does not establish complete fixability or parser/option fidelity.

The first useful release includes local declarative configuration,
filesystem-based `extends`, and configuration-aware prose reflow. It is not the
end of the roadmap; built-in rule coverage, code formatter integration, and
hardening continue through later releases.

Later-release declarative compatibility work must include:

- Custom front-matter and rule patterns with compatible JavaScript regex
  semantics. Preserve recognized metadata unchanged and test that it is excluded
  from applicable Markdown linting and formatting.
- Installed-package resolution for shared declarative configurations referenced
  by `extends`. Test package entry points, ancestor lookup, relative references
  within the resolved configuration, and missing-package diagnostics. Package
  installation remains the user's package manager's responsibility.

These are committed roadmap items, not first-release requirements. Their exact
release numbers and ordering relative to other later-release work remain open.

## Important implementation findings

- A convergence summary cannot take severity only from its fallback anchor:
  an inline override there can hide an Error contributor enabled elsewhere.
  The new transaction tests reproduce that misclassification before correction;
  cycle summaries now use original-document contributing states, while known-pair
  explanations retain local line states. The optimized 539-test suite passes.
- Independent MD051 review found valid line fragments such as `#L1` were silently
  retargeted when a heading/HTML anchor collided by case. Diagnostic validity
  exemptions now also gate repairs through shared classification; navigation
  preservation and repeated CLI checks have failing-before/passing-after tests.
- The review found automatic-link payloads contributed invented heading targets,
  while lossy native ATX child ranges dropped literal hashes and significant
  caption spaces. Explicit Markdown label data is now distinguished from automatic
  links, and lexical source-caption bounds retain omitted data. This prevents false
  findings, unsafe case repairs and incorrect duplicate counters.
- Multiline inline code/math projection also discarded significant data spaces
  after newlines. Literal projection now consumes only actual container prefixes,
  including partial prefixes on lazy continuations, and caches heading/line prefix
  endpoints. One-/two-space, padding-only, list/quote/lazy and physical-newline
  regressions pass. All four review findings have RED-to-GREEN coverage; complete
  optimized post-review verification passes 522 tests across 72 targets, with
  1,144 MD051 comparisons and affected heading/reference/HTML/code/reflow gates.
- MD051's new HTML-anchor consumer exposed the remaining full-definition-payload
  reparsing path in shared HTML image recovery. Four equal images caused four
  parses including unrelated definition titles. A real-parser regression failed
  before reusing the existing compact-header/paragraph cache; it now requires one
  parse and fewer than 4,096 bytes across image/definition axes. Prior HTML/image/
  reference comparisons and the complete optimized 512-test suite pass.
- Independent list-indentation review reproduced further normalization cycles:
  MD023 reverses heading-item indentation, while MD030 reverses both inserted and
  deleted padding between same-line nested markers. All four bounded CLI
  regressions fail before the correction; MD027-only insertion checks are not a
  sufficient convergence policy. Candidate checks now cover enabled heading,
  quote and padding consumers at inserted bytes and collapsed deletion boundaries,
  preserving deterministic prior normalization and stable remaining findings.
- The review also reproduced quadratic aggregate padding allocation: a 16 KB
  document with 4,000 items and start indentation 16,000 aborts under 128 MiB,
  including with fixes disabled. Per-item bounds alone do not protect a batch.
  Diagnostic classification no longer allocates padding. Aggregate replacement
  padding is checked before allocation against input bytes plus four, and disabled
  fixes do not construct authorization proposals. Seven public regressions and
  the complete optimized 503-test suite pass; the same 128 MiB probe now retains
  all 4,000 findings in both fix modes. The debug resource test still exceeds its
  30-second CPU budget after its control passes, without an allocation abort;
  unoptimized throughput remains an explicit release-hardening obligation.
- List-indentation interaction tests reproduced pre-existing MD013 prefix growth
  before a link/code span on a continuation line: the native text token ends at
  the newline, so reflow added a prefix without consuming the existing one outside
  that token. A direct MD013-only regression fails before the correction. Soft
  breaks must consume the physical paragraph prefix even when it extends beyond
  the text-token range; links/code stay indivisible and unchanged. Direct and
  cross-rule regressions pass, along with the optimized 496-test full suite.
- MD007 `start_indented: true` and MD027 can prescribe opposite edits to spaces
  after a quote marker, causing the normalization loop not to terminate. A bounded
  CLI regression reproduces the cycle. Disabled first MD027 anchors expose an
  equivalent MD005 cycle. The shared candidate gate now rejects inserted bytes
  overlapping enabled MD027 findings, retaining stable MD005/MD007 diagnostics.
  Both termination regressions and the optimized 496-test suite pass. General
  conflict explanations remain later roadmap work.
- Independent literal-content review found that recovered code lacked the owner/
  sibling metadata needed to join logical blocks, retaining false MD014 prompt
  findings and duplicate MD046 findings. Snapshots now carry mapped predecessor
  evidence from their producing parse; missing identities are not equated.
- The same review found comment prefixes classified following prose/code as HTML,
  causing both false failures and missed MD044 findings. Shared complete-comment
  validation now preserves separate Data/Code/Html flags after a closed comment.
- Recovery also missed quoted blank content, hiding later code and consistency
  anchors while exposing code to MD044 despite `code_blocks: false`. Consuming
  active container prefixes before testing blankness fixes recovery without
  erasing empty nested-container barriers. All three review findings have literal
  RED-to-GREEN regressions; the post-review optimized suite passes 475 tests across
  66 targets, with 516 MD014/1,812 MD034/2,373 MD044/1,662 MD046 native comparisons.
- A shared diagnostic view inserted a space for the BOM, changing list continuation
  depth and exposing indented code as prose. A literal regression reproduced false
  MD034 URL findings and MD044 findings despite `code_blocks: false`. BOM mapping
  now has zero synthetic width while retaining original source offsets; prior
  HTML/bare-URL comparisons and the regression pass.
- Native indented-code records can split across blank lines inside one container.
  This exposed an Important MD014 gap: `>     $ echo hi\n>\n>     output\n`
  incorrectly reported a prompt despite following output. Shared MD014/MD046
  logical snapshots now join only consecutive same-owner indented siblings across
  actual blank/prefix-only gaps, retaining original physical line distances.
  Separate/empty containers and adjacent parser tokens remain independent. Four
  regression tests and the later review corrections cover the fix; the optimized
  full suite passes 475 tests across 66 targets, without source edits.
- Independent reference review found nested resources erased reference-key data,
  falsely resolving missing labels and hiding unrelated unused definitions.
  Separate structural exclusions and data projection now preserve nested label
  data, literal bare-URL markers and inactive outer-link grouping without treating
  resource destinations as data. Literal regressions and expanded pinned
  comparisons cover the correction; no source-edit authority changes.
- The same review found math-internal brackets discarded already-owned link/image
  labels, suppressing MD054 and falsely reporting used definitions under MD053.
  Owned-label boundaries now skip inline math, while unrelated scanner callers
  retain their previous behavior.
- Quoted image recovery reparsed raw multiline definitions outside their containers,
  changing helper identity and losing native reference style. Compact logical
  definition headers now restore native resolution while preserving raw helper
  keys and original-byte ranges separately; root/image/nested quote/list cases
  retain independent MD052/MD053/MD054 checks.
- Every image also reparsed unrelated definition destinations/titles. Plain labels
  now skip recovery; other labels select referenced compact headers and reuse a
  cached paragraph. Deterministic parser-work tests vary both axes and distinct
  labels. Release probes restore near-disabled-rule timing, but pre-existing
  baseline image cost remains a later hardening requirement. The four review
  findings have regression-first fixes; the post-review suite passes 447 tests
  across 62 targets, with 6,305 reference comparisons and prior native oracles.
- Pinned MD044 URL exclusions can extend beyond the actual URL, hiding following
  proper-name findings in inline/fenced code. Its schema also permits empty names;
  zero-length global regex matches can fail to advance and exhaust resources.
  [Source/runtime evidence](../../compatibility/proper-name-audit.md) retains six
  ordinary observations and one bounded timeout/heap-abort probe. Native MD044
  must use actual URL spans and explicitly handle empty names before scanning;
  no native coverage or permission to rewrite literal code is claimed.
- MD054's new resource-field scanner initially advanced two bytes after any
  backslash, splitting UTF-8 before emoji, accented or CJK characters and panicking
  on valid Markdown. A literal regression reproduced the failure. Escape scanning
  now skips two bytes only for ASCII punctuation and otherwise advances complete
  characters; forty additional ordinary source observations cover these fields.
- Independent inline-whitespace review reproduced MD038 ranges extending through
  newlines and container prefixes to space padding on another line, also applying
  the wrong line's permissions. Padding can now extend a diagnostic only when
  adjacent to its selected data spaces. Bidirectional, CRLF, container and disabled-
  opening-line regressions cover this physical-range invariant.
- The same review measured an eightfold style-only slowdown from unused inline
  whitespace records and whole-line prefix expansion per code span. Field-selected
  snapshots, cached owner/line prefix endpoints and empty-path bypass restore
  baseline timing: 8,000 spans plus a 256 KiB tail take 5.321s versus 5.458s before
  the batch, instead of 41.483s. Dense root/quote regressions cover both style-only
  and MD038 configurations; pre-existing dense-line parsing cost remains.
- MD054 must not reuse upstream edits as authorization: a disallowed
  inline-link fix can discard the label and title when converting to an autolink.
  Source/runtime audit also found false-clean uppercase/entity-bearing reference
  styles and empty inline destinations. The [link-style audit](../../compatibility/link-style-audit.md)
  retains nine pinned source-only outcomes. Native MD054 now reports without
  converting links; eleven explicit corrected-policy cases use actual reference/
  resource syntax instead of raw-key lookups or destination truthiness. The
  remaining 1,760 ordinary source comparisons retain pinned diagnostic parity.
- Pinned MD039 combines JavaScript Unicode-aware trimming with ASCII-only RE2JS
  horizontal-whitespace matching. Unicode label spaces can then throw for line 0
  or move findings to an adjacent blank line. Separate receipts retain 42 exceptions
  across 21 inputs and six one-sided Unicode outcomes. The native rule selects
  actual horizontal whitespace on the label's physical line and preserves links;
  literal tests pin NBSP, U+2003 and BOM ranges instead of copying invalid anchors.
- Pinned MD038 throws an invalid-range exception when both syntax-padding tokens
  are newlines and the data line has edge spaces, for example a code span with
  opening/closing backticks on separate lines around ` a `. Raw rule capture
  confirms an end range of `[-1, 2]` on the data line. The native rule preserves
  code and uses valid original data-space ranges rather than copying
  newline-padding columns from another physical line. Three separate exception/raw
  receipts and native literal regressions cover this intentional difference.
- MD037 cannot treat every leftover delimiter run as a bare data token. Pinned
  micromark merges adjacent data after resolving matched attention and labels,
  but leaves paragraph-level residual markers separate. Blanket nested-scope
  suppression also misses genuine findings next to code, escapes, entities and
  quoted continuation prefixes. The shared model retains these boundaries without
  granting source-edit authority; 915 pinned observations cover the distinction.
- Independent MD049/MD050 review found missing alternate delimiter pairings and
  consistency anchors, image-label reparses without reference definitions, lost
  quote context in nested images, and marker edits unlocked only after MD026 had
  already run. Source-backed attention selection now handles diagnostics
  independently of native ownership; label reparses retain definitions/context.
  Marker normalization follows stable MD026 deletions and rejects generated
  punctuation conflicts rather than withholding MD026's approved original edits.
  Five literal review regressions and expanded ordinary-letter/nested-run
  comparisons cover these boundaries.
- Pinned MD052/MD053 declare `parser: "none"` but consume a micromark cache that
  stays empty when no parser-backed rule is enabled. Direct runtime probes then
  falsely report no missing/unused/duplicate reference findings. Enabling an inert
  micromark rule restores those findings. The [reference-rule audit](../../compatibility/reference-rule-audit.md)
  retains both outcomes and non-obvious label projections. The planned native
  rules must work independently, and their oracle must distinguish direct runtime
  behavior from parser-forced rule observations. Native MD052/MD053 now do so with
  2,037 and 1,825 paired observations respectively, preserving reference content.

- MD049/MD050 changes to adjacent runs can remove the other rule's ASCII intraword
  exclusion: `_one_**two**` under asterisk emphasis/underscore strong needs both
  changes in one call, not one per invocation. The marker stage now resolves its
  own dependencies with a bounded 16-round selection closure and original-byte
  edit composition, withholding the entire stage if it cannot settle. The combined
  oracle and bidirectional literal regressions cover this boundary; no generic
  repeated-pass formatter engine was added.

- MD049 inherits two non-obvious diagnostic boundaries: native image alt text
  loses the inline token tree that determines findings and consistency anchors,
  while upstream strips BOM/newlines before producing token columns. A source-
  mapped image-label view recovers diagnostics without permitting image edits;
  receipt/CLI coordinate normalization must not shift original edit spans.

- Independent separator review found that the 1 MiB replacement cap still admitted
  600,000 new lines on a tiny document, exhausting memory in downstream parser or
  per-line configuration state. A Linux 128 MiB regression reproduced the abort.
  Admission now also caps generated lines at 4,096 per stage, atomically; larger
  requirements remain findings. Both sides of that limit were checked under the
  same memory bound.
- Full-document preservation checks per separator made ordinary heading-heavy
  documents quadratic. Validate preferred batches together, with at most 32
  fallback subdivision/alternative checks and one final combined validation.
  Work-budget exhaustion must discard the entire stage: keeping the already
  explored subset caused further edits on a second invocation. Timed 800-heading
  and mixed-safe/unsafe regressions cover performance and stable rejection.
- Independent review also found wrong MD031 ending lines when an unterminated
  quoted fence loses its container, and missing MD022 findings for a one-dash
  setext heading after a quote. Token end points can include a newline beyond the
  final code-content line; empty-list misclassification can hide a physical
  setext boundary. Both selectors now retain source-backed diagnostic boundaries
  without independently granting edit ownership; expanded pinned matrices cover
  adjacent lists, quotes, terminators and repeated empty markers.

- Combined separator rules can request different but compatible prefixes at the
  same boundary, such as a quoted list followed by a root heading. Discarding
  all differing prefixes withheld safe repairs. Keep alternatives and choose an
  AST-safe prefix that satisfies the largest admissible request; selecting a
  shorter MD012-limited prefix can otherwise require a second invocation.
  `tests/block_separators.rs` reproduces both cases and checks stable output,
  diagnostics, protected bytes and edit coordinates.

- MD032 exposed a distinct lazy-HTML scope boundary: upstream admits the first
  lazy generic HTML line as a block in the active list/quote chain, not as inline
  paragraph text. Native AST-only recovery split lists early, while a plain
  masking workaround kept paragraphs alive too long. The list-specific diagnostic
  view now uses source-mapped synthetic block boundaries; original HTML and edit
  ownership remain unchanged. Pinned tests cover surviving/ended containers,
  following text/closing tags, tabs, BOM, metadata and mixed line endings.

- Native Code regions alone omit eligible fences after some recovered HTML
  boundaries. MD031 uses the existing recovered code snapshots, now retaining
  list ownership for `list_items: false`; recovered diagnostics still do not
  authorize edits without a matching native owner. Pinned MD031 regressions cover
  this boundary and surviving/ended list-quote containers.

- MD022's initial unreleased implementation could loop to a huge configured
  blank-line count at virtual file boundaries or allocate a huge replacement.
  Native timeout/allocation-limit regressions reproduced both failures. Count
  virtual boundary blanks arithmetically and bound separator insertions to 1 MiB
  per stage; oversized requirements retain diagnostics rather than exhausting
  resources. Fixed and verified on 2026-11-08: 358 Rust tests pass, including
  native bounded-process regressions and accepted large unsigned counts.
- MD022 requests above and below adjacent headings can refer to the same existing
  blank run at different byte offsets. Adding both independently overfilled that
  run, violated MD012 and broke second-call stability. Canonical shared-run
  insertion points now merge compatible requests; the regression failed before
  the fix. Include this boundary in later fence/list separator interactions.

- Independent MD060 review found false tables inside tab-indented list items:
  `-\t| a |` followed by two-space continuations is not an upstream table.
  The unexpanded native scan reported MD060 errors and falsely failed check/write;
  confirmed variants also affected MD055, MD056 and MD058. The shared table
  diagnostic selector now expands tabs for ownership and maps spans back to
  original bytes. Display alignment still measures original/sanitized text, not
  expanded spaces. Fixed and verified on 2026-11-08: 347 Rust tests pass, including
  shared-rule and CLI regressions; all affected table oracles remain green.

- MD033 consumes raw micromark tokens before markdownlint's comment sanitization;
  using `clearHtmlCommentText` for its diagnostic parse can fabricate valid HTML
  tags by hiding quotes inside comments. The MD033 differential regressions
  caught this design error. Keep parser-specific preprocessing separate, including
  upstream's diagnostic reparsing of unterminated comments and non-comment HTML.

Findings from the 2026-11-06 diagnostic batch:

- Upstream MD043 can crash while reporting missing headings on a metadata-only
  document without a final newline; MD059 can crash on a prohibited empty link
  label. Native regressions require valid source-backed findings rather than
  reproducing either crash. Pinned receipts distinguish upstream errors from
  native safety expectations.
- MD043 regex-prefixed heading requirements were accepted without an explicit
  compatibility boundary. They now fail explicitly until compatible JavaScript
  rule-pattern execution is implemented. Include them in the later regex work,
  not just front-matter title and fragment patterns.
- Default MD041 coverage changes CLI outcomes for untitled prose: enabled
  remaining errors yield exit 1 even when no safe edit exists or `fix: false`.
  This is configured lint behavior, not an operational failure or an instruction
  to fabricate a title.
- Independent review found important false-positive/false-negative diagnostics
  around indented HTML titles, raw Docfx link destinations, setext headings after
  recovered HTML, and quoted multiline image labels/reference definitions. The
  2026-11-07 fix pass adds pinned regressions for all of these boundaries.
- MD036 punctuation is a JavaScript character class, not a literal character
  set. Ranges, negation and escapes now use native ECMAScript matching with a
  bounded suffix input. Invalid classes and arbitrary regex injection receive
  explicit configuration errors; retain that boundary in later regex work.
- MD003 exposed two important idempotence interactions. MD058 table separators
  must precede heading-style fixes, and generated empty closed headings must
  already satisfy MD021 spacing. Both interactions have single-invocation
  regression tests; no generic repeated-pass engine was added.
- The literal-hash range defect also causes false MD024 duplicates and MD043
  required-heading mismatches: `# # Title` has the upstream caption `# Title`,
  not `Title`. The 2026-11-07 diagnostic follow-up corrects single-line ATX source
  boundaries while preserving existing multiline/HTML key semantics. MD025's
  Docfx exception also requires a whole-caption link rather than trusting native
  child counts. Pinned comparisons cover these caption/context corrections.
- MD060 alignment uses upstream's grapheme/display-width algorithm, not MD013's
  UTF-16 length. The audited Node 24.19.0 runtime uses Unicode 17.0/ICU 78.3:
  Arabic `لا` measures 2, unmatched Hangul `ᅡ` measures 1, tabs contribute 0 and
  ANSI escapes are stripped. Native width-library defaults can differ (for
  example, documented Arabic ligature collapsing). Pin the oracle's Unicode
  environment and test an adapter rather than assuming a width crate is equivalent.
- MD060 uses comment-sanitized lines for display widths while retaining raw
  micromark token columns. Combining marks, tabs and ZWJ emoji inside comments
  therefore need different width input from MD033. Pinned tests caught this
  distinction and ANSI translation mistakes: JavaScript `\d` is ASCII, and
  string-width skips stripping C1 OSC sequences unless ESC or CSI is also present.
  The native adapter now covers all regional-indicator pairs and these cases.

## Critical implementation flaws

- Independent emphasis review confirmed rendering loss beside ordinary letters,
  not just astral text: converting `a***one***` to `a*__one__*` removes outer
  emphasis, while `a***one* two**` to `a**_one_ two**` removes outer strong
  emphasis under the pinned parser. Native AST equality approved both. A native
  source-backed adapter now applies pinned UTF-16 attention flanking, delimiter
  consumption and rule-of-three resolution; repairs preserve the entire
  attention topology as well as the native AST and protected bytes. Regressions
  cover both rules and conversion directions, with expanded pinned comparisons.

- Provisional MD049 marker swaps could change upstream meaning despite identical
  native ASTs: `😀*one*😀` became `😀_one_😀`, which the pinned parser treats as
  literal text. Conversely, native-only underscore tokens could turn literal
  `😀_one_😀` into emphasis. Pinned micromark classifies adjacent UTF-16 surrogate
  units differently from the native scalar-based parser. Native regressions and
  the independent token-structure oracle failed before the correction; such
  underscore tokens are now excluded, and unsafe underscore repairs remain
  findings. MD050's combined cases also exposed loss of outer emphasis when only
  an inner strong run changed beside astral text. The general attention-topology
  guard supersedes the original astral-only ancestry exception. Apply this
  boundary to later delimiter changes.

- Separator review exposed an existing native `markdown` parser panic on mixed
  list/fence boundaries, including
  `- item\n---\n1. item\n-\n  ```\n    code\n1. item\n`.
  It reproduced on the pre-batch baseline and crossed the public library API;
  CLI processing exited 101 even with every rule disabled. All native AST entry
  points now contain unwinding dependency panics: root failures become explicit
  parsing errors and CLI exit 2, without file replacement. The host's panic hook
  is not changed and may still log the caught panic. `tests/parser_failures.rs`
  verifies containment and unchanged files. Full support for these valid inputs
  remains a parser-hardening requirement; the MD031 receipt labels 16 error
  outcomes separately from diagnostic parity. Abort-on-panic builds cannot offer
  unwind containment.

- Unreleased MD003 conversions initially deleted literal leading/trailing hashes
  because
  native heading-child ranges omit source text that upstream retains. Native AST
  equivalence alone does not detect this loss when whole heading bytes are exempt.
  Confirmed examples include `# # Title` and `# Title # #`; the latter can also
  expose punctuation to MD026 on a second invocation. The fix requires lexical
  source-caption preservation, rejects ambiguous native boundaries and compares
  upstream heading identities before and after every oracle conversion. Failing
  regressions now pass, including 1,206 pinned/generated cases. Fixed and verified
  on 2026-11-07: 332 Rust tests pass, including direct safety-guard regressions,
  content/level preservation and MD026 idempotence. Ambiguous captions remain
  unchanged with an MD003 finding.

## Initial audit context and builder decisions (historical)

The workspace contains `mise.toml`, pinning Rust 1.99, and `opencode.jsonc`;
there is no application scaffold or Git repository. Do not initialize a repository,
install dependencies, or build the application as part of the audit plan.

Parser selection, crate layout, CLI spelling, diagnostics format, external
formatter defaults, and exact adapter choices remain builder decisions for their
respective milestones. Evaluate source preservation and upstream parsing
differences before selecting a Markdown parser. Node may be used as a development
oracle without becoming a required runtime dependency for the formatter.

The immediate deliverable is a plan for milestone 1, not an implementation plan
for the entire formatter. Later plans should use the audit's findings.
