# Native pose interpolation

The Godot source comparison separates fixed simulation, display scheduling and
interpolation. Incant now has a default-off `experimental-pose-interpolation`
adapter and composed host for the last of these. Ordinary exports remain
unchanged. There is no player/window selection yet and no performance adoption.

## Contract

Every accepted contiguous simulation tick advances a bounded pair of local
transforms, even when no frame is drawn. A display request interpolates local
translation/scale and normalized shortest-path quaternion rotation, then composes
the hierarchy parent first. Invisible spatial ancestors participate. Temporary
render matrices are restored exactly after success, refusal or unwind; the
simulation world, input, audio, script clock and haptics do not advance.

The adapter rejects foreign worlds, backwards clocks and same-tick numeric
mutation without a new continuity epoch. Gaps, topology/binding changes and new
epochs snap rather than interpolate unrelated states. Scale-sign changes snap
the affected subtree. Teleports require an explicit trusted epoch reset after
the discontinuity is admitted and before it is displayed. Reset snaps the
current endpoints immediately; it does not arm the next tick. Resume, resize and
dropped-debt integration must apply the same continuity rule.

History has at most 65,536 entries and 32 MiB of actual allocation; replacement
temporarily permits 64 MiB for old and new history. A rejected canonical admission
never publishes replacement history. The composed presentation operation owns
the entire live session and restores it only after success. Failure or unwind
terminates that composition, including target acquisition/submission failures.

The current scope is static meshes, cameras and lights. Animation, IK, root
motion, sprites, tilemaps, world text and pixel-perfect modes refuse explicitly.
Game UI/text also refuses, including removed UI whose checkpoint owner remains.
Motion-history reconstruction, dynamic resolution and MetalFX refuse the GPU
route until qualified separately. Interpolation deliberately adds one fixed step
of visual latency; the caller supplies a finite fraction in `[0, 1]`.

## Qualification

At `ba5d497d`, 111 native adapter/composed-host tests pass, with strict lints for
the feature-enabled packages and ordinary runtime. Controls cover hierarchy and
rotation math, bounds/allocations, lifecycle rejection, teleport reset ordering,
and complete gameplay/input/audio/haptic parity against ordinary execution.
Save bytes match after both paths drain the same pending haptic requests;
checkpointing pending haptics correctly refuses on both paths.

At `331219f8`, the GPU test executable and strict all-target lint pass. Three
actual offscreen Metal controls then pass on this Mac:

- After two accepted simulation ticks with no draws, fractions 0, 0.5 and 1 each
  match the complete RGBA image from an independent fresh renderer using the
  analytically expected static pose. The three images are distinct. Canonical
  transforms, world, input and audio frame counts remain unchanged.
- Acquisition refusal, submission refusal and presentation unwind all terminate
  the composed owner without reporting a successful presentation.
- Motion-history and dynamic-resolution policy refusals occur before acquiring a
  target and terminalize the direct GPU consumer.

The first GPU suite passed its image/failure controls but failed the policy
fixture because its motion pipelines had not been initialized. A subsequent
build request selected a nonexistent experimental Cargo feature and refused
before compilation. The corrected fixture enables the real runtime motion
pipelines. Both failed attempts remain failed. Earlier compile and fixture
corrections are also retained with their original sources and results.

[Receipts and review records](evidence/native-pose-interpolation-20261018/manifest.json)
record exact source identities, commands, test output and process closure. All
owned processes/groups are absent after terminal results. This establishes scoped
CPU/GPU correctness, not native-window motion quality, shipping performance,
full animated/UI support or a phase gate. Native scheduling integration and
matched shipping-profile display/latency measurements remain next.

The [Core integration successor](native-core-pose-diagnostic.md) prepares a
default-off ordinary-window selection, fixed-step HUD admission and committed
simple-clip sampling. Its new source/tests are not covered by these historical
static receipts and remain unqualified until its own bounded controls execute.
