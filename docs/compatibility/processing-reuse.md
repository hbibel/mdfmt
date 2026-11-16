# Processing reuse and resource evidence

Formatting reuses immutable work within one pass without changing rule ownership
or the pure/process-I/O split.

## Lifetime and invalidation

One invocation-local context holds the current Source, parsed document and inline
rule configuration. Complete byte equality is required for reuse; a same-length
edit still rebuilds the context. Configuration is fixed for that context's
lifetime. Failed replacement construction returns an error without installing
partial state. Convergence histories do not retain parsed snapshots.

The same snapshot memoizes its normalization-authorization proposals. Fix flags
remain checked outside that memoized result. A module runs only when at least one
of its owning rules is enabled somewhere in the effective document, including
inline enables. Configuration validation and enabled fallible pattern consumers
still run; skipping disabled work is not permission to accept invalid settings.

Each Source separately caches one immutable recovered diagnostic tree, including
a failed recovery. Consumers share read-only handles; code needing an owned mutable
tree clones explicitly. New, derived and mapped Sources start with fresh caches.
Internal metadata-context setters invalidate the old result. Source remains
Send/Sync; mutable document permissions and pattern caches are not global.

Custom metadata's protected boundary and recognition-stability checks remain
authoritative across unchanged and changed snapshots. Recovery views keep their
own mapped coordinates rather than borrowing another source's cached authority.

## Measured regressions

Before this batch, the unchanged 4,000-item list resource test exceeded its
30-second subprocess budget under the existing 128 MiB virtual-memory limit.
The debug prose gate passed 100 paragraphs in 5.8 seconds, then timed out at 500
paragraphs under its ten-second limit.

Temporary instrumentation measured 169 native parses and 42 directive contexts
for a 37.7-second 500-paragraph invocation. The initial snapshot/recovery reuse
halved the small prose case but still missed the larger budget. A second profile
showed disabled reference/inline/separator modules rebuilding views; effective-rule
gates removed that remaining work. Instrumentation was removed before verification.
An after-change count on the same 500-paragraph input records six native parses,
three directive contexts and two diagnostic recoveries, down from 169/42/10.
That throwaway instrumentation was also removed before committing.

The original list test now passes all three fix/diagnostic controls in 7.2 seconds.
The unchanged debug prose gate passes 100/500/1,000 paragraphs in approximately
0.33/1.93/4.43 seconds, with exact expected output. These are finite single-host
smoke-test observations, not benchmark distributions or universal throughput bounds.

Three consumer tests cover same-length structural/configuration/directive changes,
interleaved metadata/default/no-match contexts, BOM/CRLF/tabs/astral positions,
thread-safety, stable repeated findings and literal original diagnostic spans.
Both complete debug and optimized suites pass 577 tests across 78 targets.
Optimized prose scaling passes 100/500/1,000 paragraphs in 37/206/495 ms.
The 280 metadata, 82 rule-pattern, 56 generated-property, 110 config, 554 content,
1,144 fragment, 29 reflow and 1,662 literal-style native comparisons pass.
Independent review found no Important or Critical defects. It independently passed
69 focused optimized tests, the unchanged debug list gate, all debug prose scaling
sizes and 15 ownership/inline-enable probes. It inspected cache invalidation,
read-only recovery, reentrancy, lifetime and rule mappings; the complete suites
remain implementer receipts, not a second independent full-suite rebuild.
Release hardening is not complete.

```sh
mise exec -- cargo test --all-targets --jobs 2
mise exec -- cargo test --release --all-targets --jobs 2
node tools/parser-oracle/reflow-scaling.mjs
node tools/parser-oracle/reflow-scaling.mjs --release
```
