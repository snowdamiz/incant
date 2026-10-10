# Native QuickJS component columns

This is the first functional script host for PLAN.md section 2.6. The canonical
`incant_script` package now depends on the native runtime rather than authoring.
The old document-backed implementation is mechanically relocated to
`incant_play_legacy`; current user-facing headless gameplay still uses that package.
This checkpoint does **not** complete migration of PlaySession or its systems.

## Execution and bytecode boundary

`CompiledScript::compile` accepts SWC CommonJS output, bounded to 1 MB, and invokes
QuickJS's compile-only module declaration and bytecode writer. It executes no
project code. `ScriptHost` initializes a fresh bounded VM from those immutable
compiler-produced bytes, then calls the retained update function on every tick.
The bytecode bytes are private and have no public file/cache deserializer. The
same pinned QuickJS compiler/runtime produces and consumes them; a hash would
not establish executable-code trust. The host retains the source byte allocation
until after all function roots, context and runtime are destroyed because the
loader's ROM-data path may retain references into it.

`update({chunks, dt})` receives one array of actual Bevy table views: stable ULID
strings, Transform f64 stride 10 (translation xyz, rotation xyzw, scale xyz), and
optional Velocity f64 stride 3. A call updates all matching entities in bulk;
there is no project clone, command bus, JSON conversion or per-tick code parsing.
The caller owns native stepping; the script phase does not increment world time.
No filesystem, network, module loader or engine-agent shell capability is installed.
The follow-up [structural binding slice](native-script-structural.md) adds a
tick-local `commands` capability to this same context without changing existing
bulk callbacks.

`examples/native_bulk` is explicitly a development runner: it loads a checked
binary scene and compiles CommonJS **once at process startup**. Shipped bytecode
packaging, authenticated loading, version compatibility and source maps remain
open. This development example is not evidence for the no-source-parsing export
gate or the completed cook pipeline.

## Foreign-view lifetime audit

The outer `NumericColumns` scope exclusively owns the native world. The adapter
collects aligned initialized f64 ranges, original values for bounded undo, and
stable IDs. It checks that ranges do not overlap. Safe slice visits end before
the adapter exposes raw pointers. The adapter then retains exclusive access to
the outer handle and performs no more visits while foreign views exist. Table
allocations cannot move, and no native system or derived-state observation can
run until the scope ends.

Each numeric range has exactly one non-owning, fixed external ArrayBuffer; its
reallocation callback is null. A rooted buffer is recorded before constructing
the fallible typed view. QuickJS's same-length transfer can steal a pointer even
from a fixed non-owning buffer, so the fresh context disables every pinned
ArrayBuffer transfer entry point before project code runs, freezes that prototype
and constructor, and refuses additional realms/workers/structuredClone. Ordinary
owned copies remain possible. Typed arrays use the engine intrinsic constructor.

Every exit detaches all roots before native component reads, undo restoration or
release of the outer ECS scope. This includes partial metadata construction:
prototype setters can execute project code during property installation, so the
error path revokes before reading even an exception's name/message getter. Saved
typed arrays, subarrays and DataViews then retain only detached buffer objects.
Failure restores the exact bounded numeric prefix while still inside the world
loan. Outer-scope cleanup publishes change ticks and refreshed hierarchy once.

This deliberately extends raw pointer use beyond a safe slice callback, within
the exclusive outer scope; the native API's documentation now states that
adapter contract. No pointer is cached across ticks or structural changes.
Unsafe operations are confined to external-buffer creation, value ownership,
bounded restoration, trusted bytecode loading and a read-only pending-job query.
Miri/sanitizer and exhaustive engine sandbox proofs are not claimed.

## Failure, budgets and synchronous work

VM memory is capped at 32 MiB and stack at 1 MiB. Each invocation has a thread-CPU
deadline, default 50 ms. Numeric undo has a separate 32 MiB cap; ID strings,
metadata and bookkeeping are additional bounded-by-row-count allocations. The
budget begins before metadata exposure and stays active through diagnostics.
Exception, allocation failure, CPU expiry, unsupported return or deferred work
rolls back numeric writes and permanently fails the host. Recreate the host to
continue. Process abort cannot run rollback destructors.

The synchronous contract rejects **created**, ignored and returned promises;
checking only a returned Promise would silently accumulate never-executed jobs.
A nonallocating native hook counts promise initialization. The pinned
QuickJS source creates two internal promises for one synchronous JS module:
`js_evaluate_module` creates its completion promise, then
`js_execute_sync_module` calls `js_async_function_call`, which creates an execution
promise before the module body runs. Ordinary initialization must observe exactly
two; ticks must observe zero. The counter saturates at three. A read-only queue
check also rejects pending jobs; the host never drains jobs. The two-promise
baseline was confirmed by an unsaturated runtime measurement and pinned-source
review, and must be re-audited when QuickJS or the module strategy changes.

Direct numeric access intentionally does not revalidate components each tick,
as PLAN.md requires. All f64 bit patterns remain memory-valid. Gameplay can write
NaN, overflow or unusable transforms; consumer math policy and optional development
diagnostics are separate work. Invalid arithmetic is not turned into an authoring
validation pass. This adapter's rollback handles execution failure, not all
possible gameplay mistakes.

## Verification and remaining migration

On arm64 macOS, all 363 workspace tests across 114 suites and workspace all-target
Clippy pass after the ownership and async review fixes. Eleven focused script
tests are included. Tests exercise actual multi-table Transform/Velocity
writes for 120 ticks, hierarchy, retained views across 5,000 spawns/archetype
moves and GC, exceptions, memory/CPU failure, transfer routes, copied buffers,
unsupported state/async, ignored unresolved promises, and hostile metadata/error
getters, dynamic imports and deferred work during metadata exposure. All 24
Python tool tests and the generated conventions check also pass.

A development CLI run executed 60 native steps with script writes from a checked
scene after the original project/journal had been removed. Two fresh processes
matched exactly, and zero ticks restored the original scene. Parent translation
was approximately [3, 2, 0], child local translation [1, 4, 3]. The probe was
repeated after the final review fixes on the newly built binary; its hash and
results are in [recorded evidence](evidence/native-script-columns-2026-10-10.json).

The dependency guard now rejects `incant_play_legacy` and checks `incant_script`
alongside `incant_runtime` and `incant_types`. The final resolved graph passes.
Regenerated schemas are byte-for-byte unchanged and the generated SDK check passes.
Hosted CI remains pending. Independent Astra Extra High review found no concrete
memory-safety defect, identified the async gap above, and confirmed it resolved
on re-review.

Persistent behavior state, timers, logs, input/actions, queries/physics,
other structural components, hot reload, saves, SDK compatibility, real gameplay migration,
browser/Wasm hosts, authenticated cook/export artifacts, profiling and performance
work remain. Each tick currently rebuilds metadata/IDs and copies numeric undo;
these costs need measurements and optimization. No shipping-profile improvement,
10,000-entity phone budget, hosted device run or phase gate is claimed here.

The final local verification commands were:

```sh
./tools/cargo fmt --all --check
./tools/cargo test -j2 --workspace --release --locked
./tools/cargo clippy -j2 --workspace --all-targets --locked -- -D warnings
./tools/cargo build -j2 -p incant_headless -p incant_script --examples --release --locked
./tools/cargo build -j2 -p incant_headless --release --locked
target/release/incant_headless schema schemas
git diff --exit-code -- schemas
node tools/generate_sdk.mjs --check
python3 tools/check_runtime_dependencies.py incant_types incant_runtime incant_script
python3 -m unittest discover -s tools/tests -q
```

## Integration with reviewed runtime and CI changes

On 2026-10-10, native-host commit `9a463ca` was merged with reviewed PR #48 head
`588b3ab96b31bcebfbff753959c9b8e6bf13f053`. The original numeric-column cherry-pick
`b472121` and its upstream commit `9a18e3f` had identical trees. The merge retains
the native host's newer outer-scope raw-pointer contract and execution-failure
rollback documentation, including the prohibition on per-tick authoring
validation. All crate sources, Cargo manifests/lockfile and the runtime dependency
guard remain byte-for-byte identical to the native-host commit.

The integrated CI retains `incant_script` as an authoring-free guard root and
includes the reviewed dependency caches, common workspace GPU feature graph,
combined editor/headless builds, shipping verification, and Windows CRT policy.
The native-host, CI-performance and CRT status entries are all preserved.

Local arm64 macOS verification of the merged tree passed:

- Full release workspace suite: 363 tests, including all eleven native-script
  tests and all five runtime doctests.
- Consolidated ignored workspace suite: all 44 GPU tests, serialized as in CI.
  This invocation compiled no packages; that is local build-reuse evidence, not
  a hosted cold/warm performance measurement.
- Workspace all-target Clippy with `-D warnings`, formatting and generated
  conventions checks.
- All 25 Python tool tests and the resolved `incant_types`, `incant_runtime`,
  `incant_script` dependency guard.
- Generated bridge/SDK checks and SDK TypeScript compilation. The first bridge
  attempt found missing local npm dependencies; `npm ci` restored the pinned
  graph and the repeated checks passed without tracked output changes.

All Cargo builds used two jobs. Full logs are retained locally as
`/tmp/incant-native-host-integration-workspace.log`,
`/tmp/incant-native-host-integration-gpu.log`,
`/tmp/incant-native-host-integration-clippy.log` and
`/tmp/incant-native-host-integration-tools.log`. Hosted Windows linkage/imports,
PDB packaging, CI cache timing and the migration/device gates above remain
pending; local integration checks do not certify them.

## Publication verification with structural bindings and CI follow-up

The complete native-host branch at `7b04b460eb35df34bbe7eefa159ec9157a45d821`
also includes the [structural bindings](native-script-structural.md) and reviewed
CI follow-up `0b501e91063b1a17f44fbb53888376ea6f028927`. The latter builds the exact
editor/headless/cook/collaboration targets together and invokes their binaries
directly, retaining every gameplay probe and fresh shipping verification.

Additional integrated nonvisual checks on 2026-10-10 passed on arm64 macOS:

- All 44 ignored GPU tests, with the exact test identities matched against the
  recorded baseline, and all 387 UI/bridge tests plus the production UI build.
- Audio device compilation and the combined release command for `incant_editor`,
  `incant_headless`, `cook_scene` and `collaboration_spike` with the editor's
  `custom-protocol` feature.
- Regenerated schemas unchanged, the evaluation-suite check, the real
  two-client collaboration example, and all fifteen existing public gameplay
  probes from the CI workflow.
- Fresh shipping-profile build/package verification: fat LTO, one codegen unit,
  portable CPU policy, abort without unwinding, matched dSYM UUIDs and successful
  source-independent cooked-scene execution of the packaged player. This player
  remains the native Transform/Velocity runner; it does not certify a shipped
  script artifact or device performance.

All 24 original script-package files were compared to their `incant_play_legacy`
counterparts. Only the package/namespace rename and explicit temporary-path
documentation differ; gameplay implementation is unchanged. The preceding
373-test workspace and workspace Clippy results cover the same Rust sources.

Logs use `/tmp/incant-native-host-publication-` followed by `gpu.log`,
`ui-tests.log`, `ui-build.log`, `audio.log`, `build.log`, `eval.log`,
`collaboration.log` and `shipping.log`. Individual public-probe logs/results and
the fresh shipping manifest/archives are retained in the ignored
`artifacts/native-host-publication/` directory. Native-script-only standalone
evidence remains linked from the structural audit. Hosted checks on the new PR
remain separate from these local results.

## Native mobile compilation

The platform smoke applications do not yet link `incant_script`. A direct mobile
check exposed missing bundled bindings in pinned `rquickjs-sys` 0.14.0 for both
iOS and Android. The native host now enables the supported `bindgen` feature only
for `target_os = "ios"` or `"android"`; its added build-time resolutions are pinned
in Cargo.lock. The engine/QuickJS versions and runtime source remain unchanged.

Local compilation passes for `aarch64-apple-ios-sim` using Xcode 27.0 (27A266a),
iPhoneSimulator SDK 27.0, and for `aarch64-linux-android` using installed NDK r28c
28.2.13676358 with its API-21 clang/ar and sysroot headers. The Android check uses
explicit target compiler/archive/linker variables plus target-specific
`BINDGEN_EXTRA_CLANG_ARGS`; no local cargo-ndk installation is required. Both use
Rust 1.99.0 and two Cargo jobs. The local NDK differs from hosted CI's pinned
28.0.13004108, which remains a separate compile check.

The existing iOS and Android jobs now compile `incant_script` explicitly, and
`crates/incant_script/**` triggers that workflow without removing existing paths.
Android uses installed cargo-ndk 4.1.2, which configures bindgen sysroot/include
arguments automatically, plus the pinned NDK's host libclang. This is verified in
the released cargo-ndk 4.1.2 source (`src/cargo.rs`); no extra compiler download or
global bindgen feature is introduced. Browser scripts remain a separate backend;
the web job is not made dependent on QuickJS.

Cargo's resolved feature graph confirms that ordinary macOS/Windows/Linux native
targets retain their existing QuickJS features and do not enable bindgen. Mobile
targets enable it. After the manifest/lockfile change, all 373 workspace tests,
workspace all-target Clippy and the authoring-free dependency guard pass again.
All existing lockfile package versions/checksums are preserved; nine build-time
or optional package resolutions are added, including bindgen 0.72.1 and clang-sys
1.9.1. Actionlint passes for the updated platform workflow.
These are compile checks only: no mobile VM execution, ABI integration with a
packaged app, reference-phone performance, or phase-gate completion is claimed.
Logs are `/tmp/incant-native-host-publication-ios-script-bindgen.log` and
`/tmp/incant-native-host-publication-android-script.log`. The locked iOS repeat is
`/tmp/incant-native-host-publication-ios-script-locked.log`; desktop and feature
graph results are `/tmp/incant-native-host-mobile-workspace.log`,
`/tmp/incant-native-host-mobile-clippy.log` and
`/tmp/incant-native-host-mobile-features.log`.

## Final main integration and publication checks

Verified PR #48 merged as `56c22d408fa3e85aaf6bc37e00f856c664dfd0c8` after all
17 hosted checks passed. Merging that actual main into this branch produced
`560ae9806ba863bc72313bad75e00614303a2b80` without changing the source tree.
The native-host branch also includes the separately reviewed CI refinements
`bcec1c4b49bb246c9a60f902aeb06d00e8176892` and
`8bed53205ee9b193a1c81a783033fee165f1f1f4`; their integration changes only
workflows and evidence after the preceding Rust/mobile checks. The full
[CI audit](ci-build-performance.md) and [retention policy](ci-cache-retention.md)
record the measured limits and qualifications.

The exact workspace-selected release build with editor `custom-protocol`
completed locally in 26.47 seconds, rebuilding only Tauri and the editor and
emitting exactly the four requested editor/headless/cook/collaboration
executables. This verifies build reuse in the existing local target directory;
it is not a controlled hosted speedup or engine-performance result. Regenerated
schemas stayed unchanged, and the evaluation suite, collaboration example and
all fifteen public gameplay probes passed again on those binaries.

A new fresh shipping build at `560ae9806ba863bc72313bad75e00614303a2b80` passed
with the final mobile lockfile. Its manifest confirms portable CPU policy, fat
LTO, one codegen unit, abort without unwinding, matching Mach-O/dSYM UUID
`02f2c9d6-a503-3809-b21c-f17be85d5fa7`, and source-free cooked-scene execution.
The player remains the Transform/Velocity runner, so this is not evidence of a
shipped script artifact or mobile performance. Logs are
`/tmp/incant-native-host-final-tools-build.log`,
`/tmp/incant-native-host-final-tools-artifacts.jsonl`,
`/tmp/incant-native-host-final-eval.log`,
`/tmp/incant-native-host-final-collaboration.log` and
`/tmp/incant-native-host-final-shipping.log`; probe results and the manifest are
in ignored `artifacts/native-host-final-publication/`. All four changed
workflows pass actionlint after the cache-policy integration, with the native
dependency guard and mobile script checks preserved. New native-host hosted
results remain pending publication; PR #48 results cover its earlier scope.
