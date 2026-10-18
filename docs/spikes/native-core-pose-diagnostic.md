# Core pose integration diagnostic

This is a source-prepared, default-off successor to the [qualified static pose
adapter](native-pose-interpolation.md). Its new CPU tests, actual Core route,
native window and rendered output have **not yet been executed or accepted**.
The earlier static CPU/Metal receipts do not qualify this added UI, animation or
window integration.

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

## Prepared qualification

Focused CPU controls cover hierarchy/animation endpoints, loop wrap, source-free
asset use, canonical restoration on success/refusal/unwind, UI generation
admission, real glyph/input/audio behavior, checkpoint parity, read-only clock
residuals, boundary resets, explicit selection and catch-up/terminal chronology.
Existing ordinary controls and strict lints will run in feature-on and feature-off
configurations. The GPU test source is adapted to the UI-bearing consumer
signature but no GPU test is authorized by the CPU packet.

`diagnostic-core-pose` is a separate default-off manual test feature. Broad
ordinary ignored-test CI cannot select its external-input fixture. It requires a
fresh trusted `INCANT_NATIVE_PROJECT` build of Core, exact source/compiler/asset
bindings and a bounded `INCANT_CORE_POSE_FIXTURE` file. It adds no runtime script
loader. The normal package loader supplies cooked data; no native window or
physical audio backend is created.

The manual test proposes the original 2,156-step physical-input win route on
ordinary and pose CPU composed hosts. It compares every stereo PCM sample,
input and haptic request, all observation checks, final world/animation/audio
state, logs, canonical render report, UI checkpoint and accepted draw input.
After each tick it samples fractions 0, 0.5 and 1 from actual retained rigid actor
models and requires shaped HUD glyphs. Each of the 6,468 displays must leave
canonical input, world tick and audio frame count unchanged. Both complete PCM
files must match the frozen route hash. A create-only report and PCM outputs
publish only after complete success; failed or partial attempts remain failures.

This CPU consumer establishes admission, input and pose sampling correctness if
it passes. It cannot establish GPU color, displayed motion, cadence, input-to-
display latency, device audio quality or shipping performance. Actual rendered
Core, focus/resize and restart/Stop integration remain separate gates; current
computer-use/window/capture restrictions prohibit those executions.
