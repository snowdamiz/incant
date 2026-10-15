# Recorded ModelMaterialV1 preparation and asynchronous fallback

The opt-in route records actual model-material selections during native CPU
gameplay, admits the resulting level manifest before GPU construction, prepares
the recorded keys at load, and compiles later missing keys on one owned worker.
Frames select a finite fallback until the real pipeline is published. Ordinary
renderer construction keeps the existing eager catalog. This is a complete small
route for `ModelMaterialV1`, not qualification of every pipeline family or the
plan's shader-stall/performance gates.

The tested source is `11d4f5f626319b014656da24064e26478e2e83de`, incorporating
main358 `552baceba079f5d47555047204a865491401442c` and the live material-refresh
composition. CPU009 passed 28 focused controls and strict lints in ordinary and
complete `cooked-native-shaders` modes. GPU001 passed six fresh-process offscreen
controls on Apple M5 Pro / Metal, macOS 27.0.1 build 26A434. Its 30 ordinary/cooked
RGBA pairs are byte-identical. Documentation added afterward does not change the
tested Rust, shader or build inputs.

## Recording and admission

The closed key set has 24 combinations: 1x/4x samples, Opaque/Mask/Blend,
single/double-sided, and normal/mirrored winding. The CPU recorder uses the same
placement, rigid animation and skinned orientation selection as rendering. It
observes the loaded tick and accepted gameplay ticks; committed material/binding
changes update the observed union without inventing an unseen mirror superset.
Mixed skinned orientation and unsupported content fail explicitly. CPU controls
cover changing world, rigid and skinned mirror signs and accepted/refused material
publication; the native fixture does not cover every animation pose.

`MaterialManifest` is bounded to 64 KiB, rejects unknown fields and invalid or
duplicate keys, and carries at most 256 sorted complete cooked-model SHA-256
receipts. Identity binds the scene ULID, exact cooked-scene bytes, compiled
logical catalog, sample/program profile and initial complete model content.
Receipts are sealed when cooked bytes are decoded; recipe fingerprints and
process-local asset version IDs are insufficient. Logical catalog identity binds
the relevant renderer/cooker/content source, Cargo manifests and lockfile. Native archive
identity remains a separate device/build/OS contract.

Recording provenance contains the actual script artifact and input hashes,
tick range/rate and all encountered model receipts. Different later gameplay
inputs are allowed to discover a new key; provenance records the earlier run,
not a promise that its inputs predict all future gameplay. Exact loaded identity
is required before an opaque admitted manifest can reach the renderer. No
project-provided shader, descriptor, native cache byte or executable code is
accepted by this route. Native model content hashing and manifest file I/O occur
at decode/admission/publication boundaries, without extra per-tick hashing or I/O.

Recording keeps bounded logical selection scratch (128 MiB, excluding already
loaded asset ownership and allocator overhead), at most 100,000 primitives and
36,001 tick observations. Backwards ticks or a failed observation poison the
recording. Only complete successful recordings publish a create-only atomic
output. Existing files are never overwritten.

## Renderer ownership and fallback

At level load the route prepares recorded real keys and fixed fallback pipelines:
12 fallback PSOs for a 1x profile, or 24 for a 4x profile that also admits 1x
pixel-perfect selection. Other fixed families retain their ordinary preparation.
This startup cost is an explicit limitation; no startup improvement was measured.
AO, bloom, motion blur, temporal reconstruction, MetalFX, wasm and ambiguous
mesh/sprite/tilemap combinations are outside this opt-in route and refuse instead
of silently changing the rendering policy.

Fallback uses the original vertex, UV and base-alpha paths, with finite clamped
base-color RGB scaled by 0.65. Opaque output alpha remains one; Mask retains its
discard threshold; Blend retains straight alpha. Winding, culling, attachment
formats, multisample coverage, depth comparison and depth-write policy match the
real material key. It deliberately omits full material lighting while pending.

The frame path performs bounded atomic request/deduplication and result lookup;
it never creates a PSO, waits for the worker or writes a file. One worker owns its
compiler/device/shader references, native diagnostic scope and validation/OOM/
internal error scopes. Each of the 24 result cells publishes once. A failed key
retains fallback with a bounded diagnostic and is not repeatedly recompiled.
Cancellation skips queued work but cannot interrupt a driver call already in
flight. Explicit cold close waits at most two seconds. A timed-out owner remains
quarantined behind the process-wide permit; replacement returns Busy. Compiler
captures and shared pipeline state are destroyed before permit release. A
blocking-destructor CPU control proves that ordering. Permit release means owned
compilation/teardown finished; only a successful join proves thread exit.

Drop requests cancellation and detaches; it does not join or perform persistence.
Archive publication remains an explicit cold completion action after successful
validation and worker close. Creating a worker thread is not evidence that native
driver work cannot stall another thread or the GPU service.

## CLI route

The source-free `incant_game_player` accepts `--record-materials PATH` with the
existing `--cpu-render-input MODELS TEXTURES CAMERA_ULID` route. Optional
`--material-program wgsl|cooked` and `--material-samples 1|4` select the recorded
profile. Recording needs no GPU. A separate run uses `--prewarm-materials PATH`
with the same CPU spatial-input arguments to request the offscreen GPU consumer;
the admitted manifest supplies its sample/program profile. Actual input changes
may cause the worker/fallback route to run. Record/load are mutually exclusive;
window/checkpoint combinations and unavailable feature modes refuse.

`incant_headless native-play` also exposes `--record-materials`, program and sample
options on the native gameplay path. Its current source was typechecked in
CPU009; that headless CLI itself was not executed. The separate source-free
player CLI was executed for CPU recording/admission and the GPU record→load route.
The tested player's normal dependency graph excludes document, CRDT, command-bus
and agent crates. Its fixture deletes authored source before invoking the player.

## Observed qualification

| Control | Actual result |
| --- | --- |
| Ordinary CPU matrix | 20 passed; three GPU tests ignored for the later lease |
| Cooked CPU matrix | Eight passed; the same three GPU tests ignored |
| Strict ordinary/cooked lint | Both passed on the selected changed packages/targets |
| Pending raster, each route | All 12 alpha/double/mirror keys at 1x/4x; unchanged frame-path PSO counts; finite fallback |
| Alpha/culling/occlusion | Zero-alpha Opaque/Mask/Blend, single-sided backface/double-sided and foreground-over-rear controls passed |
| Partial Blend | 0.25/0.75 checked against native endpoints within 0.012 linear RGB |
| Actual worker publication | Pending and published frames differ; subsequent native render-pipeline creation rows are successful Background calls with key segments |
| Source-free player, each route | Fresh CPU record, changed mirror input, one key prewarmed, new key requested, fallback used, three non-background frames and successful cold close |
| Ordinary/cooked parity | 30 pairs, maximum RGBA byte difference zero, exact alpha and foreground masks |

The six controls submit 162 finite offscreen draws. They do not open a window or
capture the desktop. Eager-versus-fallback tests check coverage/occlusion; fallback
RGB is intentionally different. No direct depth-buffer readback is claimed.
The paused-worker controls pause before compilation, never inside a native driver
call. Native diagnostic `device` values are process-local observer IDs, not Metal
hardware registry IDs. Native call duration fields are diagnostic receipts only.

Astra viewed 12 original lossless 96×80 PNGs, including mirrored/double-sided,
1x/4x, mask, fractional blend and pending/published examples. The restrained red
fallback retains silhouette and the rear blue layer; published material brightens
without a placement discontinuity. Mask openings and partial blends remain
visible. Root independently viewed five original PNGs and accepted the same
specific fixtures. That scoped appearance is accepted; arbitrary scenes, game art,
all animation poses and native-window workflows remain unqualified.

| Pending fallback | Published material | Masked fallback | 0.25 blend fallback |
| --- | --- | --- | --- |
| ![Pending](evidence/recorded-material-prewarm/pending.png) | ![Published](evidence/recorded-material-prewarm/published.png) | ![Mask](evidence/recorded-material-prewarm/masked.png) | ![Blend](evidence/recorded-material-prewarm/blended.png) |

## Retained evidence and limits

The [portable manifest](evidence/recorded-material-prewarm/manifest.json) binds 557
exact text records, 120 raw-RGBA/PNG records and 13 external executable identities.
It contains the CPU/GPU input inventories, resolved metadata, source/tool/feature
identities, all terminal outcomes, native rows, image bytes and rendered review.
Native executables and archives remain external. Verify without compiling,
extracting, importing or executing any retained code:

```sh
python3 docs/spikes/evidence/recorded-material-prewarm/verify.py
python3 -m unittest discover -s docs/spikes/evidence/recorded-material-prewarm -p test_verify.py
```

The optional verifier `--external-root` takes the retained root from manifest.json
and hashes the 13 executables without executing them. Twelve verifier controls
reject coherent attempts to overclaim native worker ownership, publication,
source-free fallback, cleanup, historical success, budget resets or performance.
These synthetic mutations validate evidence checking, not native behavior.

All seven failed CPU packets remain bound: explicit Game type inference, an
unsupported Camera fixture constructor, the original target storage cap, a reused
single-use publication ticket, a source-prefix comparator including fragment
comments, a Display-versus-Debug CLI assertion, and strict Clippy's bool-filter
iterator diagnostic. The ticket, source-prefix and CLI changes corrected test
fixtures; the final iterator change preserved key-selection semantics. Runtime
admission was not weakened. The storage cap
stopped its build and was subsequently raised explicitly from 6/8 GiB to 10/13 GiB
target/account without deletion, reset or removal of earlier charges.

CPU009's actual outer wrapper exited zero; all 478 observed owned PIDs were absent.
GPU001's wrapper exited zero; its six test leaders and all nine observed owned
PIDs including the supervisor were absent. Four nested player invocations were
synchronously waited by the tests; two were too short for the polling tracker to
observe independently. No claim is made that all nested PIDs were individually
sampled. The trusted-tool PID/start-time supervisor does not guarantee containment
of hostile instantaneous escape or external driver-service quiescence.

The original cumulative GPU allowance retains 150.227878875 seconds after the
20.284641083-second phase; these are resource charges, not performance timings.
Observed owned GPU peak RSS was 66,797,568 bytes; external driver/compiler services
are excluded. Final measured target/account usage was 8,687,766,150 /
12,850,955,008 bytes, with a 20 GiB free-space admission floor. There is no driver
stall, hitch elimination, shipping-profile, reference-phone, all-effects or phase
gate claim. Ordinary eager preparation remains available until any broader route
is separately qualified.

Later main361 is outside this native producer. Actual Cargo compiler-artifact
records show the ordinary player uses `incant_audio_runtime` without `device` and
Kira without CPAL. The complete cooked player enables `native-window` transitively
and compiles device audio/CPAL, although this exercised offscreen route uses
`OfflineBackend`. Its executable identity cannot be reused after main361's device
audio source change. Workspace-wide metadata alone is insufficient for that
per-build feature distinction. Integration and any additional qualification must
retain these original records and cumulative charges.

## Integrated CPU and Windows composition

The later joined source `00dc34c5c7913d1fcae5e2191e6db62e10172115` includes the
DX12 library route and current macOS device-audio correction. CPU010 rebuilt six
ordinary/cooked renderer, player and source-free control executables on that
exact source. Its 14 focused worker/player CPU checks pass, six GPU controls stay
ignored, and both renderer/player strict all-target Clippy stages pass. Fresh
compiler-artifact records confirm that ordinary player builds exclude device
audio/CPAL and complete cooked player builds include them. Thus the cooked
composition has current build evidence; its older executable is not reused as a
new-source artifact.

Two additional Windows GNU cross-compilation stages on macOS pass strict Clippy
for the renderer test targets, first with `dx12-pipeline-library`, then with that
feature plus complete `cooked-native-shaders`. They compile the joined recorded
constructor, strict DX12 diagnostic fixture and native fallback bundle. No
Windows executable or test was run. These checks do not establish MSVC,
Windows filesystem/library behavior, driver reuse or pixels. The borrowed checkout
was restored clean to its original capture branch after both managed groups and
all five tool-probe groups were confirmed absent.

The merge keeps the recorded admission argument before backend-specific options.
The Windows recorded constructor chooses ordinary Capture; strict DX12 diagnostic
constructors remain eager and pass no recorded manifest. Metal and DX12 cold
cache-session branches are mutually exclusive by target OS. Existing worker
ownership and explicit cold persistence remain intact. Comparison with the
original producer retains the exact material selection, worker, player,
recorder, shader-cooker and Metal HAL implementation bytes, apart from the
Windows-only constructor argument in the recorded module. Other shared renderer
changes add the reviewed DX12 feature/target branches; no material shader or
fallback arithmetic changed. The prior exact-source CPU tests continue to
support those unchanged implementations without being described as rerun.

Composition `c25c6f507f21ff5b2b576f252c147cef941d5cef` subsequently adds only the
Python DX12 storage observer repair, its tests and documentation. A complete
source comparison records all five changed paths; every Rust, shader, lockfile
and build input is identical to `00dc`. The six GPU controls and rendered review
above remain explicitly bound to `11d4`, with their original binaries and image
bytes. No new GPU run, new appearance claim or performance gate is inferred from
these CPU and source-composition results.

| Retained integration result | Scope |
| --- | --- |
| CPU010 native macOS | 14 CPU passes, six GPU controls ignored, two strict lints, six fresh artifacts |
| CPU010 cleanup | Actual outer exit zero; all 198 observed owned PIDs absent |
| Windows GNU crosscheck | Two strict test-target compile/lint stages; zero native Windows tests and zero copied executables |
| Windows cleanup | Actual outer exit zero; seven managed PID/groups absent; original checkout restored clean |
| Integration source comparison | All 22,067 qualified files compared; later composition has one added documentation record and five changed paths total |

All original costs and failures remain charged. CPU010 consumed
65.359749209 build seconds, 3.640569208 CPU seconds and 35.160788417 lint seconds,
leaving 1224.969096750 / 148.346088086 / 478.960883166 seconds in the respective
original accounts. Its sampled owned peak RSS was 1,521,500,160 bytes; external
services are excluded. The original target/account ended at 8,928,852,109 /
13,614,870,594 bytes under the unchanged 10/13 GiB ceilings. The GPU allowance
remains 150.227878875 seconds. These are supervision/resource charges, not speed
measurements.

The separate original CORE account retains its 12,020,734,905-byte baseline and
16,315,702,201-byte ceiling. The crosscheck ended at 15,772,689,721 bytes, with
543,012,480 bytes remaining; no target cleanup or reset occurred. Its conservative
26-second charge brings the original sequence to 395 of 3600 seconds, preserving
all earlier failures. Existing copied artifacts remain 380,898,056 bytes, with
zero new copies.

The additive [integration manifest](evidence/recorded-material-prewarm-integration/manifest.json)
binds 169 exact text records, six external executable identities and six external
Git-patch identities. Large patch bodies and native artifacts remain external;
source inventories, actual compiler feature records, terminal results, cleanup,
resource accounts and source-reuse comparisons are portable. The earlier
rendered packet is unchanged and hash-bound by this verifier. No archived script
is imported, extracted or executed:

```sh
python3 docs/spikes/evidence/recorded-material-prewarm-integration/verify.py
python3 -m unittest discover -s docs/spikes/evidence/recorded-material-prewarm-integration -p test_verify.py
```

Ten data-only verifier controls check the valid packet and reject coherent
changes to GPU provenance, Windows execution claims, cleanup, resource charges,
strict lint, native source reuse, audio feature records and checkout restoration.
Optional `--external-root` hashes the twelve retained executable/patch files
beneath the evidence root without executing them. These controls validate the
evidence checker; they are not additional engine, GPU or performance tests.
