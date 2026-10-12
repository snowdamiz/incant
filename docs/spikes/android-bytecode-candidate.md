# ARM Android bytecode candidate

The default build still denies ARM macOS to Android bytecode and all authored
Android project exports. A separate `probe-android-bytecode` feature permits only
the exact `mobile_compat.js` fixture, its expected name and no source map through
the standalone trusted cooker. Both producer and consumer opt in; the feature
participates in the engine build identity. The pinned QuickJS 28 numeric/ABI
checks and unsafe fresh-build loader boundary remain intact.

The source experiment has independent review and 22 passing default/candidate
Rust admission tests. Nine CPU harness controls cover actual child reaping after
ADB failures, foreign serial ownership, path transport, remote no-clobber checks
and retained failed command streams. The first Android build failed before a
runtime archive was produced: bindgen lacked the NDK target sysroot. Its raw
failure is preserved; the isolated harness now supplies target-only header paths.
The second frozen build passed in 128.96 seconds and retained its actual ARM
archive/C executable, bytecode and hardening arrays, compiler identity, target
normal graph and symbol audit. The private Cargo build tree was removed.

`tools/probes/android-bytecode/build.py` pins the local NDK 28.2.13676358 and
repository Rust compiler, builds a fresh `ci` archive and links the unchanged
C caller. `verify.py` accepts its hash-bound receipt, creates only a unique owned
headless ARM AVD, reuses the existing typed numeric/failure controls, and cleans
up that process and AVD. It never uses another AVD. This experiment is not a
production exporter, shipping-profile, physical-phone, rendering, performance
or phase-gate result. Production admission requires a separately reviewed change
after actual target records match the numeric oracle and independent review.

The actual owned Android 17 / API 37 ARM emulator run passed all 16 C ABI
controls in 24.00 seconds, including three successful processes, exact failure
statuses, output no-clobber and input held/released movement. Four completed
three-tick game reports each contain all 15 exact numeric/language log records;
the complete game/error values and 19,200-byte silent fixture PCM match the prior
macOS and iOS C/Swift runs (16 full comparisons). This checks signed zero,
subnormal/extreme f64 bits, BigInt, typed-array layout, Unicode, module state and
tick/input behavior. It does not imply arbitrary cached bytecode compatibility.

The 133 recorded execution/AVD commands include explicit owned-name checks,
graceful emulator exit 0 and AVD delete 0. Unrelated AVD metadata is unchanged.
All 282 build/execution raw command streams were rehashed. Three-package candidate
Clippy, formatting and diff checks pass. Independent final execution review is
pending; default production and authored Android admission remain denied.
The actual archive is 119,411,916 bytes and the C caller is 25,219,240 bytes under
`ci`; these are neither shipping size nor phone performance measurements.

The [evidence manifest](evidence/android-bytecode-candidate/manifest.json) retains
both source reviews, native/harness tests, the failed first target attempt and
422 completed proof records in a lossless archive. Four external binary paths
remain hash-bound and available. The original candidate commit is `ccdb3e99`;
source freezes stayed unchanged through build and execution. Later production
admission and public authored export require their own reviewed source and proof.
