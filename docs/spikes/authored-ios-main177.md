# Authored iOS archives on main177

The integrated exporter captures public-authored TypeScript imports, props and
scene bindings into a shipping iOS archive and data package. The current physics
selection layer is reconciled by admitting deterministic projects, building with
explicit defaults-off deterministic features, auditing the selected normal graph,
and verifying all five actual target dependency feature records. Fast projects
refuse before compilation. Existing desktop cross-target and mobile Wasm guards
are unchanged.

Tested source `de3ed2c9d4f5178310b45f968de25ccc2621e198` contains actual main176 and
all reviewed process/export helpers. Merge `4900a8b8e82fa18c082259943c5e4df4a332446e`
adds actual main177 ancestry with only four documentation/status files changed.
All 1,930 frozen compiled/probe inputs remain identical. Of 16 producer source
files, eleven are byte-identical; two common cook paths retain established main
physics admission, and three iOS helper/test files carry the independently
reviewed selected-graph/fixed-kernel integration.

The checks pass 205 Rust cases across 17 groups, with zero failures or ignored
cases: 181 script, eight existing cook cases, thirteen player cases and three new
mobile-project cases. The new test target was added as a separate supplemental
check after the initial runner omitted it; no source changed. Scoped all-target
Clippy, dependency and feature guards, ten exporter tests, formatting, generated
conventions, and diff checks pass. A matching immutable headless binary prepares
the public authoring fixture through transactional edits, Undo/Redo and reopen.

Both fresh shipping exports pass their actual compiler, selected dependency and
archive symbol audits. The simulator archive is 14,527,072 bytes and the unsigned
device archive is 14,522,416 bytes. Their disposable build directories are removed.
The C and Swift callers link with dead stripping and independently verify runtime
entry points and absence of source-parser symbols.

On the iPhone 18 Pro / iOS 27.0 simulator, six source/cache/build/map-deleted CPU
processes each run twelve ticks and match all 72 selected observations, 36 complete
log records and 76,800 nonzero PCM bytes against the native baseline. Fourteen
controls verify no-clobber, exact late script/checksum failures and early invalid
request refusals. All runtime bytes stay unchanged. The disposable simulator is
shut down and deleted. The device target has eight successful unsigned C/Swift
link/symbol commands and no execution. The full caller proof has 41 commands.

The [manifest](evidence/authored-ios-main177.json) binds 311 losslessly retained
artifacts (121 unique payloads), nine preserved external archive/executable
identities, eighteen successful top-level checks, exact source comparison, both
accepted reviews and all actual caller streams/reports/PCM. Original producer
evidence remains separately scoped. Final independent integration review is
accepted without findings; no physical-device, GPU, window, signing, mobile host or performance/phase
gate is claimed.
