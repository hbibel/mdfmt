# Release hardening

## Intent and baseline

Finish the roadmap's practical release evidence without converting finite tests
into a claim of universal Markdown, regex or filesystem parity. Keep the current
branch/worktree and do not publish, tag, merge or push.

The regex correction pass closed at `24b8500`. Processing reuse committed at
`283af75`, with review closure at `8b942f8`: both debug and optimized suites pass
577 tests across 78 targets, and independent review found no Important/Critical
defects. Historical baseline failures below were reproduced before those fixes.

Known obligations remain: the unchanged 4,000-item list and 500-paragraph debug
budgets previously failed, optional formatter transport is Unix-only, remote
platform jobs have not run locally, and broad corpus/fuzz/benchmark/binary
evidence is absent. These are not resolved by the current optimized suite.

## Separate deliverables

1. Restore unchanged debug resource budgets through measured processing reuse.
2. Add bounded Windows formatter transport with target-specific process evidence.
3. Add reproducible corpus, deterministic hostile-input and release-artifact gates.

Each deliverable owns a tested implementation and may receive one independent
review/fix pass. Do not rerun reviews of unrelated completed batches.

Processing reuse is complete. Run the available Linux corpus/artifact deliverable
next while Windows runtime evidence remains unavailable; this ordering does not
close or weaken the Windows implementation/evidence obligation.

## Processing reuse and resource evidence

Reproduce the existing debug failures first and profile native parse/configuration
construction on the same cases. Do not increase limits, switch those tests to
release-only, reduce input counts or call a timeout an inconclusive pass.

If the profile confirms repeated identical work, cache only exact immutable
processing snapshots: source bytes plus current configured metadata context,
parsed structure and DocumentConfig. Reuse within a processing invocation and
discard when bytes or context change. Do not cache by source length, hash alone,
filename, phase-insensitive convergence history or global mutable state.

The likely integration is one current snapshot per formatting pass, shared across
unchanged rule phases; changed output creates a fresh snapshot. Diagnostic recovery
may use a per-Source OnceLock when its result is a pure function of that Source.
Retain Source's Send/Sync behavior, source-coordinate mapping, protected-prefix
context and all fallible/resource-exhaustion behavior. Do not share mutable rule
permission state between independent documents.

Pin both performance and semantic controls: unchanged limits pass in debug and
release, all transformations produce the same source/findings, and interleaved
configs/metadata contexts cannot reuse stale parser or directive state. Count
actual native parses and directive constructions in controlled tests rather than
asserting cache internals or benchmark wall time alone.

## Windows transport

Keep fixed adapter argv, private cwd, explicit opt-in, complete MD013 permissions,
1 MiB input/stdout, 64 KiB stderr, 5s invocation/30s document execution and 100ms
cleanup recovery. Do not bring back uncancelable blocking pipe workers, temporary
output-file spooling or a shell wrapper to imitate portability.

Use native overlapped named-pipe I/O and Windows job containment. Parent operations
own stable buffers, OVERLAPPED storage and event handles until completion or
bounded cancellation. Input, output and error pipes must all make progress under
pressure; a blocked stdin cannot prevent reading output or observing deadlines.

Termination/cancellation/reaping are nonblocking under the same deadline/recovery
contract. If the OS cannot establish cleanup, retain bounded ownership instead of
freeing memory referenced by pending kernel I/O, leaking waiter threads or waiting
indefinitely. Keep at most 64 active/pending groups and refuse further launches
while containment remains uncertain. Unknown/unsupported transports still fail
closed before execution.

Reuse command-group's existing suspended-spawn/job containment where sound;
inspect its Windows lifecycle rather than assuming Unix try_wait semantics.
If its opaque job ownership prevents safe liveness verification after failed
termination, retain a conservative process-lifetime fault instead of inferring
that descendants died when the leader exited. State that recovery cost explicitly.

Windows process tests must exercise real pipe pressure, blocked stdin, leader exit
with live descendants, deadlines, partial/invalid/output-limit failures, process
cleanup and repeated formatting. Keep all Unix/seccomp regressions. Cross-compiling
alone is not runtime evidence; an emulator must be labeled as such.

## Corpus, fuzzing and benchmarks

Use pinned, licensed source fixtures with recorded provenance, not live downloads
inside application or routine tests. Combine standard Markdown examples and a
small real-document corpus. A maintainer refresh command may fetch data explicitly;
ordinary tests consume checked-in inputs only.

Require deterministic repeated output, exact preservation of code values/tables/
metadata/indivisible links outside approved repairs, valid original edit
reconstruction and no input writes on failure. Account explicitly for approved
MD026 heading edits and admitted MD051 case correction instead of weakening every
protected-content comparison.

Use seeded native property cases for delimiter/container/tab/BOM/newline/Unicode/
HTML/table/link interactions and hostile configuration patterns. Avoid a Node
oracle inside the hostile-input loop; pinned development dependencies have known
advisories and direct JS patterns can backtrack without native fuel bounds.
Time-bound any development-oracle subprocess separately.

Report debug and optimized benchmark distributions for prose, dense lists,
references, code and tables. Keep the existing small CI budgets as regression
gates; do not turn noisy microbenchmarks into exact nanosecond assertions. New
Important/Critical corpus/fuzz findings must enter the roadmap and receive
regression-first fixes, not disappear into a skip list.

## Safe writes, CI and binaries

Retain check/stdout no-write behavior, symlink and Unix hardlink rejection,
observed-change checks, ordinary permission preservation and transactional error
handling. Add end-to-end packaged-binary smoke tests for stdin, check, write,
config/package/metadata loading and missing formatter preservation. Do not claim
adversarial race protection, extended-attribute preservation or crash durability
unless implemented and tested explicitly.

CI must run native Linux/macOS/Windows tests appropriate to supported features,
bounded oracle jobs and reproducible release builds. Remote execution remains
unclaimed until actual receipts exist. Do not trigger a push or publication merely
to obtain a green badge.

Produce local release archives with the executable, usage/license/provenance
material and checksums. Test the extracted executable, not only cargo run. Verify
that artifacts contain no `.env`, development node_modules, build scratch or
machine-specific project paths. Build scripts must use the declared toolchain and
tracked Cargo.lock; publication is a separate human action.

## Builder decisions and remaining external evidence

1. Profile before adding snapshot/recovery caches. Cost if wrong: measured reuse
   may not fix the known debug limits, so another source-backed optimization is
   needed rather than a relaxed budget.
2. Use exact invocation-local snapshots and per-Source immutable recovery results.
   Cost if wrong: additional lifetime/ownership plumbing or conservative cache
   misses, not stale configuration or metadata edit authority.
3. Use overlapped Windows I/O with bounded retained ownership on failed cleanup.
   Cost if wrong: more native lifecycle code and platform-specific tests; do not
   substitute blocking transport to make compilation easier.
4. Keep corpus/property tests deterministic and offline after explicit fixture
   maintenance. Cost if wrong: narrower finite coverage, not network-dependent
   tests or a hidden application downloader.
5. Produce local artifacts without publishing or integration. Cost if wrong:
   release distribution still requires the human's explicit action.

This environment currently has only the Linux Rust target and no Windows runtime,
Wine, MinGW compiler, cargo-fuzz or cargo-audit executables. Recommended policy:
provision development-only tooling where authorized, otherwise finish available
native evidence and report the precise missing Windows runtime/remote receipts.
Do not label unavailable platform tests as passing or claim the entire roadmap
complete while a required implementation or genuine runtime blocker remains.

## Corpus/artifact implementation decisions

- Pin all 652 CommonMark 0.31.2 examples from the official spec JSON, SHA-256
  `d431b29d97b6f73e69d547109cf5081578fac931e72afe95639ebe766c1b2a20`.
  Retain original attribution/CC-BY-SA 4.0 terms as test-data provenance, separate
  from binary licensing. Include the pinned markdownlint 0.42.0 and regress 0.11.1
  READMEs as two licensed real-document inputs with hashes and license copies.
- Record protected source ranges using the pinned development micromark consumer.
  Native tests check no-edit overlap, exact original edit reconstruction, no-fix
  preservation and repeat stability. A bounded differential gate compares selected
  upstream structure/atomic source before and after MD013-only formatting at width
  12; disable default metadata and inline directives explicitly for this corpus
  contract. Existing metadata/directive and approved MD026/MD051 controls remain.
- Run 256 reproducible native seeded cases in a bounded subprocess, with mixed
  delimiters, containers, tabs, Unicode, newlines, HTML, links, tables and metadata.
  Include hostile-pattern error/recovery controls without a JS adversarial loop.
  This is seeded property fuzzing, not an unrun coverage-guided/libFuzzer campaign.
- Use a native Rust example to report five timed samples after warm-up for prose,
  dense lists, references, code and tables. Run it in debug and release; keep
  existing resource gates unchanged and make no universal throughput claim.
- Use Python standard-library release tooling only during development. Build the
  Rust CLI with the declared toolchain/locked dependencies and explicit path
  remapping into a private build directory; package only curated binary, usage,
  license/dependency notices, lockfile and normalized provenance. Emit deterministic
  ZIP metadata and SHA-256 sidecars, and smoke-test the extracted executable.
  Never read or package `.env`, node_modules, arbitrary worktree files or scratch.
- CI gains bounded corpus/property/oracle and native artifact checks, but no tag,
  release upload or publication action. Remote results remain unclaimed locally.

Costs if wrong: test-data licensing/provenance needs maintenance when pins change;
finite deterministic cases do not replace coverage-guided fuzzing; benchmark
samples are host-specific; curated notice collection is not a legal-compliance
certification; native-only local packages do not establish cross-platform runtime
evidence or authorize publication. Existing Node reference pins stay unchanged.
