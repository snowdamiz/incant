# Incant Rust module SDK

`incant-module` is a public, allocation-free `no_std` guest API for the native
capabilities implemented so far. Add a path dependency on `sdk/rust`, compile a
`cdylib` for `wasm32-unknown-unknown` with panic abort, provide a panic handler,
and export a callback with `incant_module::module!(update)`.

```rust,ignore
#![no_std]
use incant_module::{Result, Tick, IDENTITY};
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
incant_module::module!(update);
fn update(tick: &mut Tick<'_>, dt: f64) -> Result<()> {
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
and bulk flipbook playback/controls. `tools/rust_guest.py::compile` builds the
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
- Use `?` for SDK operations. Returning `Err` from the callback traps. An ignored
  error returned by an active SDK method also traps at callback exit, rejecting
  all its earlier numeric writes and queued commands. Pure snapshot `parse`
  methods only validate caller data; propagate their errors to reject a callback.
- Raw host import errors trap immediately. Expired epochs never become active
  again. A trap or invalid transaction permanently fails that module instance;
  guest globals/memory are not rolled back. The enclosing native session also
  remains terminal after a failure following accepted simulation. This does not
  undo simulation or device audio already advanced by an owning game session.
- No source/asset/configuration mutation, timers, query/character APIs, authored
  instance props, audio rendering, complete gameplay SDK, save/restore, device
  performance or distribution gate is implied by this slice.

## Generated compatibility boundary

`sdk/abi/native-v1.json` is the authoritative registry for namespace/import
signatures, static dispatch opcodes, byte record layouts, section order, tags and
bounds. `tools/generate_wasm_sdk.py` generates the private Rust guest imports,
Wasmtime bindings, static Rust/C dispatch, namespace contexts, host input tag
mapping and shared endian codecs. Generated files are checked in. Run its
`--check` mode after changes. Runtime crates never read this JSON.

Existing `incant_v1`, `incant_input_v1`, and `incant_flipbook_v1` layouts remain
unchanged. Unknown namespaces, signatures and magic/layout versions fail closed.
The host's native numeric layout is checked against generated sizes and offsets
at compile time. Actual legacy WAT controls and existing exact wire assertions
remain in the conformance suite, alongside registry signature and stale-file
checks. Changes to public wire contracts require a versioned compatibility plan,
not edits to generated output alone.

Bounds remain the native host's v1 limits: 32 MiB guest memory/copied columns,
128 MiB cumulative transfers, 65,536 calls, 1,024 commands, 4,096 chunks per tick;
input scratch at most 128 KiB and playback at most 32,768 rows. The SDK exports
snapshot maximum sizes for reusable buffers. Host fuel and native validation
remain authoritative even for guests bypassing the safe SDK. The full native
scene's entity limit and script transaction rules still apply.
