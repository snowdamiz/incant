# Apple native platform host

The opt-in `incant_game_player/mobile-apple` adapter connects the retained cooked
session to a CAMetalLayer, fixed-rate display callbacks and the actual CPAL audio
backend. Creation starts suspended. Delivery begins only after a drawable is
reserved; backgrounding, resizing and unavailable output revoke old input and
suspend audio. A frame never survives its callback. Failure closes the game;
already submitted GPU/audio output cannot be rolled back.

The shared touch router separates UI-owned gestures from gameplay and retains
their original authority through release/cancel. It bounds simultaneous touches,
suppresses extra UI presses, and refuses stale callbacks after lifecycle or UI
replacement. Device-hosted sessions start suspended without acquiring the
seed-session restart privilege.

The C ABI in `include/incant_apple.h` exposes process-unique nonwrapping numeric
handles, not Rust addresses. Calls are serialized on the main thread and resolved
through thread-local ownership. Reentry refuses; a caught unwind retires the
owner. Close cannot destroy a replacement owner. A valid open output is cleared
even when other arguments are rejected. Shipping panic-abort behavior remains.
The caller retains the CAMetalLayer through close.

`platforms/ios-player` supplies a UIKit callback adapter with a weak display-link
proxy, captured gesture epochs and explicit teardown before releasing the layer.
This is native scheduling and input glue, not authored game presentation. Its
Swift 6 source typechecks against the installed iOS simulator SDK. It has not
been linked, packaged or executed as an iOS app. IME, mobile haptics and complete
platform packaging remain follow-up work.

## Verification

[The retained manifest](evidence/mobile-native-platform/manifest.json) binds the
implementation to source `ab738f41`. Run
`python3 tools/probes/mobile-native-platform/collect.py --verify-only` for a
read-only audit. Optional `--originals --local-binaries` additionally checks the
local source logs and immediately copied test executables.

The packet preserves 31 command receipts, including two historical failures and
their later corrections. Final Apple-enabled player tests pass all 40 cases in
each physics mode, including the rejected-open regression. Both strict Clippy
configurations pass. The earlier shared-hosting suite passes 11 cases; ordinary
compilation, dependency/manifest checks and Swift typechecking retain their
source identities instead of being relabeled as new execution.

This is Mac CPU/compiler evidence. It does not claim live Metal presentation,
physical audio, mobile execution, appearance or performance. Under the director's
current instruction, mobile and other-device execution are deferred nonblocking
follow-up; gate qualification uses this Mac. Platform feature scope remains.
