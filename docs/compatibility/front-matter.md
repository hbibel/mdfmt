# Configured front matter

cli2 `frontMatter` now selects a protected metadata prefix without running Node.
An absent or empty CLI pattern retains the existing default recognizer. A custom
pattern that does not match at offset zero, or matches zero width, does not fall
back to default metadata recognition.

## Pattern and source semantics

The pinned loader validates JavaScript Unicode syntax, translates the expression
and executes RE2JS. Native code validates that JavaScript syntax, then compiles the
supported RE2-style form with bounded Rust regex. ASCII shorthand classes, word
boundaries, literal class operators/ranges and control escapes are translated
deliberately; raw Rust class set syntax is not substituted for RE2JS behavior.
Unicode properties use 56 checked-in classes generated from pinned RE2JS 2.8.6
compiled instructions, not Rust's Unicode tables. The generator records its source
hash and excludes surrogate scalars, which cannot occur in UTF-8 input. The same
property vocabulary is enforced before backend compilation; aliases accepted only
by Rust are configuration errors. Repetition counts, including nested counted
expansion, cannot exceed RE2JS's 1,000-count bound.
Unsupported native forms and invalid patterns receive named configuration errors.
Lookaround/backreferences are rejected here, although supported by the separate
[JavaScript rule-pattern consumers](config-patterns.md).

Pattern source is limited to 65,536 UTF-16 units, group nesting to 64 and compiled
metadata regex and translated source size to 1 MiB. Matching normalizes BOM/newlines only in a temporary
view and maps the chosen boundary back to original UTF-8 bytes. Original metadata,
including astral text, tabs and CRLF/CR, stays exact.

Complete-document candidates inherit an explicit metadata boundary, including
explicit no-match. Tab-expanded and recovered diagnostic views map that boundary
through their source offsets. Synthetic isolated probes and coordinate-only views
remain fresh. Generic authorized-edit handling also refuses prefix edits; accepting
a pattern never grants a Markdown rule or external formatter authority over it.
Candidates must also retain the same metadata recognition on the next independent
invocation. Assertion-dependent body repairs that would change that boundary are
withheld with the remaining rule finding, or rejected transactionally where the
owning rule cannot withhold them. Full transactions check the initial context;
process-I/O stages inherit it rather than replacing it after body formatting.

HTML markdownlint directives retain pinned scanning semantics inside metadata
strings, as inside code. Metadata is excluded from Markdown rule content, not
invented as a new directive-isolation mechanism. Body rules and optional formatters
still run where permitted; failures remain anchored to final body fences.

## API and evidence

`ResolvedConfig` gains `front_matter: Option<FrontMatterPattern>`. Use
`FrontMatterPattern::new` for a validated, cheaply cloned compiled pattern; None
retains defaults. Existing exhaustive prerelease struct literals must add the field
or use `..Default::default()`. The pure API and opt-in process-I/O split are unchanged.
cli2 precedence remains invocation-level, not a nested rule or inline option.

Eight public tests cover configured/default/no-match/zero-width/partial-line
boundaries, BOM/newlines/astral coordinates, original edits/idempotence, all-default
rule protection, active body repairs, title effects, inline directives, malformed
configuration, no metadata subprocess launches, body subprocess success and final
failure anchors, assertion-dependent body edits, newer-script property membership
and transactional RE2JS-invalid pattern rejection. The pinned development oracle
passes 280 metadata cases and 82 rule-pattern cases. Property generation checks
all 56 pinned classes without adding a runtime Node dependency.

The initial Rust class interpretation missed metadata for literal `&&`, `~~` and
leading dash ranges. Source-backed tests failed before literal/range translation.
RE2JS also rejects class backspace syntax that JavaScript accepts; a separate RED
control now requires the same explicit failure while translated control escapes pass.

Partial-line metadata matches expose an upstream line-offset quirk. Native
diagnostics use actual original-source coordinates instead of counting the removed
partial line as an extra line. Oracle cases label this intentional difference;
metadata extent, finding counts and source preservation remain checked.

The independent review found four Important defects and no Critical defect.
Regression-first corrections protect metadata recognition across body edits,
replace mismatched Unicode tables, reject RE2JS-invalid properties/repetitions and
correct standalone legacy/bracketed Unicode shorthand folding. The previously
reported 570-test result predates those findings; correction-pass verification is
recorded separately below.

Correction-pass optimized verification passes 574 tests across 77 targets, plus
32 focused debug tests and 537 vendored-engine tests (one existing doctest ignored).
Post-correction reruns pass 102 package, 554 content, 1,144 fragment, 220 fence-style,
1,662 code-style, 29 reflow, 110 config/selection and 53 schema comparisons, plus
126 fixed glob, 755 generated glob and 755 actual cli2 selection cases. Actual
Prettier 2.8.8/3.6.2 isolation controls pass. The unchanged 4,000-item debug list
resource test still fails its 30-second subprocess budget; it remains a separate
hardening obligation, not a passing debug-suite claim. This is
finite Linux evidence, not complete ECMAScript/RE2JS or platform parity, and release
hardening remains unfinished.

```sh
mise exec -- cargo test --release --test custom_front_matter --test config_patterns
mise exec -- cargo build
node tools/parser-oracle/front-matter.mjs
node tools/parser-oracle/front-matter-properties.mjs
```
