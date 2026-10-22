# Vendored native regex engine

Source: `LubuSeb/picomatch-mortis`, revision
`d0684188b1c39275f3caaab578bd94a9c0c5bc25`, directory
`vendor/regress-0.11.1`. The upstream MIT/Apache licenses and original patched-engine
notes are retained. Only the engine is used, not that project's glob compiler.

The original `src/classicalbacktrack.rs` SHA-256 is
`b05171a5d82b5b0f734a35e6d3343e48bbcba8eae048f082200592230582308b`.

mdfmt adds one engine correction: save a loop's prior state before resetting its
iteration count on `EnterLoop`. Otherwise entering a nested loop destroys state
needed when an enclosing loop backtracks, silently returning false nonmatches.
`nested_loop_entry_restores_prior_iteration_state_on_backtracking` pins both
ordinary and UTF-16 execution. No optimizer setting could avoid the original bug.

The import copied source, tests, manifests, licenses and documentation unchanged
before applying that correction and its regression test. The original source-file
SHA-256s were recorded during import; no Cargo cache files were modified.

Run its tests independently:

```sh
mise exec -- cargo test --manifest-path vendor/regress/Cargo.toml --features prohibit-unsafe,utf16
```
