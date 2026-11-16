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
`.node`. Preserve applicable exports metadata, but do not infer referrer-local
imports/self-reference: markdownlint captures its own library require and merely
adds config directories to lookup paths. Enable default Node24 conditions
`node`, `require`, `module-sync`,
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
names a missing file does not fall through to a later existing target; a null
exported subpath blocks access, while a top-level null exports field permits
legacy main/index lookup. Exact local extensionless files shadow packages in
the upstream probe, but native parsing of extensionless configs remains an
explicit unsupported-format boundary, not an execution fallback.

The pre-review native consumer passed 65 source-backed cases, including selected and
unselected numeric-key targets, arrays and terminal conditional null. The resolver
is vendored with documented corrections, not a filesystem view that rewrites
metadata or a replacement package grammar. The single independent review then
found six Important selection defects; consuming regressions expanded the corpus
to 102 cases and pass after the authorized correction pass. Complete post-fix
verification passes 561 tests across 75 targets, with 54 focused debug config tests
and clean fmt/clippy/diff checks. No second review was dispatched.

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
5. Keep home expansion in the private adapter so exact native paths never need
   lossy UTF-8 conversion. Cost if wrong: only the private interface differs from
   the initially planned already-expanded argument.
6. Vendor the pinned resolver with explicit Node corrections instead of rewriting
   metadata through a filesystem shim. Cost if wrong: additional third-party
   source and patch maintenance, not hidden changes to package identity.
7. Disable config-local imports and self-reference. Cost if wrong: generic
   caller-scoped require users cannot assume those semantics; this targets
   pinned markdownlint's captured library scope.
8. Keep exports exact-file selection separate from legacy file/directory fallback
   and remove obsolete folder mappings. Cost if wrong: legacy folder-map packages
   fail instead of exposing subpaths that pinned Node24 blocks.
9. Validate raw encoded segments before normalization and preserve hard versus
   fallback target errors. Cost if wrong: unusual invalid targets may fail closed;
   an invalid first array entry must never silently win over valid later rules.
10. Use standard WHATWG file-URL conversion only for exports; keep CommonJS
    identifiers literal. Cost if wrong: untested URL edge cases need source-backed
    corrections, not a global percent/query rewrite of existing native paths.
11. Require object package metadata and legacy file-before-directory precedence.
    Cost if wrong: malformed metadata is rejected and manual root module files can
    shadow package directories, matching pinned selection instead of convenience.
12. Keep non-Linux/big-endian runtime claims unestablished, as the reviewer set
    aside. Cost if wrong: platform-specific filesystem behavior remains hardening
    work despite both metadata backends receiving source corrections.
13. Keep global/install/modulePaths/NODE_PATH/PnP/custom-condition/config-local-scope
    exclusions, as the reviewer set aside. Cost if wrong: those environments receive
    explicit errors rather than machine-dependent inherited rules.
14. Keep executable and extensionless config targets unsupported, as the reviewer
    set aside. Cost if wrong: content-parseable targets still error rather than
    being executed, reinterpreted or reselected.
15. Leave custom regexes, non-Unix formatter transport and prior debug throughput
    obligations to their remaining batches, as the reviewer set aside. Cost if
    wrong: the package feature does not establish roadmap or release completion.
16. Leave universal hostile-input/dependency-security/race/resource guarantees
    unclaimed, as the reviewer set aside. Cost if wrong: those input classes remain
    hardening risks, not guarantees inferred from a focused compatibility corpus.
17. Retain explicit fail-closed boundaries for unknown hash condition names,
    unnecessary malformed nested metadata and unused overflowing numbers, as the
    reviewer set aside. Cost if wrong: uncommon otherwise-loadable packages error;
    no demonstrated wrong-rule selection or write is accepted as a mere nit.
18. Preserve named/provenance-bearing errors without exact upstream wording or
    every parser coercion, as the reviewer set aside. Cost if wrong: users see
    different messages or conservative errors, not silent defaults.
