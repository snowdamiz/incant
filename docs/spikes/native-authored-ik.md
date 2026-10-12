# Connected native two-bone IK

This slice connects the accepted CPU solver to authored projects, sealed model
cooking, native state, TypeScript controls, binary checkpoints, and the canonical
model consumer. It remains nonvisual: no GPU execution, rendered appearance,
mobile budget, animation authoring UI, retargeting or full Phase 1 gate is claimed.
Affected CPU checks and the ordinary public export/continuation proof passed.
Independent source and final evidence review are accepted. Current-main
integration remains a separate requirement.

## Declaration and runtime ownership

`AnimationIk` requires `Animation` and `MeshRenderer`. Each ordered constraint has
a stable canonical ULID, immediate root/middle/tip cooked model-node indices,
model-space target and pole, and weight in `[0, 1]`. Model space is before the
entity/world transform; these are not world coordinates or entity IDs. The
constraint order is significant. There are at most 32 constraints per owner and
4,096 across the scene. Node indices are below 4,096; target/pole coordinates are
finite and bounded to absolute `1e9`.

The authoring path uses ordinary `incant_cmd` transactions. Startup resource
loading validates against the actual fingerprinted cooked model. It uses the
same solver to admit rest-pose hierarchy, editable local TRS, nonzero bone
lengths and supported positive uniform orthogonal transforms. It retains only
a sealed binding, not the model sampler. Stale resource/declaration pairs refuse.
An animated pose can later make a chain unsupported (for example a reflected
joint scale); the canonical consumer explicitly rejects that candidate. It does
not skip the IK or stretch bones. The previous accepted consumer input survives.

The native ECS retains an immutable declaration plus an independently replaced
numeric constraint slice. Structural replacement, removal, despawn and entity
reuse follow the same final-state validation and transaction as other components.
Changing unrelated spatial configuration preserves numeric IK state. Multiple
controls for one owner are coalesced into one candidate slice, in queue order;
a later invalid control rejects all earlier controls and numeric tick edits.

`tick.animations.ik` provides owned entity/constraint IDs and packed chain,
target, pole and weight columns. `commands.setAnimationIk` accepts primitive
arguments and shares the existing command quota and caught-error poisoning.
Guest mutation cannot change native state or a later snapshot. The API names
model space explicitly. Rust/Wasm IK control/query ABI integration remains open;
this slice does not advertise those methods through a different ABI. Existing
Wasm pose length/read imports explicitly refuse when IK is present, without
publishing a partial pose or accepting an earlier numeric transaction prefix.
Other imports can still operate. This source baseline has pose v1–v3; publisher
integration must include the identical refusal for current-main v4.

## Binary identity and compatibility

Cooked scene v16 appends bounded IK bindings after the existing graph section.
Each owner row has 52 prefix bytes and each constraint has 78 numeric/identity
bytes. The reader checks the remaining bytes and aggregate count before
constraint allocation. Scenes without IK keep their existing v1–15 writer choice.

World save v9 adds a four-byte count and 94 bytes per committed constraint
(owner ID plus the same 78-byte constraint). Exact owner membership and ordered
constraint ID/chain pairing are required. Existing non-IK v1–8 output is retained.
Session v1–3 framing is unchanged; its trusted-world admission now permits v9.
Existing physics/module/device and other unsupported continuation boundaries are
not lifted.

Trusted session/live restore additionally compares each IK binding with the
original cooked catalog: model fingerprint, ordered constraint IDs and chains.
Numeric target/pole/weight values remain mutable. Spawned owners can reuse an
existing exact catalog binding. A low-level structural edit that invents a new
binding requires a new authored cook before it can be checkpointed under that
original catalog. Raw world decoding remains data validation; hashes/origin
fingerprints are compatibility and integrity checks, not authentication.

## Canonical consumer and allocation scope

The native adapter transports a shared immutable numeric slice through cold,
motion and rollback paths. The canonical model path samples the ordinary clip,
pair blend or weighted tree first, then applies ordered IK, then deforms checked
vertices. Existing static-model guards reject inappropriate animation/IK data.
No shader, material, shadow, visual defaults or GPU policy is changed. The
authoring edit-mode adapter still has no IK preview; only native Play/game
consumption applies this connected state. Edit-mode IK preview remains open.

Unchanged native extraction/reuse performs zero measured allocations. The solver
and preallocated geometry loop also perform zero measured allocations. Changing
IK targets is a different path and does allocate; the implementation does not
claim an allocation-free complete animation tick. The following local CPU
instrumentation counts requested allocations/reallocations and cumulative
requested bytes, not live heap/RSS, shipping timing or device performance.
Each warmed batch below contains two commands per owner; 128 batches are run
while retaining each previous immutable binding through candidate publication.

| Owners | Constraints/owner | Allocations | Cumulative requested bytes |
| ---: | ---: | ---: | ---: |
| 1 | 1 | 256 | 59,392 |
| 1 | 32 | 256 | 376,832 |
| 8 | 1 | 1,152 | 145,408 |
| 8 | 32 | 1,152 | 2,684,928 |
| 128 | 32 | 19,072 | 43,231,232 |

A numeric sample is 80 resident bytes on this target. Replaced Arc slices add a
16-byte header, and the per-batch ordered staging map allocates nodes. On this
target the measured map requests were 368 bytes for one/eight owners and 8,016
bytes for 128 owners per batch (21 map allocations in the latter case); these
are observations of the pinned implementation, not portable allocator contracts.
Candidate numeric payload is capped at `4096 * 80` bytes plus Arc headers and
bounded map nodes while previously accepted payloads remain valid. The
initial one-owner/two-constraint run recorded 260 allocations / 70,057 requested
bytes, including initial queue growth; the warmed table separates that effect.
A future bounded spare-buffer pool could reuse storage when no old consumer
holds it, but would need explicit resident accounting and a policy for retained
immutable frames. No such ownership change is claimed here.

## Acceptance boundary

Focused executed controls include actual source-deleted model solving/deformation,
sampled reflection/degeneracy refusal, canonical native rollback, script traps and
caught invalid calls, stale resource refusal, structural replacement/reuse,
recomputed-checksum malformed saves and catalog mismatches, and global caps.
The public helper used only command-bus authoring plus ordinary `export_native`
and packaged CPU player routes. It exercised real import, atomic authoring,
Undo/Redo, journal reopen and 40 ticks of native Play. After deleting its own
authoring/cache directory, source map and disposable build storage, the exported
9,268,592-byte player ran four fresh processes: uninterrupted 40 ticks, an initial
13 ticks, and two independent 27-tick continuations. The final three saves were
byte-identical; full PCM was exactly the 83,200-byte prefix plus 172,800-byte
continuation. Script state, timer events, accepted IK logs and retained native
render input also matched. PCM here verifies the composed clock, not audible
content. This CPU player observes accepted native render input; actual vertex
deformation is separately tested through the canonical CPU model path.

Seven nonzero process controls matched exact diagnostics: corrupt checksum,
wrong program, recomputed-checksum constraint identity and chain mismatches,
oversized input, failed observation assertion and existing save destination.
Rejected inputs did not publish PCM or a requested save. The existing-save case
preserved its bytes; PCM may already have been published because PCM and
checkpoint files are separate publications, not a multi-file transaction.

The final affected suite passed 643 tests in 104 groups with 82 GPU cases ignored.
All-target Clippy, runtime wasm32, static-module script checking, 68 fresh schema
identities, strict SDK/proof TypeScript, bridge generation, formatting,
conventions and authoring-free dependency checks passed. Historical test-fixture
compile errors, allocator-instrumentation correction and version-sentinel
failures are retained; the final full run passed after those corrections.

[Evidence and exact commands](evidence/native-authored-ik.json),
[public process receipt](evidence/native-authored-ik-public-export.json) and
[independent final review](evidence/native-authored-ik-final-review.json)
bind the frozen sources, build/player inputs, raw/compressed logs and physical
package hashes. The actual proof is retained under
`~/.codex/incant-evidence/native-authored-ik-shipping-001`.
The earlier [source review](evidence/native-authored-ik-source-review.json)
retains the closed v1–v3 omission finding and explicit v4 integration requirement.

Reproduce with a matching built `incant_headless`:

```sh
python3 tools/probes/native-animation-ik-export.py /absolute/new-proof-directory \
  --binary /absolute/path/to/incant_headless
```

The helper also supports `--existing-package /absolute/proof/package` for
precise control re-verification without rebuilding unchanged runtime inputs.
This run passed on its first fresh package; no fallback verification was needed.
