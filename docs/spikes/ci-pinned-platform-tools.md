# Pinned platform tool reuse

The platform workflow installs `wasm-bindgen-cli 0.2.126` and `cargo-ndk 4.1.2`
on every run. Rust dependency caching deliberately excludes Cargo's bin
directory, and `cargo install` uses a separate temporary build directory.
Successful PR #49 installs cost 81 seconds (web) and 28 seconds (Android);
the main #49 seed cost 85 and 22 seconds. Those are measured installation
costs, not a claim about the still-pending hosted tool-cache speedup.

The follow-up retains the pinned source install with `--locked` and an exact
`=version`. It caches only the selected executable in an isolated runner-temp
directory. Keys separate tool/version, Linux distribution/version, architecture
and pinned Rust toolchain input. No prefix fallback, Cargo bin directory, rustup
proxy, install metadata, intermediate compiler output or companion binary is
included. The unrelated Rust dependency cache remains unchanged.

Every restored executable must report exactly its pinned `--version` before its
directory is added to later steps' PATH. A missing, unreadable or wrong-version
file triggers a normal source installation in temporary storage. The newly
built executable is checked again and atomically published. A failed install or
bad version fails the step without publishing a PATH. The cargo-ndk version
check invokes the exact file with Cargo's required invocation marker; a different
copy elsewhere on PATH cannot satisfy the check.

Only successful default-branch platform jobs save, and only when their exact
key is absent. PRs restore only. The existing main nightly or one separately
coordinated main run can seed the small files; this change does not add a new
schedule or dispatch. A corrupt existing immutable key will use the install
fallback and needs a reviewed key-version bump to replace it.

## Verification and budget

- All 35 Python tool tests and workflow actionlint pass. The seven new cases
  exercise reuse, exact locked installation, wrong/unexecutable cached files,
  failed/wrong-version installation, host/architecture separation and failure
  before PATH publication.
- Actual source installs and immediate version-checked reuse pass on the local
  macOS host. Wasm-bindgen compilation reported 74 seconds; its reuse took
  0.003 seconds. Cargo-ndk installation took 24.45 seconds and reuse 0.16 seconds.
  Linux cache restoration, size and speedup still require hosted evidence.
- The local selected files total 8,765,712 bytes (8.36 MiB), before compression.
  This is a bounded pair of tool executables, not another Rust target cache.
  Record the actual Linux archives after the first successful main save, and
  remove superseded exact-version entries only with replacement evidence.
- At 2026-10-10 21:35 UTC, verified main50 credential replacements allowed
  deleting only three obsolete main49 entries (1,460,061,518 bytes). All other
  28 entries remained, totaling 9,586,982,223 bytes (8.928 GiB). That inventory is
  a point-in-time observation; later source/manifest generations can grow it.

Engine, GPU, credential and shipping checks are unchanged. The fresh shipping
profile proof is deliberately independent of release artifacts and stays fresh.
This platform optimization does not reduce the Windows desktop critical path;
workspace compilation, execution and queue timings are measured separately in
the warm PR comparison.
