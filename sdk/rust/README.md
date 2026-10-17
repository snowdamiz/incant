# Incant Rust module SDK

`incant-module` is a public, allocation-free `no_std` guest API for the native
capabilities implemented so far. Add a path dependency on `sdk/rust`, compile a
`cdylib` for `wasm32-unknown-unknown` with panic abort, provide a panic handler,
and export a callback with `incant_module::module!(simulate)`.

```rust,ignore
#![no_std]
use incant_module::{Result, Tick, IDENTITY};
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
incant_module::module!(simulate);
fn simulate(tick: &mut Tick<'_>, dt: f64) -> Result<()> {
    let mut scratch = [IDENTITY; 1024];
    for index in 0..tick.chunk_count()? {
        let chunk = tick.chunk(index)?;
        let rows = tick.read_transforms(chunk, &mut scratch)?;
        for row in rows.iter_mut() { row[0] += dt; }
        tick.write_transforms(chunk, rows)?;
    }
    Ok(())
}
```

The complete runnable sample is [samples/gameplay.rs](samples/gameplay.rs). It
uses accepted named input, numeric rows, spawn/parent/velocity/despawn commands,
bulk flipbook playback/controls, and committed primitive diagnostics. `tools/rust_guest.py::compile` builds the
trusted repository samples with pinned rustc and the two fixed library roots;
it does not run project Cargo build scripts, dependency downloads or proc macros.
The current helper is a trusted engine build tool, not an untrusted source sandbox.
Compile the standalone sample (install the pinned toolchain's wasm32 target first):

```
python3 tools/rust_guest.py sdk/rust/samples/gameplay.rs /tmp/gameplay.wasm
```

Run the actual two-host suite with:

```
python3 tools/probes/shipping-wasm2c.py /tmp/sdk-conformance --conformance-only
```

The directory must not already exist. The helper validates and meters the same
Wasm before wasm2c translation, links it into the native test executable, and runs
it beside the actual Wasmtime development host. Shipped applications receive
only trusted compiled native descriptors; they never compile or load guest Rust
source, raw Wasm or cached native modules at runtime.

## Data and failure contract

- `Tick` and `Chunk` are callback-bound capabilities. A chunk identifies a whole
  ECS table span, with matching IDs/Transform/optional Velocity row order. Callers
  provide initialized scratch at least as large as `chunk.len()`. No pagination
  or automatic allocation is hidden behind an undersized buffer.
- IDs are canonical big-endian 16-byte values. Transform rows are ten `f64`s:
  translation xyz, quaternion xyzw, scale xyz. Velocity rows are three `f64`s.
  Transfers **copy** numeric bytes; no native pointer, padding, ECS handle or
  native storage reference reaches the guest. Updated numeric copies are visible
  to later numeric reads in the same callback. Native publication waits for the
  existing checked numeric/structural transaction to succeed.
- `input` and `flipbooks` return validated views borrowing caller-owned byte
  scratch. Their iterators need no per-entity host calls. Input is the accepted
  tick's immutable snapshot. Flipbook rows show committed state before queued
  commands; pause/play/restart/seek become visible after successful publication.
- `animations(scratch)` returns a copied start-of-callback snapshot of authored
  skeletal playback. Rows contain stable entity ID, optional clip index, seconds,
  signed speed, playing/looping flags, completion and the optional full outgoing
  transition (clip/time/speed/play/loop plus elapsed/duration), plus the actual
  second pose and destination weight for both timed and parameter-driven blends. `control_animation`
  queues typed Select/Crossfade/Seek/Playing/Looping/Speed changes in the same atomic
  transaction as numeric edits, entity commands and logs. Selecting a clip keeps
  speed/play/loop flags; `clip: None, seconds: 0` selects rest. Seek/Select reset
  completion; native clip durations validate at commit. World clocks advance
  before script execution, so controls affect the next simulation step and the
  immediately committed render sample. Queued controls do not change this
  callback's readback. Crossfade duration is finite 0..60 seconds; zero selects
  immediately. A nonzero request cannot interrupt an active fade. Select/Seek
  cancel it; playing pauses both clocks; later speed/loop changes affect only
  destination. See [samples/crossfade.rs](samples/crossfade.rs).
  `animations` uses explicit `incant_animation_v3` snapshot imports; controls
  still use v2. `transition_animations` provides the older v2 snapshot, and
  `legacy_animations`/`legacy_control_animation` remain available for v1-only hosts.
  Active timed blends make v1 length/read refuse; parameter blends make both v1
  and v2 refuse. The parser accepts all three exact layouts. Old hosts/parsers
  reject the new namespace/magic instead of silently dropping the second pose.
  All three versions explicitly refuse native weighted tree poses.
- `animation_poses(scratch)` uses `incant_animation_v4` for complete copied
  poses, including nested animation trees. It returns scene identity and sorted
  stable entity IDs; each row exposes `samples()` and an explicit pose kind.
  `Single` is one sample. `SlerpTransition` and `SlerpParameter` retain ordered
  source/destination operands and the existing exact two-pose SLERP semantics.
  `WeightedQuaternionMean` carries the native positive normalized weights for
  local translation/scale blending and normalized quaternion averaging. These
  kinds are not interchangeable. The row clip/time is representative only; use
  all samples for a weighted pose. Unknown kinds or reserved metadata fail
  explicitly. Copied clocks, flags and timed-transition metadata remain available.
  Queued controls do not change the current callback snapshot. See
  [samples/animation_poses.rs](samples/animation_poses.rs).
- `animation_graphs(scratch)` returns sorted entity/state IDs, the explicit master
  pause bit, state clocks, and sorted stable parameter IDs with captured values.
  `control_animation_graph` queues SetParameter/SelectState commands through the
  same atomic native transaction; authored parameter ranges and state/parameter
  IDs validate at commit. State selection resets its clocks but preserves the
  master pause. Parameter changes and manual selection do not alter the current
  callback snapshot. Definitions, asset tables and native handles stay private.
  See [samples/animation_graph.rs](samples/animation_graph.rs).
- `locale(result_scratch)` and `localize(request, request_scratch, result_scratch)`
  return copied committed-locale/message values. Text/Number/CalendarDate
  arguments and fallback/pseudo settings use the shared native ICU validation.
  `set_locale(settings, request_scratch)` queues an ordinary atomic runtime
  setting change, visible to the next accepted callback. Missing records use the
  enclosing result's requested/resolved locale. Maximum buffer constants are in
  `incant_module::localization`; smaller initialized scratch works when the whole
  frame fits. [samples/localization.rs](samples/localization.rs) is runnable with
  the same trusted compile helper and actual two-host conformance command.
- `query_physics(queries, request_scratch, result_scratch)` batches up to 64
  3D/2D rays and character movement queries against the committed native solvers.
  Typed results retain request order and borrow only caller-owned scratch.
  `physics_events(dimension, scratch)` observes complete sorted sensor enter/exit
  events without draining them. Copied event frames retain their scene ID and
  dimension, exposed by `scene()` and `dimension()`, including empty frames.
  Pending numeric/structural writes are invisible;
  apply returned character displacement through the ordinary columns. Types and
  scratch bounds are in `incant_module::physics`; the
  [physics sample](samples/physics.rs) exercises both dimensions and character
  movement. Invalid/caught requests or insufficient scratch reject the callback.
  The [public package proof](../../docs/spikes/native-wasm-physics-package.md)
  exercises ordinary authored import/export and fresh source-free shipping
  players against a direct native physics oracle.
- `find_path(owner, request, result_scratch)` queries the active scene's committed
  navmesh; `find_grid_path` queries its prepared grid. Both return `None` for a
  valid miss and a borrowed view of the complete copied route on success. Mesh
  paths retain points, corridor, visited count, generation and explicit off-mesh
  link IDs/point pairs/direction. Grid paths retain adjacent indices, generation
  and exact integer cost. All u64 metadata stays exact. Pending numeric/structural
  writes remain invisible until native commit. Queries do not move entities;
  gameplay must handle off-mesh traversal and grid-to-world conversion explicitly.
  Types and scratch maxima are in `incant_module::navigation`; the complete
  [navigation sample](samples/navigation_paths.rs) observes a rebuild after
  committed obstacle movement. Unknown owners, invalid inputs and exhausted
  budgets reject the callback, including when the guest ignores an SDK error.
- `steer_agents(request, request_scratch, result_scratch)` computes local-avoidance
  velocities over explicit world-space agent samples and obstacle polygons or
  segments. It never gathers collision geometry or moves entities implicitly.
  Results borrow initialized caller-owned scratch and are sorted by stable ID;
  IDs correlate samples and need not name existing entities. The callback's fixed
  step must be between 1/240 and 1 second. Reuse scratch across ticks, then apply
  selected proposals through ordinary copied numeric writes. See
  [samples/steering.rs](samples/steering.rs).
- `tick.log(message, LogLevel::Info)` copies primitive UTF-8 diagnostics into the
  current update. `Debug`, `Warn` and `Error` are also supported. Messages accept
  at most 1,024 bytes, LF/TAB but no other controls; the host accepts at most 64
  records/16 KiB per update. History retains a prefix of 1,024 records/256 KiB and
  counts later drops without failing gameplay. Records publish only after the
  entire numeric/structural update succeeds, carry its accepted update number,
  and reach the native session/report log observer. Failed hosts hide logs.
- Use `?` for SDK operations. Returning `Err` from the callback traps. An ignored
  error returned by an active SDK method also traps at callback exit, rejecting
  all its earlier numeric writes and queued commands. Pure snapshot `parse`
  methods only validate caller data; propagate their errors to reject a callback.
- Raw host import errors trap immediately. Expired epochs never become active
  again. A trap or invalid transaction permanently fails that module instance;
  guest globals/memory are not rolled back. The enclosing native session also
  remains terminal after a failure following accepted simulation. This does not
  undo simulation or device audio already advanced by an owning game session.
- No source/asset/configuration mutation, timers, physics/pathfinding/character APIs, authored
  instance props, audio rendering, complete gameplay SDK, arbitrary-memory save/restore, device
  performance or distribution gate is implied by this slice.

## Generated compatibility boundary

`sdk/abi/native-v1.json` is the authoritative registry for namespace/import
signatures, static dispatch opcodes, byte record layouts, section order, tags and
bounds. `tools/generate_wasm_sdk.py` generates the private Rust guest imports,
Wasmtime bindings, static Rust/C dispatch, namespace contexts, host input tag
mapping and shared endian codecs. Generated files are checked in. Run its
`--check` mode after changes. Runtime crates never read this JSON.

Additive `incant_animation_v1` supplies legacy skeletal playback snapshots/controls;
`incant_animation_v2` adds complete transition observations and crossfade control,
`incant_animation_v3` adds actual pose-blend observations, and
`incant_animation_graph_v1` adds copied graph state/parameters and atomic controls.
`incant_log_v1` supplies logging and `incant_localization_v1` supplies
copied locale/message queries and atomic settings. `incant_navigation_v1` adds
local-avoidance requests/results and committed navmesh/grid path queries. The
path imports append at opcodes 36–38; all previous operations remain unchanged.
`incant_physics_v1` appends bounded batch/event/result operations at 39–41.
`incant_animation_v4` appends complete pose snapshot length/read at 42–43; all
previous operations and layouts remain unchanged.
Older hosts reject unsupported imports explicitly.
Existing `incant_v1`, `incant_input_v1`, and `incant_flipbook_v1` layouts remain
unchanged. Unknown namespaces, signatures and magic/layout versions fail closed.
The host's native numeric layout is checked against generated sizes and offsets
at compile time. Actual legacy WAT controls and existing exact wire assertions
remain in the conformance suite, alongside registry signature and stale-file
checks. Changes to public wire contracts require a versioned compatibility plan,
not edits to generated output alone.

Bounds remain the native host's v1 limits: 32 MiB guest memory/copied columns,
128 MiB cumulative transfers, 65,536 calls, 1,024 commands, 4,096 chunks per tick;
input scratch at most 128 KiB and flipbook playback at most 32,768 rows.
Skeletal playback has at most 100,000 rows (v1: 4,000,016 copied bytes;
v2: 8,000,016; v3: 10,400,016 copied bytes), 256 clips
per entity and finite speed in [-16, 16]. Snapshot row order is unspecified;
use stable IDs. Callers should retain appropriately sized scratch across ticks. The SDK exports
snapshot maximum sizes for reusable buffers; `ANIMATION_MAX_BYTES` remains the
v1 bound, and v2/v3 have explicit versioned constants. V4 has sorted entity IDs,
at most 16 samples per row and at most 1,024 weighted graph rows. Its complete
frame is bounded to 14,744,104 bytes (`ANIMATION_V4_MAX_BYTES`), including at most
214,336 samples. Native sample order and f64 weights/times are retained exactly;
the parser validates the complete frame before exposing any iterator. The
connected standalone tree currently supplies up to eight active samples;
sixteen-sample wire capacity does not imply connected tree-state fades. Future
composite/grouped pose kinds require an explicit compatible version. Graph snapshots hold at
most 1,024 entities with 16 parameters each (458,776 bytes). Parameter values
are finite within ±1,000,000 and the narrower authored range. Elapsed state time
is bounded to 3,600 seconds; clip time may exceed this for longer clips. Host fuel and native validation
remain authoritative even for guests bypassing the safe SDK. The full native
scene's entity limit and script transaction rules still apply.

Localization has separate logical input/output quotas of 64 KiB/256 KiB per
callback, at most 64 queries and 64 settings commands, with 64 arguments and 16
fallbacks. Each query costs four of the 1,024 query units. Maximum wire buffers
include extra lengths/tags; repeated result copies consume the existing transfer
quota. Correct-epoch active localization ingress failures reject the entire
batch. `format_number(value, result_scratch)` and
`format_date(CalendarDate, DateLength, result_scratch)` return copied UTF-8 text in
the callback's committed locale. `DateLength::{Short, Medium, Long}` uses generated
registry tags; Medium matches TypeScript's default. Values must be finite within
the JS-safe integer magnitude; calendar years are -9999..9999 and actual Gregorian
validity is checked natively. These calls do not pseudo-wrap their results. A
queued locale change affects only later callbacks. Formatting shares existing
query/output quotas, charges no logical input bytes (matching TypeScript), and
charges its fixed 24-byte request and result copies to transfer limits. See
[samples/formatting.rs](samples/formatting.rs). Catalog edits and report
enumeration remain unsupported.

Physics uses the same 1,024 native query units as navigation/localization, one
unit per batch row or event observation, with at most 128 character queries per
callback. A batch holds at most 64 queries and 4,096 aggregate returned collision
IDs. Requests/results are at most 7,184/71,696 bytes. Events preserve the native
131,072-row bound (5,242,912 bytes); provide smaller scratch for smaller frames.
The existing native ray and character scalar domains apply before narrowing,
including the character displacement limit against the actual callback dt.
Queries preserve masks, optional exclusion, sensors, autostep, ground snap and
slope settings. Events describe the last committed physics step, not pending
movement. The SDK provides no solver handle or implicit movement.

Navmesh, grid and local avoidance share 32 requests per callback. Path searches
also share 65,536 requested expansions, charged even for misses. Mesh requests
permit finite coordinates within ±100,000, snap distance 0.001–10 and 1–32,768
visits; grid cells are u16 indices with 1–65,536 expansions. Native owner/grid
bounds remain authoritative. There are at most 4,096 points, corridor entries,
traversal records or grid cells. Maximum copied results are 245,800 mesh bytes
and 32,800 grid bytes. A short destination never receives a partial frame.

Local avoidance is additionally bounded per callback to 128 aggregate agent
samples and 128 aggregate obstacle edges. Each request has at most 32 obstacles;
closed polygons have 3..32 vertices and segments exactly two. The copied request
and result maxima are 17,192 and 5,136 bytes. Scalar domains, duplicate IDs and
geometry are validated by the same native solver as TypeScript. Each request
charges one shared query unit, as well as the existing call and transfer budgets.
A new request replaces the latest reply; replies never survive callback expiry.
Correct-epoch ingress or output errors reject the complete update even when a
caller ignores the error. Stale epochs are rejected without poisoning a different
currently valid update. This is local avoidance only; navmesh path queries,
automatic obstacle collection and debug drawing remain separate capabilities.

### Complete compound poses and outgoing graph clocks

`Tick::animation_poses_v5(scratch)` returns ordinary clip/pair/weighted poses and
compound poses. Each row contains one or two `AnimationPoseGroup` views with an
explicit `PoseInterpolation` (`Clip`, `SphericalPair`, `Weighted`). A compound
row has two independently evaluated groups and `outer_weight`, the destination
contribution of their TRS crossfade. Do not flatten them: a weighted quaternion
mean and pairwise SLERP have different numerical semantics. Inactive pair and
outer endpoints remain present. Both compound groups contain at most eight
samples. The general typed weighted-pose contract permits sixteen samples in
one standalone mean, matching `AnimationPoseSamples::new` and `blend`; the
authored graph-state limit remains eight.

`Tick::animation_ik_v2(scratch)` contains that entire v5 frame and every ordered
model-space constraint, with the same `set_animation_ik` controls. Standalone
pose readers, including v5, refuse committed IK. `animation_graphs_v2(scratch)`
returns current graph rows/parameters plus sorted outgoing state/clock records.
Both groups use the captured current parameter values. Existing graph controls
remain unchanged; they do not alter the current callback's captured frames.

Operations 47–52 append these three length/read pairs. All earlier operations and
wire layouts remain unchanged. Old graph/IK frames that cannot represent a
compound pose refuse the whole callback, even after a complete-frame cache has
been populated. Entire-frame validation checks sparse associations, canonical
owner ordering, duplicate IDs, sample/group bounds and all numerical fields
before exposing borrowed views. An invalid active call or caught SDK error
rejects the callback's entire staged numeric/structural/log prefix.

These copied observations do not grant chain-topology authority or persist Wasm
memory. Arbitrary whole-module snapshots remain explicitly unsupported. See
[samples/animation_compound.rs](samples/animation_compound.rs) for complete
captured reads and controls through the real native host.

`Tick::root_motion` copies the complete `incant_root_motion_v1` frame: unchanged
pose-v5 and ordered IK-v2 data followed by canonical root owner rows. Evaluate
the native pose, strip the selected root translation/rotation to its fingerprint-
bound cooked rest (preserving sampled scale), then apply the ordered IK rows.
The rows also expose movement policy, applying flag, exact `u64` ticks and signed
`i64` cycles with stable current/outgoing clock owners, plus requested/applied
interval and accumulated rigid deltas. Application toggles never disable root
stripping or replay past movement. `control_root_motion` participates in the same
atomic command quota and sticky callback-error boundary as other native controls.

Every older pose/IK snapshot import explicitly refuses a world with committed
root stripping, including ExtractOnly and disabled application. Graph-only
observations remain available. The new complete frame accepts ordinary worlds
with an empty root table. Callers supply bounded scratch; no native pointer,
model loader or per-tick source/document parser is introduced. Physics and static
guest heap checkpoint limitations are unchanged. Actual dual-host and public root
package evidence is recorded separately; API availability alone is not that proof.

### Audio declarations and transport

`Tick::audio(scratch)` copies the desired committed audio declarations at callback
entry: scene/tick, stable entity/clip/bus identities, exact `u64` transport epochs,
bus parents/gains, source options, spatial positions and listener rotation. This
is not a mixer cursor or voice-status API. An ended one-shot can still declare
`playing = true`; use `Restart` to request another playback. The bounded frame is
at most 32,240 bytes (32 buses, 128 sources and one listener). Its checked views
borrow caller-owned scratch and remain unchanged by commands queued afterward.

`Tick::audio_commands(commands, scratch)` queues an ordered batch of
`AudioCommand { entity, control }`. `AudioControl` provides `SetPlaying`, `Stop`,
`Restart`, `Seek`, `SourceGain` and `BusGain`. Stop/restart/seek increment the
checked native transport epoch; pause/resume preserves it. Gains validate the
original `f64` range before native `f32` conversion. Seek admits finite seconds
in `[0, 3600]`; synchronization with the captured clip additionally checks its
actual duration. That later mixer failure is terminal, not an atomic callback
rollback claim.

The new `incant_audio_v1` operations 59–61 preserve every older import/layout.
A batch has at most 1024 commands and needs 16 + 32 × count scratch bytes.
Audio shares the existing command/call/transfer quotas. Invalid authorized host
calls and caught SDK scratch/argument errors discard the whole callback's staged
numeric, structural and log changes; short reads never copy a partial frame.
Queued commands never call an audio backend directly. See the real Rust
[transport sample](samples/audio_transport.rs) and
[scope/evidence](../../docs/spikes/native-wasm-audio.md).

### Explicit persistent state (native qualification pending)

`persistent_state!(schema = [5; 32], bytes = 16)` opts a guest into a fixed
host-owned state region. Choose a nonzero 32-byte schema identifier and a length
of 1–65,536 bytes for the actual application layout.
`Tick::read_persistent(offset, buffer)` and `write_persistent(offset, bytes)`
require a fresh tick capability. Reads observe earlier writes in that callback;
bytes publish only with the successful native transaction. Current-capability
errors poison that transaction even when ignored. Expired tokens cannot access
or invalidate the new tick. State calls are bounded to 256 operations and 1 MiB
of transferred bytes per tick, within the shared native call budget.

Capture reads committed host bytes without calling guest code. Ordinary loads
and native checkpoint reconstruction create a fresh VM, then restore the
validated region, world clock and capability watermark before any update.
Globals, tables, heap objects and retained pointers restart; they are not saved.
Guests without the declaration keep the explicit state-ABI refusal. A pending
restart request refuses capture without consuming the request. Same-layout
reload preserves committed bytes; changing byte length/schema refuses reload.

See [samples/persistent_state.rs](samples/persistent_state.rs) for typed byte
encoding and [the implementation scope](../../docs/spikes/native-wasm-persistent-state.md).
Source controls exist for both native hosts, but their execution and a new
ordinary shipping export remain pending. This API is not evidence of those gates.
