# Declarative configuration contract

This is the target contract for markdownlint-cli2 0.24.0 / markdownlint 0.42.0,
not a claim of complete compatibility. A native loader and directive/selection
support now exist; [implementation evidence and remaining limits](configuration-completion.md)
distinguish tested behavior from targets below.

Sources are the pinned [cli2 implementation][cli], [schema][schema],
[option merge function][merge], and [library implementation][library].
See `rules.json` for all 53 built-in rules and their options.

## Loading mechanisms

- Supported target: automatic `.markdownlint-cli2.jsonc` then
  `.markdownlint-cli2.yaml`, choosing only the first existing file.
- Supported target: automatic `.markdownlint.jsonc`, `.markdownlint.json`,
  `.markdownlint.yaml`, `.markdownlint.yml`, in that order, independently of the
  cli2-file search. JSON files also allow comments upstream.
- Supported target: explicit configuration paths in JSON/JSONC, YAML/YML, or
  TOML, plus JSON Pointer selection into the parsed object. No automatic TOML
  discovery. Known filename suffixes identify cli2 options versus rule config;
  for generic filenames, the presence of any cli2 options key selects cli2 kind.
- Supported target: declarative rule-config `extends`, including inside cli2
  `config` and override `config`, shallow merging with local entries winning.
  Relative paths resolve from the referring file; support home-relative paths.
  Standalone `extends` returns inherited keys followed by local keys. Cli2 instead
  copies resolved keys into the existing `config`/override object: local key
  positions stay first and previously absent inherited keys append. This
  distinction can change tag/alias application order; both forms have runtime
  regression coverage in the configuration evidence.
  Bare module/package resolution is deferred: emit
  `unsupported-config-resolution` in the first release. A later release must
  resolve already-installed declarative packages using compatible local lookup
  and package metadata. This does not fetch from npm or use `.npmrc`.
- Unsupported: `.cjs` / `.mjs`, whether selected automatically or explicitly.
  Emit `unsupported-config-format` with the selected file path. A lower-priority
  JS file shadowed by a declarative file does not need execution or rejection.
- Deferred: exact JS/YAML/TOML parser coercion and malformed-input parity.
  Invalid or unsupported input must produce `invalid-config`, never silently
  substitute defaults. Parser selection belongs to the configuration milestone.

Diagnostic identifiers here are proposed semantic categories, not final CLI
message wording or numeric exit codes. Errors must name the path/property and
stop formatting affected input; they must not permit a misleading success.

## Discovery, inheritance, and precedence

Start discovery at the invocation directory, then visit applicable descendant
directories. This is not an unconditional search upward to the repository root.
When targeting paths outside the invocation tree, upstream applies the base
configuration at the closest common ancestor; test that separately before
claiming file-selection parity.

An explicit `--config` is a base configuration, not an instruction to suppress
discovery or force highest precedence. At the invocation directory, upstream
merges the CLI fix default, the explicitly loaded options, then the discovered
cli2 options, in that order. Discovered rule keys therefore override explicit
base keys; other explicit keys survive unless replaced. A selected standalone
`.markdownlint.*` config can subsequently win over the merged cli2 `config`.
Explicit config's own `extends` resolves relative to that file. The upstream
`relativeDir` handling also affects module-relative paths; those executable
features remain outside the native scope.

Cli2 option objects merge from parent to child: rule-config objects shallow
merge by key, and other properties replace rather than concatenate. Thus
replacing `MD013` replaces its entire option object, not individual parameters.

Standalone `.markdownlint.*` rule configs replace inherited standalone rule
configs, rather than merge them. At the point of linting a selected standalone
config wins over cli2's `config`. The source's inheritance algorithm also prevents
inheriting a parent standalone config when an effective cli2 `config` exists;
do not implement these two configuration families as one generic deep merge.

Overrides apply only to files already selected for the relevant directory.
Filters resolve relative to that directory; the first matching override wins.
`combine: merge` shallow merges rule keys; `replace` replaces the rule config.
An applicable standalone config still wins over the override's cli2 `config`.

Examples to pin:

- `.markdownlint.jsonc` disables MD013 while `.markdownlint.json` enables it:
  the JSONC file wins.
- Parent standalone config sets `default: false, MD013: {line_length: 60}`;
  child standalone config sets only `MD013: {line_length: 40}`: the child does
  not inherit `default: false`, so other built-in rules return to their defaults.
- Parent cli2 `config` disables all defaults and sets MD013 to 60; child cli2
  `config` sets MD013 to 40: the `default: false` entry survives.
- Two override filters match a file: only the first applies, even when the second
  would produce a different width.

## Rule configuration and inline directives

Rule IDs, aliases, and tags match case-insensitively. Entries apply in object
iteration order, so a later tag or alias can override an earlier rule-specific
setting. Preserve declaration order through parsing and merging. An explicit
rule object replaces earlier parameters for that rule, not a deep merge.

`default` controls baseline enablement and severity. Supported rule values are
booleans, `error`, `warning`, or an option object with optional `enabled` and
`severity`. Common object defaults are `enabled: true`, `severity: error`.
Warnings remain warnings; do not invent an error-only model.

MD026 is enabled by default and must remove configured trailing heading
punctuation when enabled. This prescribed content edit is explicitly approved;
generic preservation must not suppress it. Respect custom/empty punctuation
sets, disabled rules, and upstream exceptions such as HTML entity semicolons.

Support inline `markdownlint-configure-file`, `enable-file`, `disable-file`,
`enable`, `disable`, `capture`, `restore`, `disable-line`, and
`disable-next-line`. Upstream applies configure-file and file-wide directives
before line-state directives, then line-specific exclusions. Preserve directives
and avoid edits spanning disabled/protected regions. `noInlineConfig` bypasses
all directive handling. Full parser-sensitive edge cases need runtime tests.

Rule-specific regular expressions need separate attention: `front_matter_title`
in MD001/MD025/MD041 and MD051 `ignored_pattern` use native JS RegExp upstream.
Built-in defaults are supported target behavior. Custom expressions
are planned for a later release with compatible semantics; until then, emit
`unsupported-rule-pattern`, naming the rule and parameter, rather than interpreting
them with a subtly different Rust regex engine.

For MD001/MD025/MD041, an omitted `front_matter_title` uses source fallback
`^\s*"?title"?\s*[:=]` with case-insensitive matching, accepting quoted title
keys. The schema default lacks optional quotes. `sourceDefault` and
`sourceFlags` in the inventory record this distinction; do not inject schema
defaults before applying source behavior.

For the first reflow milestone, an enabled MD013 supplies the prose wrapping
target. Eligible prose reflows even when upstream's final-token exception means
the original line has no diagnostic; exceptions govern remaining-violation
reporting, not arbitrary splitting of links or code. A disabled MD013 must not
trigger MD013-driven edits. The user has approved this width-based reflow policy;
lines exactly at the configured width are allowed. Exact paragraph packing and
joining of existing soft breaks remain for the formatting-foundation design.

## File selection

Support glob lists, `!` / `#` negation, `:` literal paths, stdin `-`, dotfiles,
forward-slash normalization, the special dot-only pattern, and `--` literals
according to the pinned CLI. Do not claim globby/micromatch parity simply because
a Rust glob crate accepts similar syntax. Symlinks, case sensitivity, escaping,
brace/extglob behavior, negation-only input, and files outside the invocation
tree need differential tests in the configuration milestone.

Top-level `globs` append to CLI patterns unless suppressed by `--no-globs`.
Top-level `ignores` become negative enumeration patterns. Nested `ignores` filter
after enumeration and inherit/reapply to descendant contents.

`gitignore: true` includes ignore files up to the Git root. A string selects
ignore-file globs; `**/.gitignore` is not equivalent because it does not add
ancestors above the invocation tree.

## Complete cli2 property inventory

“Supported” below means planned native compatibility, not current coverage.
Non-empty module arrays trigger errors; empty arrays are inert and acceptable.
`modulePaths` is likewise rejected only when non-empty.

### $schema — supported

Editor metadata only; it does not select or execute a schema at runtime.

### config — supported

Rule object, including declarative `extends`. Custom regex parameters have the
explicit deferred boundary described above.

### customRules — unsupported

Non-empty value: `unsupported-executable-feature`. Do not load Node rule modules.

### fix — supported

Preserve upstream's configuration ability to override the CLI fix default.
The future formatter CLI must define how write/check/stdin modes map to this;
do not silently override `fix: false`. CLI spelling remains a later decision.

### frontMatter — deferred

An explicit pattern requires native JavaScript RegExp semantics (`u` flag) in
cli2. Compatible custom patterns are a later-release roadmap item. Emit
`unsupported-front-matter-pattern` until compatibility exists. Without this
property, preserve upstream's default front-matter recognition and content.

### gitignore — supported

Boolean or ignore-file glob, effective only at invocation level.

### globs — supported

Append patterns at invocation level; preserve `--no-globs` behavior.

### ignores — supported

Enumeration-time top-level filtering and inherited nested filtering differ.

### markdownItPlugins — unsupported

Non-empty value: `unsupported-executable-feature`. Custom syntax cannot be
assumed equivalent to native Markdown parsing.

### modulePaths — unsupported

Non-empty value: `unsupported-executable-feature`; no Node module resolution.

### noBanner — supported

Invocation-level output suppression. A CLI with no banner already satisfies
suppression; it need not imitate upstream's branding.

### noInlineConfig — supported

Disables all inline configuration handling, including configure-file.

### noProgress — supported

Invocation-level progress suppression; formatter output must stay uncontaminated.

### outputFormatters — unsupported

Non-empty value: `unsupported-executable-feature`. Native diagnostic output is
separate from Node-based output formatter modules.

### overrides — supported

Ordered filter/config/combine entries, with first-match and shallow merge/replace
semantics. Resolve declarative `extends` relative to the containing config file.

### showFound — supported

Invocation-level found-file listing, only when progress is enabled. Keep listings
separate from formatted content on stdout in formatter/editor modes.

## Decisions before configuration implementation

- Track custom regex and installed-package `extends` as agreed later-release
  deliverables. Document first-release diagnostics without claiming complete
  declarative compatibility before those deliverables exist.
- Define write/check/stdin behavior with `fix`, and streams for `showFound`.
- Validate malformed-config behavior and unknown rule handling against upstream;
  do not silently accept unsupported settings.

[cli]: https://github.com/DavidAnson/markdownlint-cli2/blob/341e2a331ba578029e66c2e01183e7a6b5ca9ab4/markdownlint-cli2.mjs
[schema]: https://github.com/DavidAnson/markdownlint-cli2/blob/341e2a331ba578029e66c2e01183e7a6b5ca9ab4/schema/markdownlint-cli2-config-schema.json
[merge]: https://github.com/DavidAnson/markdownlint-cli2/blob/341e2a331ba578029e66c2e01183e7a6b5ca9ab4/merge-options.mjs
[library]: https://github.com/DavidAnson/markdownlint/blob/b67636d47f335cf124990faae4a72c5e9fadd613/lib/markdownlint.mjs
