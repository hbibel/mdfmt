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
Unsupported native forms and invalid patterns receive named configuration errors.
Lookaround/backreferences are rejected here, although supported by the separate
[JavaScript rule-pattern consumers](config-patterns.md).

Pattern source is limited to 65,536 UTF-16 units, group nesting to 64 and compiled
metadata regex size to 1 MiB. Matching normalizes BOM/newlines only in a temporary
view and maps the chosen boundary back to original UTF-8 bytes. Original metadata,
including astral text, tabs and CRLF/CR, stays exact.

Complete-document candidates inherit an explicit metadata boundary, including
explicit no-match. Tab-expanded and recovered diagnostic views map that boundary
through their source offsets. Synthetic isolated probes and coordinate-only views
remain fresh. Generic authorized-edit handling also refuses prefix edits; accepting
a pattern never grants a Markdown rule or external formatter authority over it.

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

Five public tests cover configured/default/no-match/zero-width/partial-line
boundaries, BOM/newlines/astral coordinates, original edits/idempotence, all-default
rule protection, active body repairs, title effects, inline directives, malformed
configuration, no metadata subprocess launches, body subprocess success and final
failure anchors. The pinned development oracle passes 29 metadata cases; prior
42 rule-pattern and 102 package cases also pass.

The initial Rust class interpretation missed metadata for literal `&&`, `~~` and
leading dash ranges. Source-backed tests failed before literal/range translation.
RE2JS also rejects class backspace syntax that JavaScript accepts; a separate RED
control now requires the same explicit failure while translated control escapes pass.

Partial-line metadata matches expose an upstream line-offset quirk. Native
diagnostics use actual original-source coordinates instead of counting the removed
partial line as an extra line. Oracle cases label this intentional difference;
metadata extent, finding counts and source preservation remain checked.

The complete optimized suite passes 570 tests across 77 targets. Prior 554 content,
1,144 fragment, 220 fence-style, 1,662 code-style, 29 reflow, 110 config/selection
and 53 schema comparisons pass. Independent regex-batch review is pending. This is
finite Linux evidence, not complete ECMAScript/RE2JS or platform parity, and release
hardening remains unfinished.

```sh
mise exec -- cargo test --release --test custom_front_matter --test config_patterns
mise exec -- cargo build
node tools/parser-oracle/front-matter.mjs
```
