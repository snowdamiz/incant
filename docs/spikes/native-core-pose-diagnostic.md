# Core pose integration diagnostic

This default-off successor to the [static pose adapter](native-pose-interpolation.md)
has passed focused CPU qualification and the actual 2,156-step Core route on
ordinary and interpolated composed CPU hosts. The final producer is
`66dfbcbb4524efe2a93b4e626a2ae9459c26c799`, including main422. Native-window
execution, GPU color, displayed motion and pacing remain **unqualified**. Earlier
static Metal receipts do not qualify this added UI, animation or window path.

`native-pose-interpolation` adds the trusted, bounded window selection
`--pose-interpolation previous-current --stop-after N`. Ordinary exports do not
enable the feature. Even when compiled, the ordinary selection retains the
ordinary host. A project/package cannot select this policy. Minimum-duration,
display-link and batched presentation selections refuse in combination with it.

## Chronology and ownership

Every ordinary fixed tick still consumes the same input, runs the same gameplay,
updates audio and admits the complete canonical render input. No simulation
deadline, rational clock arithmetic, catch-up quota, surface reservation, input,
haptics or audio policy changes. The selected host separately presents a numeric
overlay after successful admission. The final draw in a due batch uses the
existing rational accumulator residual; intermediate catch-up draws and accepted
Stop/Restart transitions flush fraction one. Initial/background refreshes also
use fraction one. This does not create extra display opportunities or claim to
repair the measured display-interval tail.

Previous/current local transforms are composed parent first, including invisible
ancestors, cameras and lights. Every temporary matrix and animation sample is
restored after success, refusal or unwind. Resource identities remain committed.
The history keeps its existing 65,536-node / 32 MiB allocated-buffer limits;
animation endpoints count within them. A cold replacement can coexist with one
old history, bounded at 64 MiB. Asset, UI and retained renderer budgets remain
separate.

Simple clip animation uses the same two accepted samples and existing local-pose
blender as normal rendering. It never interpolates wrapped clip seconds or
advances a separate animation clock. Clip changes snap. Rigid node animation is
the actual Core actor path; a skinned fixture also checks the shared sample
contract. Graphs/crossfades, IK/root motion, sprites, tilemaps, world-space text
and pixel-perfect cameras remain explicit refusals. Temporal reconstruction,
dynamic resolution and MetalFX keep their prior GPU policy refusal.

The ordinary UI Binding prepares/commits the HUD at fixed ticks, with its font
catalog, shaped glyphs, input routing and persistent checkpoint owner. A display
borrows the exact accepted UI input; UI itself is not interpolated. Direct
display refuses a UI generation not admitted with the canonical spatial owner.
Both host-wrapper selections delegate checkpoint preparation to their actual
host, including ordinary mode in a feature-enabled executable.

Focus, suspension/resume, surface retry, resize/layout, discarded wall debt and
session replacement arm a reset. The reset snaps the current **already admitted**
pose before its next display; it does not arm a future gameplay tick. Repeated
boundary notices coalesce. Arbitrary gameplay teleports and same-clip seeks do
not yet carry a general discontinuity event into the window driver. Such routes
are unsupported by this diagnostic unless the trusted caller explicitly resets
after admission and before display. Ordinary continuous interpolation adds up to
one fixed tick of visual latency; endpoint flushes intentionally expose current
state.

## CPU qualification (2026-10-18)

Focused CPU controls cover hierarchy/animation endpoints, loop wrap, source-free
asset use, canonical restoration on success/refusal/unwind, UI generation
admission, real glyph/input/audio behavior, checkpoint parity, read-only clock
residuals, boundary resets, explicit selection and catch-up/terminal chronology.
Existing ordinary controls and strict lints passed in feature-on and feature-off
configurations, with the source generations below kept distinct. The GPU test source is adapted to the UI-bearing consumer
signature but no GPU test is authorized by the CPU packet.

`diagnostic-core-pose` is a separate default-off manual test feature. Broad
ordinary ignored-test CI cannot select its external-input fixture. It requires a
fresh trusted `INCANT_NATIVE_PROJECT` build of Core, exact source/compiler/asset
bindings and a bounded `INCANT_CORE_POSE_FIXTURE` file. It adds no runtime script
loader. The normal package loader supplies cooked data; no native window or
physical audio backend is created.

The manual test executed the original 2,156-step physical-input win route on
ordinary and pose CPU composed hosts. It compares every stereo PCM sample,
input and haptic request, all observation checks, final world/animation/audio
state, logs, canonical render report, UI checkpoint and accepted draw input.
After each tick it samples fractions 0, 0.5 and 1 from actual retained rigid actor
models and requires shaped HUD glyphs. Each of the 6,468 displays left
canonical input, world tick and audio frame count unchanged. Both complete PCM
files matched the frozen route hash. A create-only report and PCM outputs
publish only after complete success; failed or partial attempts remain failures.

This CPU consumer establishes the tested admission, input and pose sampling
correctness. It cannot establish GPU color, displayed motion, cadence, input-to-
display latency, device audio quality or shipping performance. Actual rendered
Core, focus/resize and restart/Stop integration remain separate gates; current
computer-use/window/capture restrictions prohibit those executions.

### Results and retained failures

- Source `8a875aee`: 461 behavior controls passed (20 adapter/animation, 42
  composed host/UI, 202 feature-on player and 197 ordinary player). The supervisor
  nevertheless failed when strict lint found two redundant `f64` conversions in
  a test. No unaffected behavior suite was repeated.
- Source `e1f6f181`: the two identity conversions were removed; both strict
  all-target on/off lints passed. The first fresh Core cook and exact native test
  also passed, but its aggregate analyzer failed: the fixture serialized spatial
  input without the actual HUD summary that the historical report contains.
  Current ordinary/pose HUD equality was tested internally; historical HUD
  equality was not recoverable from that incomplete JSON. This failure remains
  failed in the packet.
- Source `66dfbcbb`: the ordinary final-report serializer was extracted without
  changing its contents and shared with the fixture; main422 was merged without
  conflicting feature paths. One incremental authored build, one exact copied
  Core test and scoped strict player-library lint passed. The analyzer stayed
  byte-identical and checked the full HUD report; no comparison was removed.

The final route produced 6,468 accepted HUD/pose display calls and 49,460 rigid
pose samples. All eight historical observation checks and all 17 observed entity
records match exactly, as do gameplay logs and the canonical render report.
Final HUD data contains 13 logical widgets, seven text sources and 95 glyphs,
with atlas SHA-256
`923038f23c7f3fbc406d472dd98273c3d848cd7024e1340c175faf17f05409e5`.
Every stereo sample was compared during the route. Both complete PCM files are
13,798,400 bytes with SHA-256
`ccabf2c68b6388ac554ac3efb5609ddf6b4a45d1a3f316891d5f344094e99ee3`.
The test additionally compares whole world snapshots, animation and audio state,
input/haptic outputs, UI checkpoint/draw data, and restores canonical input after
every display. These are CPU input/sampling observations, not rendered pixels.

The manual producer uses Rust1.99, the `ci` test profile, implicit arm64 Mac host,
the original fast4 physics selection, Node22.23.3 and the pinned TypeScript/SWC
compiler closure. It freshly cooks the same 60 runtime inputs from the historical
74-file workload; 14 authoring history files are deliberately absent, preserving
the normal empty recovery epoch. The graph excludes GPU, native-window and
device-audio dependencies. The compiler-bearing CPU test is **not a shipping
build or performance measurement**. Supervisor wall durations are resource
charges only. All costs, including the first aggregate failure, remain charged.

The [portable packet](evidence/native-core-pose/manifest.json) retains source
manifests/patches, exact plans and commands, raw test/lint streams, both aggregate
outcomes and corrected reports, compiler/authoring identities, and cleanup
receipts. Heavy executables, cooked assets and PCM remain external hash-bound
originals. No archived code or executable is run by the verifier:

```sh
python3 docs/spikes/evidence/native-core-pose/verify.py
python3 docs/spikes/evidence/native-core-pose/verify.py --originals --binaries --git-source .
```

Default replay verifies retained evidence, not a new native execution. Optional
original/binary checks require the producing machine's external files;
`--git-source` checks the changed feature bytes in each recorded commit.
`--source-root` checks all compiler inputs for the final producing source and
intentionally refuses a later checkout whose documentation or code differs.
No renderer/window/pacing policy is adopted by these CPU results.

Final terminal-only development checks passed after lockfile-pinned dependency
installation with lifecycle scripts disabled: conventions, tools/Core tests,
rustfmt, bridge/SDK freshness, TypeScript, UI build, 1,040 UI tests, Wasm SDK
freshness and diff checks. The first offline dependency attempt refused an
uncached npm tarball before checks began; its failed receipt is preserved.
These are build/test checks, with no browser, window, capture helper or device
execution. The evidence verifier was replayed again after adding these records.
