# Authored native Wasm initial instances

Source implementation, based on main409 (`dec42101f`). Native compilation,
Wasmtime/wasm2c execution and packaged export qualification are pending. No
hardware input, window, GPU, performance or complete SDK-parity claim follows.

`SetSceneProgram` accepts the existing scene-wide Wasm module plus an optional
`instances` map from an existing scene entity ULID to immutable initial props:

```json
{
  "op": "set_scene_program",
  "scene_id": "00000000000000000000000009",
  "program": {
    "kind": "wasm",
    "module": "00000000000000000000000004",
    "instances": {
      "00000000000000000000000001": {"speed": 10},
      "00000000000000000000000002": {"speed": 20}
    }
  }
}
```

This uses ordinary `incant_cmd` validation, revision/provenance and Undo/Redo.
The module must still identify an imported Wasm asset. Props support null,
boolean, finite number, UTF-8 string, array and sorted-key object values through
the existing `CoroutineValue` representation. TypeScript and Wasm share the
factored canonical encoder and its existing aggregate limits: 32,768 owners,
65,536 property nodes, 1 MiB budgeted property bytes and depth 16. The byte
budget includes the existing key/container overhead; it is not a promise that
a 1 MiB string fits. Unknown owners, excessive values and mixed per-entity
TypeScript scripts refuse the whole authoring transaction.

The cold cook pairs canonical binary props with the exact cooked-scene SHA-256.
Authored native builds embed that binary; the player reads no project JSON,
source path or guest-selected executable. Development Play uses the same cold
preparation. Module-only JSON omits the empty map and retains its previous
selection and persistent-state framing. TS fingerprint encoding and limits have
explicit regression test sources; the newly compiled runtime/code identity will
still change normally with this source revision.

`Tick::instances(scratch)` copies one checked `INCINS01` snapshot through the
additive `incant_instances_v1` imports (operations 75–76). Views borrow only the
caller's initialized scratch. The snapshot is captured before queued commands,
uses sorted owner IDs and includes nested read-only property views. A complete
frame is at most 3 MiB. Shared call/transfer limits also apply. A short or invalid
current-epoch copy poisons the transaction, including staged numeric writes;
stale epoch calls cannot authorize access or poison a later accepted epoch.

Startup binds each authored owner to its exact runtime entity incarnation.
Committed despawn permanently retires that membership, including despawn and
same-ID respawn between callbacks. Previously copied snapshots remain historical
data. There is one scene-wide callback even if no owners remain; this is not
dynamic attachment, multiple-module dispatch or TypeScript source scheduling.

Checkpoint identity includes the canonical catalog digest. Catalog-bearing
persistent state uses `INCWPS04`, wrapping the existing v2/v3 state plus the
digest and active owner IDs. Restore validates all data before publication,
rebinds only saved active IDs to the receiving world, and refuses a changed
catalog. Reload retains accepted membership only for the same trusted catalog.
Plain module state continues to encode v2/v3 and decode older valid formats.
The existing total session-checkpoint cap remains; simultaneous maxima across
every optional subsystem are not promised.

Test sources cover ordinary SetSceneProgram/save/Undo/Redo and atomic refusal;
cold cook with nested values and signed zero; legacy TS fingerprint/limits;
snapshot bounds, stale epochs, rollback, owner reuse and checkpoint identity.
The Rust guest fixture has two differently tuned owners and an unrelated entity,
plus retire, scratch, callback-error, trap and stale-token variants. The existing
conformance preparation now builds these for actual Wasmtime/wasm2c/TS comparison
when native qualification is granted. Those guest runs have not yet occurred.

Source-only checks: 43 focused Python controls passed after resolving the test
fixture's temporary-directory alias; the original path refusal was preserved in
the development record. Generated ABI/SDK, format and conventions checks are
recorded in the source handoff. The JSON schema snapshots are source-derived;
authoritative comparison with a freshly compiled headless generator is pending.
