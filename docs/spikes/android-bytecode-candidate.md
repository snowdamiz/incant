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
Actual target compilation and ARM emulator execution are still pending.

`tools/probes/android-bytecode/build.py` pins the local NDK 28.2.13676358 and
repository Rust compiler, builds a fresh `ci` archive and links the unchanged
C caller. `verify.py` accepts its hash-bound receipt, creates only a unique owned
headless ARM AVD, reuses the existing typed numeric/failure controls, and cleans
up that process and AVD. It never uses another AVD. This experiment is not a
production exporter, shipping-profile, physical-phone, rendering, performance
or phase-gate result. Production admission requires a separately reviewed change
after actual target records match the numeric oracle.

The [initial evidence](evidence/android-bytecode-candidate/manifest.json) retains
native tests, source review and the failed first target attempt. Later evidence
must retain exact compiler, source, bytecode, caller, emulator and cleanup identity.
