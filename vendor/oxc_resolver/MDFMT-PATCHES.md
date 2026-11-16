# Native config-resolution patches

Source: crates.io `oxc_resolver` 11.24.3, copied without changing registry files.
Archive SHA-256: `2c7fd0839a5ecee17c0d2c88c8006e8ab4dff25d4a79e3a0bf7a9f0cce17f4e3`.
The MIT license is retained from upstream tag `v11.24.3` (the crate archive omits
the license file). No upstream fixture suite or benchmark dependencies are used
as application verification.

Local changes:

- `ResolveOptions::package_self` defaults to true; mdfmt sets it false and uses
  empty `imports_fields`. Pinned markdownlint's captured `require.resolve` uses
  config directories as extra lookup roots, not as its own parent-module scope.
- Ignore top-level null exports fields for legacy main/index selection; a null
  subpath remains blocked.
- Validate array-index keys when a conditional target object is actually visited.
  Unselected conditions/subpaths remain unvisited. Propagate this hard error out
  of target arrays rather than falling back to an existing file.
- Preserve terminal null in conditional targets while allowing array null/empty
  fallback. Retain prior invalid/null results across subsequent undefined entries;
  leave all-undefined arrays undefined. Reject primitive targets instead of
  treating them as undefined.

These corrections follow Node24's `internal/modules/esm/resolve` target algorithm
and are exercised through actual pinned markdownlint0.42.0/cli2 0.24.0 observations
in `tests/config_packages.rs` and `tools/parser-oracle/config-packages.mjs`.
The application still has intentional local-lookup and selected-format limits;
this is not a claim of complete Node module resolution.
