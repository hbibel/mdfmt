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
[MD055 table-pipe-style diagnostics](../../compatibility/md055.md).
Remaining built-in rules, interactions and conflict detection are next.

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
