# Optional fenced-code formatters

## Intent

Implement roadmap milestone 6 without turning the native Markdown formatter into
a Node wrapper. An available language formatter may change a supported fenced
body, but unavailable, failed, timed-out, unstable or unsafe output must leave
that body's original bytes intact. Remaining MD013 findings still describe the
final Markdown. Existing acceptance scenarios are contracts, not execution proof.

The user delegated reversible implementation choices and requested autonomous
completion. The choices below are builder decisions, not new user approvals.

## Invocation and scope

- Add CLI `--format-code`, off by default. Executing installed programs is explicit;
  existing commands and the pure library API retain their current behavior.
- Add a separate `format_with_code_formatters` API that documents process I/O.
  Keep existing `FormatOptions { fix }` and `ResolvedConfig` literals compatible.
- Both fix controls, enabled MD013, `code_blocks: true` and permission across the
  whole original fenced body are required before invoking a formatter. Inline
  directives inside literal code do not become configuration.
- Support explicit language aliases: `rust`/`rs` via rustfmt, `go`/`golang` via
  gofmt, `python`/`py` via Black, and `json` via Prettier's JSON parser. Unknown
  languages, unlabeled/indented/unclosed fences and extra info attributes remain
  unchanged. No implicit JavaScript, template or plugin execution is added.
- Use installed executables from PATH, with fixed argument vectors, never a shell,
  package download or installation. Programs on PATH are trusted executable code;
  this is process containment for reliability, not a security sandbox.
- Run in a private temporary directory. Rustfmt uses a generated explicit config;
  Black uses `--isolated`; Prettier uses `--no-config --no-editorconfig --parser json`.
  Project formatter configuration/plugins are not discovered. Gofmt has no config.

## Ownership and formatting contract

Only original native Code nodes with complete, unambiguous container ownership
qualify. Preserve the opening/closing fences, info string, every other code body,
tables, links, metadata and all bytes outside the selected body spans.

Extract logical code using the native Code value and exact physical container
prefixes. Reject uncertain tabs, partial/lazy prefixes or a source reconstruction
that does not match that logical value. Restore the complete quote/list prefix
and the source's newline convention on every returned line, including blanks.
Support root, quoted, listed and nested quote/list closed fences; conservative
rejection remains preferable to damaging ownership.

The available width is MD013's `code_block_line_length`, inheriting `line_length`,
minus the restored prefix's UTF-16 width. Pass it to rustfmt's `max_width`, Black's
`--line-length` and Prettier's `--print-width`. Gofmt has no width setting; do not
pretend it does. Zero/oversized available widths preserve code with a warning.
Formatting may leave indivisible lines over the limit; normal MD013 exceptions
still determine the final findings.

Require exit success, complete UTF-8 output without NUL bytes, nonempty output for
nonempty code, and exact same logical output on a second formatter invocation.
Normalize physical output newlines before comparing/restoring. Trust the selected
formatter for language semantics; idempotence is not a semantic proof.

Apply proposed body replacements only through a private validator. Reparse the
whole candidate and compare position-free ASTs, ignoring only the values of the
specifically selected original Code ordinals. Keep code language/meta and every
other node field exact. Fence-breaking output, added nodes or changed container
ownership is rejected. Do not relax generic protected-region/edit equivalence.

Run the code stage once after ordinary Markdown formatting settles, then settle
Markdown again for final diagnostics and safe surrounding edits. Compose all
edits back to original coordinates. A second public call must make no edits.
Retain formatter warnings at their final owning fence anchors, not stale offsets.

## Process limits and failures

Use the synchronous process-group API from pinned `command-group = 5.0.1`
without Tokio. Read stdout and stderr concurrently with bounded buffers; write
stdin concurrently so pipe pressure cannot evade the deadline. Kill and reap the
group on every failure/deadline, and clean up remaining group members on leader
exit before joining pipe workers. Never use unbounded `wait_with_output`.

Limits are 1 MiB input/body, 1 MiB stdout, 64 KiB stderr, 5 seconds per invocation,
30 seconds across a document and 64 eligible bodies per document. Both stability
invocations consume the same budget. Input widths above 4,096 are skipped rather
than passed to an unbounded adapter. Private runner tests may lower these limits.
Do not claim hard memory/CPU isolation or containment of deliberately escaped
descendants. Cross-platform runtime guarantees require later CI evidence.

Unavailable executables, nonzero status, invalid/partial/empty output, timeouts,
limits, instability and unsafe candidate Markdown produce a warning with
`rule_id: None`, anchored to the owning fence. Use `formatter-unavailable`,
`formatter-failed`, `formatter-timeout`, `formatter-output-limit`,
`formatter-unstable`, `formatter-unsafe` and `formatter-skipped` as applicable.
Warnings do not independently cause exit 1; final error findings/check changes
retain existing exit behavior. One failed body does not discard a successful
independent body or unrelated Markdown edits.

## Evidence required

- Real subprocess tests cover success, missing executable, partial nonzero output,
  both full pipes, invalid UTF-8, output overflow, blocked stdin, deadline cleanup,
  descendant-held pipes and a deliberately non-idempotent formatter.
- Exact argv tests cover each fixed language adapter and width/prefix behavior.
  Controlled fixture executables are process tests, not proof of real formatter
  compatibility. Available local rustfmt and gofmt get actual execution smoke
  tests; unavailable Black/Prettier remain explicitly unexecuted locally.
- Public API/CLI tests cover root/quote/list/nesting, directives/fix controls,
  BOM/CRLF/CR, literal code comments, adjacent failures/successes, unsafe fences,
  final MD013 findings, original-coordinate edits and second-call identity.
- Complete optimized tests and prior Markdown native gates must remain green.
  One independent batch review and one regression-first Important/Critical fix
  pass close this milestone; later regex/package compatibility and hardening
  remain unfinished.
