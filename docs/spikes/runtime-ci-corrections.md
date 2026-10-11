# Native runtime CI corrections

The first PR #62 run at `d8d00c3` revealed two real integration defects. Linux
Wasm conformance job `114357530649` attempted to parse Cargo download progress
merged with metadata JSON. The probe now separates stdout and stderr and rejects
a nonzero process before parsing its stdout. Three actual child-process
regressions cover progress/warnings, nonzero exit with valid JSON, and JSON on the
wrong channel. The complete combined static/development conformance helper also
passes, including the compiled Rust guest; runtime and C sources are unchanged.

Linux physics job `114357529847` rejected the stale isolated lockfile under
`--locked`. Refreshing that lock exposed an unnecessary ULID random-generator
dependency in the new pure sprite geometry crate. A real wasm32 compilation
reproduced getrandom's unsupported-target failure. That crate uses only ID
parsing, formatting and deterministic constructors, so it now disables ULID's
default features. The isolated lock adds only incant_2d, its runtime edge and the
already pinned ULID/web-time packages. No unrelated dependency versions change.

After independent source review, locked wasm32 compilation, six native parity
streams (three repetitions each with one/four workers), affected runtime release
tests, GPU-player all-target release Clippy, 43 Python tools, formatting,
conventions and actionlint pass. The local parity run did not launch a browser;
the hosted Linux job retains its real browser execution and cross-host comparison.
No CI coverage or shipping audit was removed.

[Source identities and local validation](evidence/runtime-ci-corrections-2026-10-10.json).

## Follow-up Linux and Windows failures

The corrected dependency run passed native physics on Linux, including the
actual browser execution. Wasm then reached a strict-C11 Linux trap-link failure:
[reproduction and POSIX declaration fix](wasm2c-linux-c11.md). Root integration
transplants exactly those seven build lines into its pre-MSVC build script; the
combined static/development helper passes all 25 tests, Clippy and the real
Rust guest probe again. No trap/vendor semantics change.

The Windows editor job exposed one test attempting six independent Camera
accessibility scans under a single five-second deadline. It now uses six
separately named cases with the same fixtures, rules, assertions and per-case
deadline. Teardown drains the entire pending mount/scan before another fixture
can replace the DOM. The isolated forced-timeout check confirms subsequent cases
remain valid; 529 full UI tests and strict TypeScript pass, and root repeats all
59 Camera cases. [Exact test evidence](evidence/editor-camera-scan-isolation-2026-10-11.json).

Root conformance log: `/tmp/incant-pr62-posix-conformance.log`; Camera log:
`/tmp/incant-pr62-camera-final.log`. Full hosted verification remains required.
