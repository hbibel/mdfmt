# Installed shared configuration packages

## Intent

Complete the roadmap's installed-package `extends` item using native Rust local
resolution. Users may reference an already-installed declarative config without
copying its path into `node_modules`. Do not fetch/install packages, execute
JavaScript or require Node at runtime. Custom regex support remains a separate
unfinished item.

## Resolution contract

Use pinned markdownlint0.42.0/cli2 0.24.0 as the source reference. Their resolver
first tries the exact path relative to the referring config, then CommonJS
`require.resolve` semantics. Existing direct paths, including directories that
subsequently fail as config files, shadow a package with the same name. Home and
absolute paths retain existing handling.

For fallback resolution, use native `oxc_resolver = 11.24.3` behind a private
adapter instead of implementing another package/exports grammar. Configure only
ancestor `node_modules`, main/index and CommonJS extension order `.js`, `.json`,
`.node`; exports/imports and package self-reference retain applicable metadata
semantics. Enable default Node24 conditions `node`, `require`, `module-sync`,
`node-addons`, `default`, respecting metadata key order rather than imposing an
adapter priority. Disable NODE_PATH, Yarn PnP, aliases, browser fields, tsconfig,
custom roots and nonstandard directory exports.

The supported lookup is local to the referring config and its ancestors, not the
Node installation, global folders, registry, modulePaths or npm credentials.
This explicit local boundary avoids machine-dependent global config selection.
Node built-ins/URLs/executable targets are not declarative config sources.

Once a target is selected, existing supported config parsers and merge logic own
its contents. JSON/JSONC/YAML/YML/TOML files are allowed. Selected JavaScript,
native modules, extensionless and other unsupported config formats are rejected
explicitly, not executed or skipped in favor of another package candidate.
Relative `extends` inside a resolved package starts at that selected file;
installed-package symlinks follow Node's real target while direct filesystem
references retain existing lexical path semantics. Canonical identities still
detect cycles with the existing128-file inheritance limit.

Preserve standalone inherited-before-local key order and cli2's in-place local
key positions for `config` and every override. Rule aliases/tags, shallow
replacement, discovery and file selection must not change. Missing/unexported/
invalid package targets produce `invalid-config` on `extends`, naming the
reference and referring path; selected unsupported formats retain their existing
`unsupported-config-format` diagnostic with the selected target path. Failure
must preserve input files and exit2, never silently use defaults.

## Source and runtime evidence so far

Source review finds fallback `resolveModule` delegates to native require.resolve;
cli2 extends config/override objects in place. Direct temporary-package probes
using actual pinned readConfig establish main JSON entrypoints, insertion-ordered
default/require conditions, Node24 module-sync/node-addons conditions, subpath
patterns and invalid-main→index fallback. A valid first exports-array target that
names a missing file does not fall through to a later existing target; null
exports block legacy main. Exact local extensionless files shadow packages in
the upstream probe, but native parsing of extensionless configs remains an
explicit unsupported-format boundary, not an execution fallback.

These are upstream observations, not a native implementation. The chosen resolver
must pass a consuming native differential corpus before compatibility is claimed.

## Verification and scope

Public resolver/CLI tests and a pinned development-only Node oracle must cover
unscoped/scoped packages, nested ancestor precedence, explicit subpaths, main/
index, exports strings/conditions/order/patterns/null/arrays/invalid targets,
imports/self-reference, symlinks, relative nested extends, malformed metadata,
missing packages, direct shadows and cycles. Assert effective rule states/key
order and actual formatting outcomes, not just a selected path.

Prove executable selections never execute through marker-bearing JS/native
fixtures and unchanged inputs on errors. Keep prior config/directive/selection
and complete optimized suites green, then one independent review with one
regression-first Important/Critical fix pass. No claim of universal Node module
resolution, custom-regex completion or release hardening follows this batch.

## Builder decisions

1. Use a mature native resolver with explicit CommonJS options. Cost if wrong:
   extra dependency surface or source-backed adapter fixes, not a Node runtime.
2. Limit lookup to local installed packages/ancestor metadata. Cost if wrong:
   global/NODE_PATH/PnP-dependent setups receive errors instead of inherited
   machine-dependent configurations.
3. Preserve existing supported target formats after selection. Cost if wrong:
   packages whose entrypoint is extensionless or JS containing JSON remain
   explicitly unsupported rather than silently executed or reselected.
4. Keep metadata conditions aligned to pinned Node24 defaults, without reading
   arbitrary NODE_OPTIONS/custom conditions. Cost if wrong: custom-condition
   environments need a later explicit compatibility policy.
