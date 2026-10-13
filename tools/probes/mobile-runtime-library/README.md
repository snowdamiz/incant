# Real mobile runtime C ABI host verification

These probes call `incant_game_run_v1` through the published C header and a real
static archive. The C executable and Swift's imported C declaration both consume
bounded, caller-owned immutable bytes. The typed fixture generator writes a
`CookedScene`, hashed package, recorded native inputs and observation assertions;
it never uses the old document-oriented platform smoke.

The current fixture deliberately matches the build-linked default `game.js`.
KeyD is consumed after integration: three ticks produce x=0.1 when held, x=0.05
when released at tick2. KeyF throws during tick2. These are actual native gameplay
checks, but not an authored-game mobile exporter or complete phone host.

## Existing archive run

Build the fixture generator in a private target:

```sh
CARGO_TARGET_DIR=/private/target cargo build --locked --release -j2 \
  --manifest-path tools/probes/mobile-runtime-library/fixture/Cargo.toml
python3 tools/probes/mobile-runtime-library/verify.py \
  --archive-receipt /absolute/receipt.json --receipt-sha256 EXACT_SHA256 \
  --fixture-generator /private/target/release/incant-mobile-runtime-fixture \
  --output /absolute/new-proof-directory
```

The receipt binds archive/header hashes, target, profile, feature set, successful
archive-build command, source identities and the exact default guest hash. An
explicit `linked_guest` record can instead bind the separately reviewed
`mobile-compat` source and its expectations by path/SHA-256. That route requires
every one of the 15 typed numerical log records, including tick/level, and the
held/released native position observations. No missing records are accepted. A
caller executable is linked from that archive; it is not a source-rebuilt
substitute. Reports retain every command, output digest, request and assertion.
The runner requires exact failure diagnostics, not merely nonzero process exits.
It checks create-only output preservation, repeated exact reports/PCM, invalid
requests and corrupt package refusal. Completed failed observation reports can
retain PCM, matching the existing player contract.

For `aarch64-apple-ios-sim`, it creates its own disposable simulator, boots it
without opening Simulator.app, then uses `simctl spawn` for these same C/Swift
commands and deletes only that simulator. No UI, screenshot, display, physical
phone or frame-time claim follows. For `aarch64-apple-ios`, it only links callers;
it does not sign, install or execute a device application.

The macOS default archive currently passes 32 C/Swift controls. The first proof
is retained as a test-oracle correction to the exact `script tick failed`
diagnostic; no engine code or archive changed. iOS producer compatibility is
being implemented separately: an existing cross-target cook refusal is a real
failed build, not a portability pass.

## Android candidate job

`android.py` provides an NDK archive plus real C-ABI link check for the existing
Linux x86_64 candidate job and its pinned NDK/cargo-ndk tools:

```sh
python3 tools/probes/mobile-runtime-library/android.py \
  --ndk "$ANDROID_NDK_HOME" --target-dir /private/android-target \
  --output /absolute/new-android-proof
```

It keeps target compatibility guards, produces failure evidence if they refuse,
and never counts a linked ELF as runtime execution. The existing candidate job
must not enable this new step as a successful runtime gate until the Android
producer/consumer bytecode contract is admitted and reviewed. Local NDK and
Android runtime execution evidence are absent. Hosted Android runtime testing,
physical phones, touch/native surface/audio, shipping size and device performance
remain separate requirements.

Harness rejection controls can be run with
`python3 tools/probes/mobile-runtime-library/test_verify.py`; they are not live
platform evidence. The fixture's standalone Cargo lock preserves the engine's
resolved versions and checked `rerecast` patch.
