# Configuration implementation evidence

The native loader implements part of roadmap milestone 3. No rule fixes, lint
findings, MD013 reflow, or MD026 edits ship with it. The broader
[configuration contract](configuration.md) remains the target.

## Implemented

- Automatic JSONC/JSON/YAML discovery in the documented priority order, beginning
  at the invocation directory rather than searching arbitrary ancestors.
- Explicit JSON/JSONC/YAML/TOML configuration and JSON Pointer selection.
- Filesystem and home-relative `extends`, local-file precedence, shallow merging,
  cycle detection, standalone replacement, and separate cli2 inheritance.
- Case-insensitive built-in IDs, aliases and tags, ordered application, option
  objects, pinned schema constraints, enablement and warning/error severity without
  injecting schema defaults. Cli2's in-place extends ordering differs from
  standalone extends and is tested separately.
- First-match merge/replace overrides; standalone configuration still wins.
- All documented inline actions, including multiline configure-file, capture/
  restore, file-wide and line-specific passes, and `noInlineConfig`. Following
  upstream, scanning includes directive-looking comments in code and front matter.
- File/directory inputs, `*`, `**`, `?`, character classes, comma braces, dotfiles,
  `!`/`#` exclusions, `:` and `--` literal paths, dot-only selection, configuration
  globs, `--no-globs`, nested ignores, and gitignore files.
- Numeric/alphabetic brace ranges, including descending, padded and stepped
  enumeration; all fourteen upstream ASCII POSIX classes; repeated negation;
  and UTF-16 wildcard accounting for BMP and non-BMP names. Enumeration and
  override filters retain their distinct pinned range/class/negation semantics.
- Exact-one `@(a|b)` extglobs, including nested/empty alternatives, comma braces,
  ordinary character classes, Unicode and escaped literals. These work in file
  globs, exclusions, configuration globs, nested ignores and override filters,
  subject to the interaction limits below.
- Explicit errors for selected JavaScript configs, non-empty executable module
  settings, deferred custom regexes and installed-package resolution. Empty module
  arrays are inert. Errors name the configuration path and property.
- Effective fix permission and directive validation in the library/CLI. Stdout
  remains document-only; `showFound` uses stderr and respects `noProgress`.

The filesystem-free API is `format_with_config`. `ConfigResolver` performs explicit
filesystem loading; `ResolvedConfig::document` exposes per-line rule state and
conservative edit permissions for future rule implementations. Existing `format`
uses default rule configuration and still honors inline directives.

## Runtime comparisons

The development oracle uses the locked markdownlint-cli2 0.24.0 and markdownlint
0.42.0 packages. It runs actual cli2 discovery, inheritance, overrides and file
filtering, observing the boundary at which cli2 calls the real lint function.
Fixes are disabled so the oracle cannot rewrite fixture documents. For inline
state and parameters it executes the exact effective-config and per-line-state
functions extracted from the pinned library source, with the library's real
registry, helpers and configuration parsers. This is narrower than comparing all
built-in rule diagnostics.

Sixty-seven shared scenarios in `tests/fixtures/configuration/cases.json` produce
recorded `observations.json`. Rust tests reconstruct the same trees: 62 compare
selected paths and probed rule enablement, severity and parameters, while five
require explicit native errors for observed unsupported interactions. Coverage
includes discovery precedence, shallow cli2 inheritance, standalone replacement,
suppressed parent standalone config, explicit TOML/Pointer precedence, extends/
first override, outside-tree input, braces/negation/dotfiles, nested ignores,
ancestor gitignore, raw inline actions, multiline configure-file and noInlineConfig.
Review regressions additionally cover `./` prefixes, wildcard directory expansion,
trailing directory exclusions, positive reinclusion, enumeration versus override
character classes, cli2 extends order, sibling-tree gitignore scope and JavaScript
directive whitespace.
Further regressions pin nonrecursive `*`, extension-matching directories,
deduplicated configuration globs, ignored-parent whitelist rules, TOML inline
configuration, parent-relative comma braces, literal bracket alternatives, generic
filename classification and rule-schema metadata.
An additional 83 pattern cases in `glob-cases.json` record enumeration matching
and micromatch filtering over a shared set of paths, including all POSIX classes,
range-filter quirks, Unicode classes, and literal-marker collision regressions.
Supported contexts are compared natively; named unsupported contexts must return
`unsupported-glob` instead of approximating their recorded upstream results.
The first 30 configuration observations remain unchanged.
Review regressions also pin literal class operators, enclosing braces around
nested ranges, and complemented exclusions below the invocation directory.

From the repository root, install only the development oracle dependencies:

```sh
npm ci --ignore-scripts --prefix tools/parser-oracle
node tools/parser-oracle/configuration.mjs
node tools/parser-oracle/rule-options.mjs
node tools/parser-oracle/globs.mjs
mise exec -- cargo test --test config_oracle
```

An existing installation can instead be supplied with `--modules /path/to/node_modules`.
`--record` regenerates observations; normal execution compares without updating
them. Node is not a formatter build or runtime dependency.

`rule-options.mjs` checks the compact native option-constraint snapshot against the
same pinned upstream schema. It does not inject defaults into effective options.

The pinned development dependencies currently produce eight npm audit advisories
(five high, three low), including nested-pattern denial of service in `braces`
and a KaTeX trust/prototype-pollution advisory. They are not Rust runtime
dependencies. Keep oracle inputs local and trusted; changing reference versions
or applying `npm audit fix --force` would invalidate the compatibility baseline.

## Remaining boundaries

Milestone 3 is not complete. In particular:

- Repetition/optional `+(...)`, `*(...)`, `?(...)` and negative `!(...)` extglobs
  return `unsupported-glob`. Exact-one extglobs reject globstars inside the group,
  combined POSIX classes, plain parentheses inside groups, overlapping braces,
  alternatives inside braces, leading unescaped question marks, and post-group
  `?`/`+` quantifiers. Enumeration also rejects path separators inside
  groups: upstream's directory pruning differs from whole-path regex matching.
  Filters reject brace ranges combined with extglobs because upstream backtracking
  can rewrite other alternatives. Extglob nesting is limited to 64 groups.
  Literal text suffixes immediately after filter ranges also return
  `unsupported-glob`. Unknown POSIX classes and ambiguous double-hyphen POSIX
  classes are rejected explicitly. More complex
  escaping, nested range/filter interactions, JavaScript regex backtracking quirks,
  and case-insensitive filesystems need broader differential coverage before
  claiming globby/micromatch parity. Current runtime evidence is Linux-only.
- Patterns exceeding the upstream 65,536 UTF-16-unit limit are rejected. Numeric
  endpoints must fit JavaScript's safe-integer range, individual expanded ranges
  are limited to 1,000 values, and aggregate native range expansion has an
  additional conservative allocation budget. Exceeding a limit is an error, not
  an empty or partial selection.
- Native enumeration is deterministic and terminates symlink cycles. It is not
  the upstream parallel enumeration algorithm. Missing named file inputs retain
  the foundation's operational-error behavior instead of silently selecting no file.
- `--` makes remaining filenames literal, preserving the foundation contract.
- Gitignore support covers tested local files and ancestor lookup to the Git root;
  sibling target trees do not add their own ignore-file scope.
  Git configuration include chains and broad custom ignore-file discovery are not
  established as equivalent to the pinned globby implementation.
- Invalid or unknown configuration is rejected rather than coerced or silently
  ignored. Exact malformed-input and parser-coercion parity remains deferred.
  Missing JSON Pointer targets are errors, not empty-default substitutions.
- Custom JavaScript regex semantics and installed-package `extends` remain the
  agreed later-release items; configuration code never executes JavaScript.
- Configuration resolution is cached for one invocation. This is not a watch mode
  and does not reload files changed during processing.
- A selection/configuration failure prevents processing the invocation. Per-file
  document/I/O failures still aggregate while later files are processed.

The configuration tests establish configuration behavior, not lint cleanliness or
the formatting expectations in `acceptance.json`.

## Verification outcome

On 2026-10-22, all 54 Rust tests passed, including existing preservation and safe
replacement tests. Rustfmt, Clippy with warnings denied and `git diff --check`
passed. The locked development suite verified 22 parser observations, 30
configuration/selection scenarios and 53 rule option schemas. Independent review
findings gained regressions and fixes; its final targeted follow-up reported no
remaining blockers in those fixes, without claiming exhaustive compatibility.

The Task 4 continuation passes 60 Rust tests, rustfmt, Clippy with warnings denied,
and `git diff --check`. The pinned development suite reproduces 22 parser
observations, 50 configuration/selection scenarios, 53 option schemas, and 41
enumeration/filter pattern comparisons. Independent review found three selection
differences; each gained a failing upstream-backed regression before its fix.
This is a verified partial milestone, not complete glob or formatter coverage.

The exact-one extglob continuation passes 65 Rust tests, rustfmt, Clippy with
warnings denied, and the pinned development suite (22 parser observations, 67
configuration scenarios, 53 schemas, 83 direct pattern observations). Runtime
comparisons exposed POSIX backtracking, globstar and cross-segment traversal
differences; these interactions are explicitly rejected, not claimed compatible.
Independent review found four additional selection differences. Regressions failed
before fixing grouped bangs and manufactured globstars, and before diagnosing
post-group quantifiers and leading question marks. All four regressions now pass.
