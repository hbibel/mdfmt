# Native artifact checks

Development tooling builds a host-native Rust CLI, curated ZIP and SHA-256 sidecar.
Nothing is tagged, pushed, uploaded or published.

```sh
python3 tools/release.py --output dist --build-dir target/native-release
python3 -m unittest discover -s tools -p test_release.py
```

The packager invokes the declared mise toolchain and `cargo build --release
--locked --target HOST`. It uses a separate build directory, explicit path
remapping and stripped debug information. Custom RUSTFLAGS are rejected rather
than silently ignored. Python and mise are needed only to build/test the package;
the extracted binary runs without Rust, Python or Node.

## Contents and provenance

Each archive contains exactly seven files:

- Native executable with Unix executable-mode metadata.
- README with direct binary usage and preserved compatibility caveats.
- Project MIT license, matching Cargo's existing license declaration.
- Tracked Cargo.lock.
- BUILD-INFO.json with version, host, compiler, source identity, production-input
  fingerprint, recipe hash, package inventory and payload hashes.
- Combined locked-package and generated RE2JS/Unicode notices.
- The installed toolchain's Rust-library notice document.

The Linux normal/build dependency closure contains 138 packages including mdfmt.
Two registry packages omit license text; version-specific supplements retain
copyrights from their published VCS revisions. Missing unknown notices fail
explicitly. Notice collection is not a legal-compliance certification.
Project and vendored-package notices use explicit file allowlists. Unrelated
notice-like files in the worktree are not release inputs.

Input reads reject environment-file path components, including resolved aliases.
A regression exposed the first provenance inventory bypassing that guard; the
shared guarded reader now covers source hashing as well as notices/payloads.
The regression uses a literal placeholder beneath an environment-named directory,
not a real secret file. No `.env` file was read. Curated payloads exclude arbitrary
worktree files, dependency directories, fixtures and build scratch. Known original
project/cargo/build/toolchain paths are checked before archive replacement.

Archive members are sorted and use fixed metadata. SOURCE_DATE_EPOCH selects the
timestamp; otherwise the source commit's timestamp is used. Repeated packages
from the same native build and inputs have identical ZIP bytes and sidecars.
This is not proof of identical binaries across independent build directories,
hosts or toolchains. A source fingerprint and dirty-input flag do not provide
hermetic-build attestation. The production fingerprint inventories all files under
the three local source trees, their manifests/lockfile, embedded rule inventory,
toolchain declaration and any local build scripts/Cargo configuration. Dirty
detection covers these inputs as well as curated release material.

Build commands run in a fresh Unix process group or a Windows kill-on-close job.
The Windows bootstrap joins its job before starting any build tool; inability to
establish containment fails closed. Temporary build-log files avoid waiting for
inherited pipe handles, and timeout cleanup has a one-second leader-reap budget.
These are development-tool mechanisms, not the formatter's optional-code transport
or a sandbox against malicious build tools. Native Windows execution is still
unavailable locally.

## Verified behavior and limits

Seven Python tests cover actual builds, extracted executables and build deadlines. They
check curated members, hashes, repeated archive bytes, missing project paths,
stdin/stdout/check/write behavior, unchanged second writes, local declarative
package configuration, custom metadata, transactional invalid-pattern errors,
absent formatter/code retention, Unix modes/link-write rejection, and the local
build-helper deadline and descendant termination with explicitly inherited
stdout/stderr, private-notice exclusion, changed vendored/embedded-input provenance
and command output/failure capture. Windows deadline behavior has not run locally.

Native Linux archive creation, extraction, `--version` and stdin formatting pass
locally. It is a host-native GNU/Linux artifact requiring compatible OS/libc, not
a universal static Linux binary. Existing safe-write limits remain: no claim of
adversarial race protection, extended-metadata preservation or crash durability.

The single combined corpus/artifact review found four Important gaps and no
Critical/minor findings. Privacy, known-input provenance, upstream original-byte
coordinates and build-descendant cleanup now have RED→GREEN controls. Complete
debug/release verification each passes 582 tests across 80 targets; all seven
Python tests pass. No application runtime dependency or formatter production edit
was added.

CI adds explicit job deadlines, expanded portable Rust corpus/context checks,
bounded corpus comparison and native package tests on Linux/macOS/Windows. These
workflow changes are parsed locally; remote execution is not claimed. Windows
formatter transport/runtime remains separate and incomplete.
