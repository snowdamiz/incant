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
