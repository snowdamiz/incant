# Android concrete retained device host

This opt-in `incant_game_player/mobile-android` route connects the cooked native
player to Android `SurfaceView`/Choreographer, Vulkan and CPAL's API26 AAudio
backend. It is separate from the API24 offline archive API. It has no authoring,
source compiler, JIT, account or document mutation path at runtime.

The standalone Java activity owns one native session on its UI thread. JNI uses
positive nonwrapping IDs, never memory addresses. Other threads have no matching
owner; overlapping opens, stale identities and reentrant accesses refuse. A failed
construction releases its claim. Close is idempotent for the last retired ID and
cannot close a replacement. Rust unwind handling retires an existing owner;
shipping uses panic=abort, so a Rust panic terminates the process there.

The application Context is a JNI global reference initialized before any CPAL
backend. It remains alive with its JavaVM for the process lifetime. Activity
teardown does not reset ndk-context while audio callbacks could still reference
it. This is a standalone embedding contract: android-activity/ndk-glue and another
ndk-context initializer must not be linked. The exporter checks that graph.

A wgpu surface owns an acquired ANativeWindow through a safe window-handle
wrapper. An acquired texture remains within one callback. Surface destruction
suspends audio/input and detaches the target while retaining the game/VM. Surface
replacement uses the existing renderer, checks format/presentation compatibility,
and retires input authority before resuming. Losing an unrecoverable Vulkan
surface/device stops the owner. Activity destruction closes the session; activity
recreation starts a new game rather than claiming checkpoint persistence.

Only a successfully reserved output starts delivery. Unavailable targets defer
fixed-rate simulation. Catch-up is bounded to four ticks per display callback;
background/focus/audio-focus loss suspends output and clears pending gestures.
Logical-point touches use sixteen bounded native slots. Each Android pointer
keeps its DOWN epoch through UP/CANCEL; old releases never acquire a new epoch.
UI-owned gestures use the shared NativeTouchInput contract. Keyboard/IME,
mobile gamepad and device haptics are not supplied by this adapter.

Nonzero output dimensions are 16..4096 per axis, at most 4,194,304 pixels. Logical
viewport dimensions must be finite and 1..65536. These are output admission
bounds, not a total GPU/device allocation guarantee. Device audio availability is
required; there is no offline fallback. Already-submitted OS audio cannot be
rolled back after a later render/presentation failure; the session then stops.

## Export and verification boundary

`python3 tools/export_android_host.py PROJECT OUTPUT --ndk NDK --sdk SDK
--java-home JDK [--scene ULID] [--camera ULID] [--evidence NEW_DIRECTORY]` builds a
fresh ARM64 shipping static archive, links only the six JNI roots into a shared
library, rejects runtime parser symbols, and packages an unsigned APK with SDK36
build tools and minimum API26. It checks the selected physics graph, actual
compiler invocations, existing trusted project/QuickJS ABI receipt and tool
identities. The separate archive exporter and its API24 Wasm route are unchanged.
This first device-host exporter explicitly refuses static Wasm; its native link
and producer audit have not been widened silently from API24 to API26.

The APK carries only cooked assets and linked bytecode. A bounded hashed catalog
extracts up to 8192 files/512 MiB into a fresh application-private directory; each
file is at most 128 MiB. No external path or shell comes from project content.
The exporter never signs, installs, launches, or supplies credentials. The APK
build helper is implemented but a full authored shipping APK run remains separate
from the focused source/compile evidence below.

## Scoped evidence

External execution records are in `~/.codex/incant-evidence/android-device-host-001`.
Source001 retained the first JNI0.22 typed-string/signature compile failure and
three passing pure lifecycle controls. Source002 uses the pinned JNI signature
macros, adds failure/unwind and reentry/drop controls, and bounds Java catalog
line reads before allocation.

Source002 passed five CPU lifecycle/slot tests, six exporter policy tests, Java
`--release 17 -Xlint:all -Werror` compilation against Android36 headers, actual
`aarch64-linux-android` API26 Cargo check and strict target-library Clippy. Only
three optional dependency edges were added; locked package identities did not
change. The full selected Mac player library passed 38 CPU tests and strict library/test
Clippy. The first formatting check found only module declaration ordering; that
mechanical correction is source003. Independent source review is still pending. No Android device, GPU, physical audio, rendered appearance or
performance result follows from these checks. Per the current director's gate,
mobile/other-device execution is deferred and nonblocking; desktop Mac testing
remains the required execution surface.
