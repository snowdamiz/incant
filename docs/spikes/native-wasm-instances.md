# Authored native Wasm initial instances

Implementation based on main409 (`dec42101f`). Scoped native CPU tests, actual
Wasmtime/wasm2c guest comparisons, strict lints, fresh headless schema comparison
and one ordinary authored source-free export passed. No
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
conformance preparation builds these for actual Wasmtime/wasm2c/TS comparison.
All four focused actual-guest tests
passed, including same-module reload after removal followed by same-ID respawn.

Source-only checks: 43 focused Python controls passed after resolving the test
fixture's temporary-directory alias; the original path refusal was preserved in
the development record. Generated ABI/SDK, format and conventions checks passed.
The fresh headless producer now matches all published schema bytes. Its first
check refused two lexical differences in `agent-tools.json`; parsed JSON values
were identical, and the actual generator's byte spelling was restored.

The [compact qualification record](evidence/native-wasm-instances.json) retains
exact commands, counts, source/artifact identities, log hashes and local original
paths. Initial `9ee0f8ea` passed 35 tests, then strict lint refused an unused
iterator result. `f42c8bee` explicitly discards that eagerly validated view and
adds removal-tick checkpoint and common reload regressions: seven focused runtime
tests and enabled/ordinary strict lints passed. The headless build first hit its
150-second cap; its bounded continuation built successfully. Generated-byte
correction `00425f75` changes no runtime code. At `b03dda12`, six Rust guest
variants were compiled/metered/translated, four actual two-host tests passed and
their strict feature lint passed. These are CI-profile functional results.

Total recorded native work was 1,031.509414 seconds, including refusals and the
headless timeout; last owned target allocation/peak was 6,785,257,472 bytes under
a 12 GiB cap. Each actual wrapper terminal was observed and all recorded process
identities/groups were checked absent. Current headless, cooker, runtime-test and
guest-test binaries have independent retained copies. The original five-test
runtime executable was overwritten by its approved seven-test successor; its
source, output and hash remain, but those historical binary bytes do not.

The retained exporter is the exact `f42c8bee` producer, SHA-256
`31fac9b9ba1209287377e6fd6aee58f34d2c0121780288a42002a3546438b27e`.
It is usable only for source-compatible authoring/export checks. This does not
qualify alpha/tilemap additions or relabel it as a current-main executable.
Root's separate combined integration `9d44372a` subsequently built a fresh
headless producer and matched all 80 schemas, preserving both additions without
another snapshot change. That producer is SHA-256
`f0fb0cf5c654dd03e4ad77e57901586cb6abb426a923dd77f86af169f272641f`;
its separate build does not rerun or replace the original guest results.

## Ordinary authored export

The [export proof record](evidence/native-wasm-instances-export.json) retains the
raw authoring/RPC receipts, four gameplay reports, PCM/checkpoint bytes, exact
export/deletion identities and independent process closeout. Its fresh shipping
source is `28806e21`, whose native code equals the combined `9d44372a` producer.
The copied `9d44372a` headless is used only for public authoring; the unchanged
exporter builds its own cooker and shipping player in fresh owned storage.

Ordinary `init`, Wasm/WAV import, idempotent reimport and `command.execute`
configured two typed owners, an unrelated entity and a looping audio source.
Invalid-owner and stale-revision edits preserved the project/revision. Undo/Redo,
save and journal reopen preserved the exact authored catalog. The cold instance
hash matched the independent canonical frame, including nested properties.

The packaged retire guest removes owner one on tick 1, then respawns the same ID
on tick 2. A checkpoint taken immediately on removal excludes that owner; fresh
resumes keep the replacement retired at position zero. Owner two advances by 20
per tick and the unrelated entity stays at zero. This fixture removes owner one
before its position can be observed; the earlier normal-guest comparison supplies
the separate observed 10-versus-20 tuning result.

All four standalone runs passed: six ticks uninterrupted, one tick to the removal
checkpoint, and two fresh five-tick continuations. The final checkpoints match
byte-for-byte. Full offline PCM equals prefix plus resumed PCM, both resumes
match, and every sample is finite with non-silent output. This does not prove
physical audio playback. A checkpoint with the canonical speed-11 catalog digest
and valid outer integrity was refused by the original speed-10 package with the
specific `instance catalog mismatch` error and no new PCM/save files. It changes
saved data, not the receiving program; the earlier two-host test separately
checks a changed receiving catalog.

Normal dependency, codegen, bytecode-symbol and source-deletion guards passed.
Build storage was removed before the exporter's startup check; owned authoring,
cache and WABT copies were removed before the four replays. The repository and
qualified guest remain retained, and a restricted child environment is not a
filesystem sandbox. Runtime package files remained unchanged after every run.

Actual execution `41617` ended with exit 0 in 320.926801 seconds. Independent
post-terminal checks found all 236 observed PIDs and 30 owned groups absent,
including the wrapper, and all five protected binary copies still matched.
Export-owned scratch peaked at 3,767,926,784 bytes and ended empty; sampled RSS
peaked at 7,254,212,608 bytes, within the 8 GiB limits. Existing CI targets were
not written. The retained player is 9,966,224 bytes, SHA-256
`94cb06ebd7681a18ddb9e4147fd0c99ce58df3b14106e9746cc2ca52b7294a60`.
These are bounded functional results, not performance or complete SDK-parity
acceptance.

## Integration review

Independent [source](evidence/native-wasm-instances-source-review.json),
[retirement/reload](evidence/native-wasm-instances-retirement-review.json) and
[actual-guest/integration](evidence/native-wasm-instances-integration-review.json)
reviews found no blocking issue. The latter rehashes all four retained executables
and verifies 60 identical Wasm source paths; only the dependency lock and agent
registry compose with the merged alpha tool. The fresh combined producer passes
all 80 schema checks ([terminal record](evidence/native-wasm-instances-combined-schema.json)).
All ten [local development checks](evidence/native-wasm-instances-development.json)
also pass on the same combined source tree. Subsequent edits retain documentation
and evidence only. Ordinary authored shipping qualification remains separate.
