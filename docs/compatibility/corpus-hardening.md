# Offline corpus and native property evidence

This gate checks preservation and repeatability on licensed standard examples,
real documentation and deterministic mixed-syntax inputs. It does not establish
complete parser parity, lint fidelity or coverage-guided fuzzing results.

## Inputs and contracts

The corpus retains all 652 CommonMark 0.31.2 examples, plus the pinned markdownlint
0.42.0 and regress 0.11.1 READMEs. The manifest records source URLs, byte hashes,
licenses and attribution. CommonMark's CC-BY-SA test data remains separate from
binary contents. Git preserves raw fixture bytes across platforms.

Ordinary Rust tests use checked-in observations captured from the pinned upstream
micromark consumer. They require exact no-fix bytes, no overlap with upstream
code/table/link/image/HTML/definition/hard-break/entity/escape boundaries, original
edit reconstruction and a second call with no edits. Layout repairs cannot change
payload characters. The corpus contract enables only MD013 at width 12, disables
inline directives and explicitly selects no metadata, so standard examples are
not reinterpreted as application configuration or front matter.

The development differential gate runs the native CLI twice per input with bounded
subprocesses. It compares selected upstream semantic structure and atomic source,
normalizing only ASCII soft whitespace inside inline blocks. It retains word
separation around inline nodes and literal entity/escape payloads. Micromark's
intermediate content chunks and markdownlint's artificial undefined-reference
diagnostic copies are not rendered semantic blocks. Dedicated probe assertions
failed before those distinctions were corrected; those probe failures were not
formatter corruption findings.

The independent review exposed a separate authority error: upstream coordinates
refer to BOM-stripped, newline-normalized parser input, not original source bytes.
The observer now maps UTF-16 line/column positions to original UTF-8 bytes and
asserts that every atomic slice recovers the upstream token text after newline
normalization. Twelve checked-in controls combine BOM, LF/CRLF/CR, tabs, astral
text and lazy container continuation. The source probe and native consumer assert
literal link/code slices at original offsets before applying the range-overlap
gate. The original 654 receipts did not change.

The 654 native/source comparisons pass. The Rust gate also passes all 654 cases
with exact protected-range/edit/idempotence checks. No formatter production change
was needed for this corpus. Existing metadata/directive contracts remain covered
by their separate suites; this fixed corpus mode does not override normal users'
configuration.

## Seeded native properties

A thirty-second parent subprocess bounds 256 fixed-seed cases mixing delimiters,
quotes/lists, tabs, BOM, astral text, LF/CRLF/CR, HTML, links, code and tables.
Twenty-nine cases produce real formatting changes. They check stable repeated
output, original edit reconstruction, code values and table/link source, protected
metadata and valid diagnostic coordinates. A hostile regex must return a typed
work-limit failure, then an independent safe document must succeed.

These are deterministic property cases, not an unrun libFuzzer campaign. A separate
literal control confirms that approved MD026 punctuation removal and MD051 case
repair still produce `# Target` and `[case](#target)`; generic preservation cannot
waive those authorized edits.

## Benchmark observations

The native benchmark measures the pure library with MD013 strict width 40, one
warm-up and five independent calls per category. On Linux/Rust 1.99.1, the local
2026-11-17 run reported these median/p95 durations in microseconds:

- Prose: debug 384676/386027; optimized 43758/43990.
- Dense lists: debug 338677/340818; optimized 41648/42274.
- References: debug 138875/222328; optimized 18768/18961.
- Code: debug 62340/62411; optimized 6698/7017.
- Tables: debug 97817/113148; optimized 10450/10558.

The JSON output retains all five raw samples. Code/table inputs remain unchanged;
prose/list/reference inputs exercise real transformations. These finite host
observations are not throughput guarantees or process-startup measurements.
Existing time/memory regression gates remain unchanged.

## Reproduce

```sh
python3 tools/refresh-corpus.py
mise exec -- cargo test --test hardening_corpus --example benchmark
mise exec -- cargo test --release --test hardening_corpus --example benchmark
node tools/parser-oracle/corpus.mjs
mise exec -- cargo run --example benchmark
mise exec -- cargo run --release --example benchmark
```

Only explicit `python3 tools/refresh-corpus.py --fetch` downloads/copies pinned
inputs; source observations refresh with `corpus.mjs --record --source-only`.
Ordinary Rust tests need neither network access nor Node. Complete debug and
optimized verification each pass 582 tests across 80 targets, including the
benchmark's sample/statistics test and original-byte coordinate controls.
Fmt/clippy/diff checks are clean. The combined corpus/artifact review found four
Important tooling/evidence gaps with regression-first corrections; no formatter
production defect was established. Windows transport/runtime and remote CI
evidence remain open.
