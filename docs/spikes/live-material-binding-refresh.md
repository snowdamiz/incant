# Live model-local material bindings

Native Play can now refresh an existing model's primitive-to-material assignment
and complete model-local material, image, texture and sampler tables without
replacing the running ECS world or script VM. Real importer-backed controls cover
table growth, reordering, removal to glTF's implicit default, re-adding maps and
all five supported map roles. The bounded model decoder remains authoritative for
indices, resource roles, finite values and limits.

Scene/node/primitive membership, project asset identity, skeleton/rest/influence
layout and clip bindings/durations remain constrained by the existing refresh
contract. This is an extension of the cold complete-catalog preparation and
borrowed publication path, not a mutable renderer shortcut. Preparation loads and
builds resources outside authoring locks; final revision, session, world, catalog,
binding and consumer checks still guard atomic publication. Rejected candidates
retain the complete accepted catalog. Retirement remains outside the caller's
publication lock. `incant_cmd` import, watch, Undo and Redo use the same authority.

## Navigation provenance

Static navigation recipes use startup geometry. Material-only changes may refresh
a navigation source only while its accepted visual geometry has never diverged
from startup during this Play session. The comparison is deliberately bitwise and
conservative over all vertices/indices and node transforms: normal, UV and signed
zero changes can count as geometry changes. Animated navigation remains refused.

A bounded sorted per-model divergence latch travels with the ticket and catalog
generation. Both staging and final publication recheck current navigation
dependencies. Thus accepting geometry G0→G1 before adding startup-bound navigation
cannot make a later G1 material change appear safe. Restoring G0, including through
Undo, does not clear that conservative latch; a fresh Play load does. Refusal is
scoped to the model actually being refreshed, so another unchanged latched model
does not block an unrelated texture-only publication. The latch is prepared cold
and swapped with caller-owned retirement storage at publication.

## Executed qualification

The numerical producer is **`8e5745bd38b829e71d8d3843905ed5731ef4c17b`**, a clean
main354 composition. Its material production code is unchanged from `f2db3fe3`.
The main354 archive integration adds opt-in renderer plumbing and an ordinary
Metal HAL pipeline-construction refactor; the GPU-host checks below compiled that
composition with only `default,physics-deterministic` features. Main355's separate
TypeScript-loader observation is outside these executed source identities.

The final review composition **`3d4c0d195b69d7304c92b048c9a9c8e5f45d24cc`**
joins main355. Its compiled-input delta is one bounded Rust phase-observation
parser and two Node compiler-driver/milestone files, separately qualified on
main355. Material/runtime/render/HAL, Cargo and toolchain inputs are unchanged.
The packet retains this exact delta; no native rerun of the joined composition is
claimed. All 160 inventoried producer inputs remain byte-identical.

| Evidence | Actual result and producing source |
| --- | --- |
| Native adapter + Play/import/history | **37 controls pass** at `14cc7bda`: native 9 animation + 11 model + 7 texture; Play 1 animation + 4 model + 5 texture. Joint strict all-target Clippy passes. |
| GPU fixture admission | **1 imported-data CPU control** and strict all-target GPU-host Clippy pass at final `8e5745bd`. The ignored GPU body is compiled by this target. Earlier successful fixture checks at `a2722aaf` and `30da0b60` remain separate records. |
| Publication lifecycle | CPU observers cover zero allocations/disposals during final adapter publication, old-version validity, mixed catalog failure, stale/foreign tickets, publication unwind, readiness refusal and current navigation changes. These do not claim allocation-free cold GPU staging. |
| Actual GPU | **Apple M5 Pro / Metal**, one numerical headless control: 7 accepted frames/presents, six 160×96 raw RGBA captures. **612 pixels** change red→blue→green with exactly equal primary masks and alpha. Late consumer and stale catalog refusals retain complete accepted RGBA exactly. |
| Running state and temporal provenance | Material publications occur at tick1; the next script tick reaches tick2 and persistent calls1+2 yield translation0.3. World identity is preserved. Each first replacement draw has a fresh history owner and no previous history; no submitted motion frame exists before that redraw. Old catalog handles remain valid, staged resources survive owned source/cache deletion, and the target drops once on stop. |

The same world/VM evidence, real authoring/history controls and numerical GPU
control are separate checks. No native-window import/rebind workflow, GUI clicks,
Blend appearance, appearance review, performance or phase gate is claimed.
Opaque reassignment and fully covered MASK rendering are exercised numerically;
implicit defaults, shrinking/reordered tables and Blend data are CPU coverage.

## Preserved failures and limits

* **CPU001 / `f2db3fe3`:** 8 existing animation controls passed; the new material
  fixture correctly refused because adding a normal map without authored tangents
  regenerated geometry. Source002 authors stable tangents and adds an explicit
  generated-tangent refusal. The production geometry guard was retained.
* **CPU003 / `a2722aaf`:** the fixture control passed, then strict Clippy rejected
  duplicate loading of a shared test module. `30da0b60` shares that module through
  test-only visibility/imports; fixture data and assertions are unchanged.
* **GPU001 / `30da0b60`:** initial and late-refusal raw pixels matched exactly.
  The test then incorrectly unwrapped the new cold scene's motion frame before
  its first submission. `8e5745bd` asserts the source-defined absence and retains
  the old snapshot properties; all next-redraw, pixel and state checks remain.
  The enclosing failed receipt is not relabeled as passing.

The original 256MiB cumulative executable-copy limit was explicitly raised to
320MiB for the module-sharing successor, retaining all prior copies; final charge
is **313,245,720 bytes**. Shared CORE target growth, including root's intervening
qualification, remains charged against the original 4GiB ceiling. GPU002 deducted
a conservative two seconds for GPU001 from the original 120/180-second limits
and retained prior log/pixel charges. Every owned child/group was independently
absent after cleanup. These are resource and correctness observations, not timing
or cache-hit evidence.

## Portable replay

[The compact packet](evidence/live-material-binding-refresh/manifest.json) retains
source inventories and changed snapshots, commands, exact logs, failure receipts,
raw pixels and executable identities. Executables remain outside Git. Its source
inventory is bounded to the recorded 160 inputs, not a complete transitive
compiler/toolchain capture. Default replay uses saved bytes only; optional modes
check retained originals, external binaries and the named Git/live source inputs.

```sh
python3 docs/spikes/evidence/live-material-binding-refresh/verify.py
python3 docs/spikes/evidence/live-material-binding-refresh/verify.py \
  --originals --binaries --git-source 8e5745bd38b829e71d8d3843905ed5731ef4c17b \
  --source-root .
```

The live-source option intentionally refuses changed inventoried inputs. A later
integration must describe any other compiled-input differences separately; replay
does not turn later source into the producer or satisfy a wider gate.

## Native workflow fixture preparation

The shared native editor runner now accepts `--asset-kind model --model-variant
material-tables`. Its two bounded, self-contained glTF inputs retain identical
positions, normals, UVs, authored tangents and node membership while replacing
material, image, texture and sampler tables and reassigning the primitive. The
replacement uses all five material map roles. It drives the existing real model
import/watch/Undo/Redo/recovery handlers; there is no new editor mutation API.

Fifteen focused Python controls pass. The retained main376 headless executable
(`7cfe5b13…`) imports both actual fixtures, retains the asset ID, advances the
revision from 1 to 2, and treats a repeated replacement import as unchanged.
Cooked texture-role counts change from 2 to 5; project validation passes. The
first local runner stopped on a disappearing transaction scratch file in its
resource monitor. Its cleanup passed; a fresh successor allows that expected
sampling race and completes all five child commands. Both attempts are retained
in [the fixture evidence](evidence/native-material-workflow-fixture.json).

This is fixture admission, not the still-pending actual native-window material
workflow. The prior CPU and numerical Metal evidence above retains its original
producing sources.
