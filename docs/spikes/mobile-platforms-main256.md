# Native Apple and Android host composition on main256

The opt-in Apple and Android hosts are composed with the main256 haptics and
persisted-checkpoint player. Tested source is `a928652ad67a714c9709cfb12db813a0f3163378`.
The exact actual-main256 tree equals the reviewed haptics branch used as its base.
Final integration review is pending; this packet does not claim device execution.

## Composition

The original 28-path union has 26 paths exactly equal to one accepted producer.
Only `HostedGameSession` and its hosting tests require the existing haptics and
new suspended-callback constructor to coexist. Root source review accepted both
joins. The Android manifest and lock composition uses its reviewed producer;
all 968 package identities remain unchanged, with only three already-locked
optional player edges added: jni 0.22.4, ndk and ndk-context.

The initial joined Mac configuration passed 151 tests, then strict Clippy found
that Apple and desktop hosts loaded the same clock file twice. The reviewed
two-file correction shares the existing desktop clock module on macOS when
native-window is enabled. Apple-only/iOS keeps its previous module; Android is
target-disjoint. Clock implementation bytes are unchanged. Removing the duplicate
module also removes three duplicate tests, so the final count is 148, not a
regression or omitted unique coverage. The original success logs and lint failure
remain in the archive.

## Current execution

Both deterministic and fast SIMD8 compositions passed 148 actual tests across
15 harnesses each: 103 player and 45 game cases, zero failed or ignored. The
selected features combine Apple/Android/mobile FFI, desktop window, native
checkpoints, UI and the linked static haptic fixture. Fresh CLI binaries are
copied immediately and used by the process-level checkpoint tests. There are
32 final executable copies; the initial generation's 16 copies also remain.

All 44 final commands passed: both complete composed suites and strict Clippy,
iOS compilation, Android API26 ARM64 compilation and strict Clippy, ordinary
player compilation, seven Android helper controls, manifest/dependency guards,
formatting, generated conventions and whitespace. The target and all child
processes were released. The static fixture is hash-bound; this is not a fresh
shipping export or shipped Wasm checkpoint claim.

## Preserved producer evidence and limits

Apple packet `1ab47d13` retains 43 records, 31 commands (two historical failures),
seven copied executables, final 40-player-test runs per mode and Swift typechecking.
Android packet `8b1d3e14` retains 131 records, 19 command receipts, three copied
executables, 38 Mac player cases and Java compilation. Their counts overlap
current tests and are not summed into current execution.

Both original collectors are unchanged. The Apple collector is replayed in a
temporary exact producer Git-file view so its live-source equality assertion is
preserved. The Android collector verifies its immutable source object directly.
The foreign Swift/C/Java inputs are unchanged. Producer ancestry is retained via
explicit unchanged-tree merges; the reviewed current source is not overwritten.

This establishes Mac CPU and compiler behavior only. No GPU, phone launch,
physical device audio, motor output, appearance, exported Apple app, full Android
APK link/package or performance sampling was performed. Mobile haptics and IME
remain unsupported. Mobile and other-device execution are deferred, nonblocking
follow-up under the current Mac gate policy; no phase/distribution approval is
implied.

Portable read-only audit:

```sh
python3 docs/spikes/evidence/mobile-platforms-main256/verify.py --verify-only
```

`--originals --local-binaries` additionally rehashes the original local records
and all retained producer/current executable copies. The verifier checks 3,875
frozen current inputs, all 29 changed source/build paths, the accepted joins,
artifact features, exact test totals, failure history and producer archives.
