# Compatible live animation content

Compatible animation content refresh is implemented. Qualification covers 66 CPU controls and one numerical Metal control, preserving the live world, script VM and playback clock. This extends the existing native Play asset transaction without changing runtime document dependencies. Actual animated editor-window/GUI workflow remains unrun.

For an existing animated model, this slice admits bounded clip key-time/value content changes with unchanged clip count/order/names, channel node/property/interpolation bindings and exact clip-duration bits. Rest TRS, skeleton/joint order, inverse-bind matrices, node-to-skin bindings, influence streams and animated vertex/index buffers remain unchanged. Existing node/scene/primitive/material/texture-role checks still apply. Compatible PBR factors and image content may change with the clips in one complete catalog. Static geometry refresh retains its previous behavior. Animation presence changes and incompatible bindings explicitly refuse.

The native world retains its original immutable asset reference and playback/graph definitions. Startup still loads the exact referenced model bytes. Later sealed catalog tickets establish compatibility before a new visual version is staged. Binding checks use that original world fingerprint and validate the actual new model's complete duration table, IK layout and root catalog; this also permits subsequent structural rebinding without relaxing fresh startup. No public fingerprint alias or mutable catalog escape is added.

Visual versions remain host-local, as with texture/static-model refresh. Native checkpoints retain the world's original asset recipe; this slice does not persist a refreshed visual catalog into native checkpoint bytes. Restart recooks the current authored assets through the existing startup path.

Clip/fade/graph clocks, parameters, root-motion observations and script state remain in their existing owners. Final publication copies the newest complete animation/pose/IK/root selection into preallocated CPU storage, after the existing revision/session/world/catalog checks. It neither seeks nor recreates playback. Cooked animation payload contributes to the existing published-set budget. Preparation and GPU resource creation stay outside authoring locks; final publication uses the same borrowed handle swaps, panic boundary and deferred retirement.

Models used as baked navigation sources remain refused. The canonical root-only catalog must still equal the native simulation's retained catalog, so changed root-motion tracks refuse while compatible non-root keys may publish. Animated geometry, rest and skin data are fixed, preserving model-derived simulation bindings; live rendered bounds are recomputed by the staged canonical consumer. The numerical Metal control verifies a distinct motion-history owner and no accepted previous deformation on the first replacement draw. It does not establish temporal reconstruction image quality. Target ownership/presentation counters retain their existing behavior.

The executed CPU controls use actual glTF/PNG cooking and the canonical skin sampler. They cover changed key counts and deformation at a continuing clock, atomic mixed image/texture content, unchanged outgoing fade/graph state, later structural rebinding, content-checked fresh startup, equal-count binding refusals, payload/corrupt-cache refusals, root-catalog refusal/retention and stale/consumer/panic rollback. A Play control uses real importer and command-bus Undo/Redo transactions, plus a persistent module counter to distinguish VM continuation from replacement. Existing Play publication and texture/static-model controls passed because the final transaction protocol is unchanged.

The 66 CPU controls comprise 54 native rendering and 9 Play tests on source002, two filtered render-game hosted integration controls on source003, and one keys-only fixture control on the final source005. The render-game library command compiled but ran zero tests; it is not publication-test evidence. Strict Clippy passed for the three production crates on source002 and the changed GPU-host game target on source005. The fixture check also passed on source004; this repetition is retained separately rather than added to the coverage count.

On the Apple M5 Pro Metal adapter, the final source005 control rendered a fixed 160×96 target. With identical geometry, material and image content, changed clip keys moved the 276-pixel red mask exactly 12 pixels at the same 0.5-second clock, preserving its area. Incompatible-duration and late-consumer refusal buffers matched the complete original bytes. Publication presented no frame under the test lock; the next draw used the new catalog and fresh motion owner. After deleting only the owned temporary sources/cache, the next tick advanced the existing animation to 0.5166666666666667 seconds and the persistent guest counter from 1 to 3. World identity remained unchanged; final tick was 2, with seven accepted frames/presentations and one target-owner drop. Five raw RGBA buffers and their extrema/counts are retained. This is separate from the CPU import/Undo/Redo proof.

The failed attempts remain failures:

| Frozen source | Retained outcome |
| --- | --- |
| source001 `043863a5` | CPU compilation refused two test-only slice-index assumptions about a node iterator; no tests ran. |
| source002 `e2bdc615` | Corrected selectors; native54 + Play9 and production scoped lint passed. Library command compiled with zero tests. |
| source003 `b56e9285` | Both hosted controls passed. New CPU fixture failed its authored `y=0` expectation; the cooker orders that vertex at `y=1`. |
| source004 `f022f23d` | Corrected fixture and scoped lint passed. GPU001 refused cold host construction with `MotionError::Disabled`; it produced no frames/pixels. |
| source005 `cb494bb2` | Fixture prewarms the existing motion pipelines; fixture/lint and GPU002 passed with unchanged classifier and deformation/history assertions. Only existing pinned `pollster` was added as a test dependency. |

Production Rust remains byte-identical to source001. Test-only successors preserve all resolved package versions. CPU builds used Rust 1.99.0, the explicit arm64 CI profile, locked/offline dependencies and two build/Rayon threads. The GPU control used an immediately copied qualified executable, a 120-second child bound and a 180-second cooperative execution deadline plus mandatory process-group cleanup. All executed children were reaped. This is correctness evidence, not shipping performance.

The [portable packet](evidence/animated-model-refresh/manifest.json) contains exact source/command receipts, raw outcomes and buffers, and external executable hashes. Its verifier never invokes an engine, GPU or build:

```sh
python3 docs/spikes/evidence/animated-model-refresh/verify.py
python3 docs/spikes/evidence/animated-model-refresh/verify.py --originals --binaries --git-source cb494bb2b6bb065e7f37569efe11f11adb788f97 --source-root /absolute/path/to/incant
```

No animated editor-window workflow, GUI import/history clicks, appearance acceptance, performance improvement, broad animation-layout refresh or phase gate is claimed. The separate actual-editor workflow plan is pending capability intake. Future unsupported/degenerated numeric poses still follow the canonical consumer's existing typed refusal behavior.
