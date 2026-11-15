# Optional fenced-code formatters

`--format-code` explicitly opts into installed rustfmt (`rust`/`rs`), gofmt
(`go`/`golang`), Black (`python`/`py`) and Prettier's JSON parser (`json`).
The ordinary CLI and existing pure library APIs never execute these programs.
The separate `format_with_code_formatters` API documents process I/O;
`CodeFormatOptions::default()` leaves execution disabled.
External execution currently requires Unix. Other platforms preserve code and
report `formatter-skipped` until a bounded transport is implemented and verified.

## Permission and ownership

Both fix controls must allow edits, MD013 must be enabled with code checking,
and the complete original native fence must have MD013 permission. Unlabeled,
unsupported, extra-info, indented, unclosed or uncertain/recovered-only fences
remain unchanged without execution. Conservative unmodeled-syntax guards remain.
Python-style comments are literal data; recognized HTML markdownlint directives
retain pinned semantics, including inside code strings, and can prevent edits.

The code stage restores complete root/list/quote prefixes and LF/CRLF/CR newlines.
Only selected code-body spans change; fences, info, other code and surrounding
Markdown stay exact within this stage. Ordinary enabled Markdown rules may still
change surrounding prose in the same invocation. A private candidate-tree check
ignores only selected Code values while retaining every other node field,
including language/meta/container ownership. Generic protected-region policy is
not relaxed. Fence-breaking output is rejected.

MD013's code width inherits its prose width when omitted. Restored prefix UTF-16
width is subtracted before supplying rustfmt/Black/Prettier width arguments.
Gofmt has no width option; remaining long lines still receive normal MD013
findings. Formatting runs below the limit too, but does not force unsafe wrapping.

## Process and failure contract

Commands use fixed argv and a private temporary working directory, never a shell,
download or package installation. Rustfmt receives an explicit generated config,
Black uses isolated mode and Prettier disables config/editorconfig and automatic
plugin search. A private cwd alone does not isolate Prettier2's installation-relative
plugin discovery; the explicit `--no-plugin-search` flag is required.
Installed programs and the inherited executable environment are trusted; this is
not a security sandbox or proof of language-semantic equivalence.

Input/stdout are bounded to1MiB, stderr to64KiB, execution to5s per invocation and
30s across the document, and selection to64 eligible bodies. Widths outside
1..=4096 are skipped. Owned nonblocking stdin/stdout/stderr handling covers full
pipes without reader/writer threads. Cleanup uses group termination and nonblocking
reaping with at most100ms recovery; it never performs blocking wait/pipe joins.
Denied termination returns `formatter-cleanup-failed`, preserving the body and
preventing later launches. Up to64 active/pending groups may be owned process-wide;
pending handles are reaped opportunistically on later calls, and no new execution
is allowed while any remains pending. The OS/owner may have to terminate a denied
live group. Deliberately escaped descendants, whole-pass parser resources and OS
scheduling latency are not hard-bound guarantees.

Only successful complete UTF-8, non-NUL, nonempty output that settles on a second
formatter invocation can be admitted. Unavailable/failed/timed-out/oversized/
unstable/unsafe output preserves the original body and adds a warning. Warnings
are anchored to final owning fences and do not independently produce exit1.
Other successful bodies and unrelated Markdown edits survive an independent
failure. Final ordinary findings, including MD013, remain authoritative.

## Evidence

Five runner/adapter tests failed on missing APIs before implementation; six
initial integration tests then failed on the missing opt-in API. There are now
six runner tests and nine integration tests, with 554 complete optimized tests
passing across 74 targets, fmt/clippy/diff clean. Cases include concurrent full
pipes, partial failures, missing programs, blocked stdin, descendant-held output,
invalid/empty/NUL/unstable/oversized output, production5s timeout, unsafe fences,
body/width caps, aliases, both fix controls, warning exits, physical newline/BOM
variants, root/list/quote/nesting, exact prefix-width argv and input payloads,
blank-line prefix restoration, table no-spawn controls,
original-coordinate edit reconstruction and repeated output/diagnostic stability.
The64-body case permits exactly128 stability subprocess invocations and preserves
the65th body with a warning.

Actual isolated rustfmt/gofmt smoke tests pass locally; the public API also runs
actual rustfmt. Black is absent locally and has fixed-argv/controlled native-process
coverage, not real compatibility evidence. Actual pinned Prettier2.8.8/3.6.2 now
run through the CLI with a neighboring side-effect plugin: exact JSON output,
four stability invocations and zero plugin executions pass for each version.
These packages are development-only fixtures, not native runtime dependencies or
a broad guarantee across untested formatter versions.

The single independent review found two Important defects and no Critical
findings/minors: Prettier2 loaded neighboring plugins, and ignored termination
errors led to blocking cleanup past deadlines. Both have RED-to-GREEN regressions.
The cleanup test uses real Linuxx86_64 seccomp denying SIGKILL for an ordinary
same-group child, an unfiltered outer safety deadline, exact preservation and a
no-further-launch check. No mocked kill or escaped descendant substitutes for it.
The complete post-fix optimized suite and code/reflow native gates pass; no second
review was dispatched.

Prior native gates pass29 reflow,288MD013,440MD026,1206MD003,220MD048,1662MD046,
516MD014,687MD005,1680MD007,1144MD051 and6305 reference comparisons. One aggregate
oracle command reached its900s shell limit after MD053; the final1771MD054 cases
passed when run separately. No application assertion or work budget was relaxed.

The independent adapter-batch review and its single fix pass are complete.
Coverage stays53 built-ins
with `complete:false`; later regex/package compatibility, broad corpus/fuzz/
benchmark/platform evidence and release hardening remain unfinished, including
non-Unix process transport. The pinned development Node oracle has5 audit
advisories (3 high/2 low), tracked separately in the roadmap; no native Rust
application exposure or silent baseline upgrade is inferred.

```sh
mise exec -- cargo test --release --all-targets --jobs 2
mise exec -- cargo run -- --format-code --check document.md
npm run test:code-formatters --prefix tools/parser-oracle
```
