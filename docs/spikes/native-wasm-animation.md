# Native WebAssembly animation controls

The additive `incant_animation_v1` namespace gives actual Wasmtime development
modules and compiled wasm2c native modules access to the same native playback
states and atomic controls as TypeScript. It introduces no authoring simulation,
asset decoding or project mutation path. Old import signatures and opcodes remain
unchanged. Runtime code does not parse the build-time JSON ABI registry.

`Tick::animations` copies one bounded snapshot into caller-owned initialized
scratch. Each row contains the stable entity ID, optional clip, seconds, signed
speed, playing, looping and completion. Row order is unspecified. The borrowed
iterator validates the whole wire frame before exposing rows, supports unaligned
storage and has no per-entity host calls or allocations. The host retains row and
byte capacity across updates; immutable animation assets are not copied.

`Tick::control_animation` accepts Select, Seek, Playing, Looping and Speed.
Controls queue normal `StructuralCommand::ControlAnimation` entries. They publish
only if the complete existing numeric/structural/log transaction succeeds.
Readback stays at the start-of-callback state despite queued changes; numeric
column readback retains the existing staged-write behavior. Clock advancement is
owned by the native world before scripts, not by the import. A successful control
therefore changes the committed render sample immediately and the next native
clock step. Selecting/resting preserves speed/play/loop flags. Select and Seek
reset completion; playing or looping true clears completion. Clip existence and
duration bounds are checked against the actual native declaration at commit.

## Wire boundary

The authoritative generated registry remains `sdk/abi/native-v1.json`. The new
imports use opcodes 22–24: snapshot length, snapshot copy and control. A 16-byte
header (`INCAN001`, count and stride) precedes 40-byte rows. Rows are 16 big-endian
identity bytes, little-endian u32 clip/flags, then f64 seconds/speed. Clip
`0xffffffff` means rest; flags 1/2/4 mean playing/looping/completed. Unknown flags,
nonfinite values and impossible rest/completion flags refuse.

The fixed 24-byte control record is u32 kind, u32 clip, f64 seconds and f64 value.
Kinds 0/1/2/3/4 select, seek, set playing, set looping and set speed respectively.
Unused fields must be positive zero. Select uses clip/seconds; Seek uses seconds;
flags use value 0 or 1; speed uses signed value. Every byte range and epoch is
checked before use. Records are copied, never cast to native pointers.

At most 100,000 snapshot rows use 4,000,016 bytes. Clip indices are below 256 and
finite speeds are between -16 and 16. The existing 32 MiB guest memory, 128 MiB
transfer, 65,536 host call and 1,024 structural command ceilings still apply.
Native clip durations and scene capacities remain authoritative. Caller-selected
scratch must hold the whole snapshot; insufficient capacity is an explicit error.

Correct-epoch animation ingress errors latch rejection even if a direct boundary
caller ignores the error and later submits a valid operation. The safe Rust SDK
also latches invalid local arguments and undersized buffers. Actual imports trap
on errors, including stale epochs. Callback traps and commit refusals reject all
staged numeric, entity, animation and diagnostic work and fail the module. This
does not undo native simulation or external device audio that already advanced
before the callback. Guest memory/globals are not rolled back.

## Verified scope

The complete combined native suite passes 281 tests/doctests in 48 groups, with
zero failures or ignored cases, plus all-target Clippy. Sixteen new tests cover
SDK framing, independent Capability boundaries and actual two-host animation
execution. Seven real no_std Rust animation guests and two raw WAT fixtures are
compiled by the trusted helper, validated/metered, translated with WABT 1.0.42 and
linked into the native test executable beside Wasmtime 49.0.2. The host is
arm64 macOS with Rust 1.99.0. No GPU or screenshot was used.

Independent expected playback values cover pause/seek, reverse speed, looping,
clip switching, zero-length clips, rest, completion and restart, while a second
instance advances independently. Read-after-control remains unchanged. Ignored
SDK buffer/nonfinite/rest errors, native clip/entity rejection, raw unaligned
frames, stale epochs, malformed tags/reserved fields, out-of-bounds copies and
traps all reject staged numeric, entity, animation and log work. Direct boundary
tests additionally reach the actual shared call/transfer/command/memory ceilings.
Unknown namespaces, changed signatures and import calls at module start refuse.

Three fresh development processes run the pre-existing bulk sample after deleting
its copied guest source; their outputs agree exactly. This checks the existing
fresh-process path, not a new source-deleted shipping animation export. Native
static animation execution is established by the real wasm2c conformance tests.
The separate TypeScript animation export proof is documented independently.

Default wasm32 compilation of ABI/SDK/runtime/development crates, normal runtime
and compiler exclusion traversals, generated-file equality, formatting,
conventions and 20 relevant Python tooling tests pass. Compilation is not actual
browser/device execution. Cargo.lock is unchanged. The independent host/SDK and
conformance review receipts plus exact source/log/guest identities are recorded in
[evidence](evidence/native-wasm-animation.json).

Earlier runs preserve their failures: a test compared accumulated native time
with exact literal 0.2 before switching to native-copy equality plus a tolerance;
Clippy requested constant-sized test iteration; mistyped package names were
corrected. Root self-review tightened the SDK parser to reject completed+looping
flags before final conformance. The final compiled inputs were frozen throughout
validation; no production fix was inferred from the floating-point test error.

The trusted conformance helper accepts `--profile ci` for bounded local builds
and resolves the cook executable from Cargo's machine output, honoring private
`CARGO_TARGET_DIR`. Its default stays `dev`; actual shipping builds continue to
use the shipping profile. No dependency version, runtime compiler boundary or
host/device acceptance requirement is relaxed.

Animation blending, state machines, IK, root motion, complete gameplay SDK,
arbitrary-source build sandboxing, reference-phone performance and Phase 1
completion remain separate work. This slice requires no rendered appearance
change or new screenshot. Native rendering acceptance remains with Claude.

## Actual main117 integration

Frozen `ca82e3f` preserves the accepted original `619e850` ancestry on main116.
The actual combined Wasmtime/static suite again passes 281 tests in 48 groups,
zero ignored or failed, plus all-target Clippy and three fresh development bulk
processes. Default wasm32 compilation, 18 relevant tooling tests, generated SDK equality,
normal runtime dependency checks, formatting and conventions pass. A follow-on
compile invocation initially named a nonexistent SDK package; correcting the
command to `incant-module` passes without changing source or repeating conformance.

Main117 then merges with only additive STATUS resolution. Every tested runtime,
script, SDK, helper and Cargo input remains identical; the Console changes affect
only editor/bridge code and documentation. All 27 accepted implementation hashes
are preserved. [Integrated logs and process receipts](evidence/native-wasm-animation-main117.json)
record exact source boundaries. No new graphics, native-window, shipping-animation
export, browser or device acceptance is claimed by this CPU SDK integration.

## Crossfade extension

The preceding v1 evidence describes its original frozen implementation. The
additive [crossfade SDK](native-animation-crossfade-sdk.md) now uses explicit
`incant_animation_v2` imports, preserving opcodes 0–25 and all v1 records. Current
`Tick::animations` exposes full transitions through v2; explicit legacy methods
remain available for older hosts. An active crossfade makes both v1 length/read
refuse rather than hiding one side of a blend. No v1 row grew silently.
