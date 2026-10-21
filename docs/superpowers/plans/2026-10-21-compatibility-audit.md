# Compatibility Audit Implementation Plan

> For agentic workers: Use the executing-plans skill to execute this audit
> task-by-task. This is a documentation and acceptance-fixture milestone, not
> formatter implementation. Steps use checkbox syntax for tracking.

Goal: Establish a versioned, testable compatibility contract for mdfmt.

Architecture: Use exact upstream releases as the behavioral reference. Separate
rule semantics, configuration resolution, and safe formatting decisions so that
later implementation can be verified without requiring identical upstream edits.

Tech stack: Markdown documentation and JSON acceptance data; pinned upstream
markdownlint sources. The application will use Rust.

Spec: `docs/superpowers/specs/2026-10-21-mdfmt-design.md`

## Global constraints

- Must preserve content and Markdown meaning rather than force an unsafe fix.
- Must preserve inline links without conversion to reference links.
- Must preserve tables.
- Must support declarative configuration, not JavaScript configuration execution.
- Must be idempotent: formatting the result again produces identical output.
- Do not scaffold the application, install dependencies, initialize Git, or commit
  during this audit without further authorization.
- Never read `.env`.

## Review focus

- Standalone links and unbreakable tokens: apply upstream MD013 exemptions.
- Nested containers and hard breaks: wrapping must preserve Markdown meaning.
- Nested configuration and overrides: use the actual effective configuration.
- Node-dependent settings in declarative files: diagnose unsupported behavior.
- Tables and unavailable code formatters: preserve content without hiding lints.

## File structure

- `docs/compatibility/baseline.md`: exact reference versions and source links.
- `docs/compatibility/rules.json`: machine-readable built-in rule inventory.
- `docs/compatibility/configuration.md`: discovery and resolution contract.
- `docs/compatibility/acceptance.json`: illustrative input/config/output cases.

### Task 1: Pin the reference and inventory rules

Consumes: the approved scope in the spec.

Produces: `baseline.md` and `rules.json`. Each rule entry has `id`, `aliases`,
`tags`, `options`, `upstreamFixable`, `fixability`, `rationale`, and `source`.
Each option has `name`, `type`, and `default`. `fixability` is `safe`,
`conditional`, or `diagnostic-only`; document conditional transformations.

- [x] Resolve an exact released cli2 version and its actual markdownlint
  dependency version from release metadata. Record immutable tags or commit
  links, retrieval date, and the distinction between supported target and current
  implementation coverage (initially none).
- [x] Enumerate every built-in rule from that reference's source/schema. Record
  options and upstream fix support from source, not assumptions based on names.
- [x] Classify each rule by safe transformation availability. Explicitly record
  that table preservation limits automatic repair of table rules and that code
  formatting remains conditional on an external tool.
- [x] Cross-check inventory IDs and options against the pinned source/schema.
  Parse `rules.json` with an available JSON parser; verify unique IDs, required
  fields, and exact set equality with the upstream built-in rule IDs. Record the
  verification command and result in `baseline.md`.

### Task 2: Specify configuration compatibility

Consumes: Task 1's pinned reference and sources.

Produces: `configuration.md`, with supported, unsupported, or deferred status
for every reference CLI configuration property and loading mechanism. Each
non-supported entry includes the intended explicit diagnostic behavior.

- [x] Document discovery names and precedence, rule defaults/aliases/tags,
  `extends`, nested configurations, overrides, relative path bases, explicit
  config loading, and JSON Pointer support where present in the pinned release.
- [x] Document inline directives, `noInlineConfig`, globs, ignores, and gitignore
  semantics. Include concrete examples for competing files in one directory,
  parent/child resolution, and first-match override behavior.
- [x] Audit every cli2 property against native implementation feasibility,
  including custom rules, plugins, module paths, output formatters, and
  JavaScript regex semantics in `frontMatter`. Do not equate declarative input
  with executable-feature support or silently invent fallback semantics.
- [x] Cross-check the property inventory against the pinned schema. Confirm
  every property has a status and source, and every unsupported behavior has a
  diagnostic policy. Record any unresolved compatibility decisions explicitly
  for user review before implementation.

### Task 3: Define formatting acceptance cases

Consumes: Tasks 1 and 2's rule semantics and effective configuration contract.

Produces: `acceptance.json`. Each case has `id`, `input`, `config`,
`expectedOutput`, `remainingRuleIds`, and `rationale`. Configuration-resolution
cases additionally have `files`, `cwd`, `target`, and `expectedEffectiveConfig`.
External formatter cases additionally have `formatterScenario` set to `success`,
`unavailable`, `failure`, or `timeout`; no real formatter is required in this audit.

- [x] Write exact expected cases for prose wrapping at 60 characters, an inline
  link that moves intact to a new line, an overlong indivisible link, and a
  standalone link exempt under upstream MD013. Include strict/stern/default
  distinctions and count container prefixes within the configured width.
- [x] Add nested-list and blockquote cases, explicit hard breaks, inline code,
  preserved tables with checking enabled/disabled, headings with independent
  limits, and code blocks with independent limits. Record remaining violations
  rather than expecting forced rewrites.
- [x] Add configuration cases for same-directory precedence, nested resolution,
  `extends`, first-match overrides, inline disabling, and `noInlineConfig`.
  Unsupported executable features receive a separate explicit error expectation
  rather than fabricated effective configuration or formatting output.
- [x] Add code formatter success, unavailable, failure, and timeout examples.
  Successful output must retain fence/container structure; all other cases must
  preserve original code. Mark adapter-specific behavior as proposed rather than
  already implemented or user-selected.
- [x] Parse the JSON and manually verify every expected output against the
  pinned rule source, configured widths, and preservation requirements. These
  are acceptance specifications, not passing formatter tests. Record which
  expectations still require execution against the upstream oracle.

### Task 4: Review and hand off

Consumes: all audit artifacts.

Produces: a concise summary in `baseline.md` of scope, unresolved decisions,
coverage gaps, and recommendations for the formatting-foundation plan.

- [x] Check every spec requirement against the inventory or acceptance cases.
  Check all five review-focus classes have concrete acceptance examples.
- [x] Distinguish source-verified facts, experimentally verified behavior, and
  proposed mdfmt policy. Do not claim runtime parity from documentation review.
- [x] Present the contract and remaining decisions for user approval. Do not
  select parser dependencies or start application implementation in this task.
