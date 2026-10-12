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
  instance props, audio rendering, complete gameplay SDK, save/restore, device
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
v1 bound, and v2/v3 have explicit versioned constants. Graph snapshots hold at
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
