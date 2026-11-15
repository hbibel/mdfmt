# Optional fenced-code formatters

`--format-code` explicitly opts into installed rustfmt (`rust`/`rs`), gofmt
(`go`/`golang`), Black (`python`/`py`) and Prettier's JSON parser (`json`).
The ordinary CLI and existing pure library APIs never execute these programs.
The separate `format_with_code_formatters` API documents process I/O;
`CodeFormatOptions::default()` leaves execution disabled.

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
Black uses isolated mode and Prettier disables config/editorconfig discovery.
Installed programs and the inherited executable environment are trusted; this is
not a security sandbox or proof of language-semantic equivalence.

Input/stdout are bounded to1MiB, stderr to64KiB, execution to5s per invocation and
30s across the document, and selection to64 eligible bodies. Widths outside
1..=4096 are skipped. Concurrent stdin/stdout/stderr handling covers full pipes;
process-group cleanup precedes pipe-worker joins, including when the leader exits
while descendants hold pipes. Deliberately escaped descendants and whole-pass
parser CPU/memory bounds are not claimed.

Only successful complete UTF-8, non-NUL, nonempty output that settles on a second
formatter invocation can be admitted. Unavailable/failed/timed-out/oversized/
unstable/unsafe output preserves the original body and adds a warning. Warnings
are anchored to final owning fences and do not independently produce exit1.
Other successful bodies and unrelated Markdown edits survive an independent
failure. Final ordinary findings, including MD013, remain authoritative.

## Evidence

Five runner/adapter tests failed on missing APIs before implementation; six
initial integration tests then failed on the missing opt-in API. There are now
five runner tests and nine integration tests, with 553 complete optimized tests
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
actual rustfmt. Black and Prettier are absent locally. Their fixed argv and the
CLI process contract are tested with controlled native fixture executables, not
claimed real Black/Prettier compatibility evidence.

Prior native gates pass29 reflow,288MD013,440MD026,1206MD003,220MD048,1662MD046,
516MD014,687MD005,1680MD007,1144MD051 and6305 reference comparisons. One aggregate
oracle command reached its900s shell limit after MD053; the final1771MD054 cases
passed when run separately. No application assertion or work budget was relaxed.

The independent adapter-batch review remains pending. Coverage stays53 built-ins
with `complete:false`; later regex/package compatibility, broad corpus/fuzz/
benchmark/platform evidence and release hardening remain unfinished.

```sh
mise exec -- cargo test --release --all-targets --jobs 2
mise exec -- cargo run -- --format-code --check document.md
```
