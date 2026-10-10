# 0028 off-mesh link rendered-motion review: result

## Status

**Complete for this packet's scope.** A real-engine scene sends one walker
across three separate platforms using actual `api.findPath` off-mesh links. It
uses a directed jump-up, a bidirectional bridge taken forward and then reversed,
and a return drop that ordinary `set_component NavigationMesh` commands enable
and later disable. Traversal metadata alone drives every jump. Saving mid-drop
and reopening reproduces the uninterrupted frames byte for byte. Two
independently authored test projects and runs on the same immutable binary
produced byte-identical frames and logs. The engine was not recompiled.

The review found **one visible engine defect** and several look notes:

1. **Hovering walker (engine, quantized heights).** Every returned route height
   sits one `cell_height` above the true platform top. The walker visibly hovers
   with a detached shadow in the near-level feet view. See Findings.
2. **Snapped centre endpoints moved 5 cm inward.** This is not a defect in
   motion, because the behavior jumps between the snapped points. It is recorded
   because authored and snapped endpoints differ.
3. **Behavior look notes.** There is no launch anticipation or landing
   absorption. The body finishes turning during the first airborne ticks. The
   up-jump and the return drop trace the same profile arc. These belong to my
   behavior, not to the engine.

This result does not approve a phase gate or claim the wider phase complete.
The scene has **no RigidBody or Collider components**, so it uses no Rapier
bodies and makes no physics claim. Jumps are a logical parabola applied with
Transform commands. The engine does not execute them or promise that they are
safe. No cross-device determinism or platform budget is certified.

### Model and transport

Exact model: Claude Opus 5.5, model ID `claude-opus-5-5`. No model substitution
occurred.

Transport is ACP on the host's evidence. `tools/acp/config.json` pins model
`claude-opus-5-5` and the `claude-agent-acp` adapter. From inside the session I
cannot inspect the ACP messages themselves.

### Packet acknowledgement

**Priority revision 2 (report accuracy only).** The director accepted the
rendered review and measured results for this scoped link increment, and kept
the visible height limitation explicitly open. As instructed, I made two wording
corrections in this file and nothing else. No captures were rerun, and the
engine, scene, behavior and evidence are unchanged.

1. **Sleep prevention.** All recorded runs executed under `caffeinate -s -i`,
   both the outer ACP runner and each `run_set.sh` command. The earlier claim
   that no temporary sleep assertions were needed is withdrawn. Script budget
   and Run results below now state the actual conditions.
2. **Repeat wording.** "Two independent builds" is replaced by two
   independently authored test projects and runs on the same immutable binary.
   No engine recompilation occurred.

The hover defect, exact measurements, session-local generation difference and
all limits are retained unchanged.

The original packet had no priority-revision section and carried no new director
feedback. No files existed from an interrupted attempt: the worktree was clean
at commit `1a9769d`. I read `CLAUDE.md`, the packet, the navigation-links spike
document, the SDK types, `tools/probes/navigation-links.py` and the 0026 and 0027
helpers. I read the relevant engine sources read-only before changing anything.

No Rust, native or web UI, other handoff, mutation routing, credentials or
external accounts were touched. Nothing was compiled from Rust. Nothing was
published, merged or signed. `npm ci` installed the pinned Node dependencies
into this worktree's ignored `node_modules`.

## Binary and source

| Item | Value |
|---|---|
| Binary | `artifacts/tools/incant_headless` |
| SHA-256, before and after every engine call | `dbbb6627a6e7878a302fb9f035a3a4e456707f476a950a6395e1e81d9d25b6ba` |
| `binary.json` source commit | `ccd6d33e0bc144debffd70890792aa70a6cc47c6` |
| Worktree base | `1a9769d` |
| GPU adapter reported by `play` | Apple M5 Pro |

The helper verifies the hash before and after each of its 26 `play` calls per
set. All 52 recorded pairs match `binary.json`.

## Scene

All geometry is original mathematical glTF written by the helper. The project is
built only through public `init`, `import`, `rpc` (`command.execute` then
`project.save`) and `validate`. No project JSON is edited directly. Authoring
file hashes are identical before and after all play runs.

- **Platforms.** West 3.5 × 4 m with its top at 0 m, Centre 3.5 × 4 m at 0.5 m,
  East 3.5 × 4 m at 0 m. They are separated by 1.75 m trenches over a near-black
  floor at −1.5 m. The tops carry 0.5 m checker tiles for scale. The three
  platform MeshRenderers are the only navigation sources (`geometry: "mesh"`).
- **Walker.** A capsule of radius 0.25 m and height 1.3 m, matching the
  navigation agent. Its feet are at the entity origin. A dark visor marks
  facing.
- **Links.** These are all `extra_cost` 0 with a 0.3 m snap distance.

| Link ID | Role | Authored start → end | Flags |
|---|---|---|---|
| `01JA2NAV000000000000000001` | West → Centre jump-up, north lane | (−3.9, 0, −1) → (−1.35, 0.5, −1) | directed, enabled |
| `01JA2NAV000000000000000002` | Centre ↔ East bridge | (1.35, 0.5, 0) → (3.9, 0, 0) | bidirectional, enabled |
| `01JA2NAV000000000000000003` | Centre → West return drop, south lane | (−1.35, 0.5, 1) → (−3.9, 0, 1) | directed, starts disabled |

- **Visual language.** Each link has render-only ring pads at its authored
  endpoints and a chevron behind each permitted launch. Amber marks the directed
  jump-up. Teal marks the bridge, with chevrons at both ends. The return drop is
  grey while disabled and green while enabled; the behavior swaps those markers
  when it toggles the link.
- **Trails.** Floor dots drop every 10 walking ticks. Small floating octahedra in
  the link's colour drop every 3 airborne ticks, so each arc is drawn by real
  rendered geometry. Nothing is native debug draw.
- **Cameras.** The overview is an elevated three-quarter view of the whole
  course. The profile is a near-orthographic side view from 40 m with a 12° field
  of view, which makes arc shapes and heights read true. The two gap views are
  close three-quarter views of each trench. The goal-feet view is a near-level
  10° telephoto on the goal mark, used for grounding.

## Behavior

`tools/off_mesh_course.ts` is compiled with strict `tsc` and
`tools/build_script.mjs`.

1. **Tick 1.** Query West → East and expect the jump-up then the bridge. Query
   East → West and expect **null**, because the jump-up is directed and the
   return link is disabled.
2. **Outbound.** Walk at 1.4 m/s. Leftover time carries across corners and
   walk/jump boundaries, so no tick loses motion.
3. **Jumps.** Segment *i* → *i*+1 is airborne only when `traversals` holds
   `from_index == i` and `to_index == i + 1`. The behavior throws on any
   non-consecutive traversal. It never inspects point spacing. The arc runs from
   `points[from_index]` to `points[to_index]`, the actual snapped endpoints. Its
   duration is the horizontal distance divided by 2.4 m/s, with a 0.5 s minimum.
   Its apex is 0.35 m plus half the height change above the chord.
4. **Pause and unlock.** The walker holds the goal for 30 ticks. East → West is
   queried again and returns null. The behavior then sends `set_component` with
   the return link enabled.
5. **Return.** The next tick it replans. It gets the bridge with `reversed: true`
   and then the return drop, and walks home.
6. **Lock.** On arrival it disables the return link. The next tick East → West
   is null again, and West → East is still valid at a new generation.

## Measured results

All values are unrounded doubles from the committed logs, recomputed by
`tools/trace_check.py` from positions.

### Queries, generations and tile reuse

| Tick | Query | Result | Generation | Traversals: link, from → to, reversed |
|---|---|---|---|---|
| 1 | West → East | path | 1 | jump-up 1 → 2 false; bridge 3 → 4 false |
| 1 | East → West | **null** | | |
| 486 | East → West, before enabling | **null** | | |
| 487 | East → West, after enabling | path | 2 | bridge 1 → 2 **true**; return 3 → 4 false |
| 943 | East → West, after disabling | **null** | | |
| 943 | West → East, after disabling | path | 3 | jump-up 1 → 2 false; bridge 3 → 4 false |

The rebuild reports below come from `state.navigation` in the stdout of
log-only plays ending at each tick.

| Run ends at tick | Generation | Active links | Rebuilt tiles | Reused tiles | Polygons |
|---|---|---|---|---|---|
| 0 | 1 | 2 | 10 | 0 | 10 |
| 485 | 1 | 2 | 0 | 10 | 10 |
| 486, enable committed | **2** | **3** | **0** | **10** | 10 |
| 941 | 2 | 3 | 0 | 10 | 10 |
| 942, disable committed | **3** | **2** | **0** | **10** | 10 |

Each link-only edit advanced the generation and reused all 10 geometry tiles. A
caution for readers: when a later tick leaves the stamp unchanged, the engine
reports every tile as reused. So "reused 10" alone does not prove reuse on later
ticks. The proof is the 486 and 942 rows, where the stamp changed, the
generation advanced and nothing was rebuilt.

### Metadata versus motion

| Check | Result |
|---|---|
| Ticks whose airborne state disagrees with `traversals` | 0 |
| Max error of airborne positions from the parabola on snapped endpoints | 0.0 |
| Ticks without motion during active legs | 0 |
| Stationary ticks outside the 30-tick pause and after finishing | 0 |
| Shortest walking segment in any route | 2.299999475479126 m |

No submillimetre segments occur. The bridge's Centre point in the outbound route
has z = −1.1102230246251565e-16 rather than 0. The same endpoint is exactly 0 in
the return route.

### Jumps

| Link | Reversed | Launch tick | Land tick | Airborne ticks | Peak feet y (m) | Min logical capsule clearance (m) |
|---|---|---|---|---|---|---|
| jump-up | false | 108 | 173 | 65 | 0.926007921991588 | 0.06404795821653342 at tick 172 |
| bridge | false | 292 | 357 | 65 | 0.9259510912428189 | 0.053751186457955624 at tick 292 |
| bridge | true | 585 | 650 | 65 | 0.926027606103746 | 0.06651734274938914 at tick 649 |
| return | false | 769 | 834 | 65 | 0.9259860099322293 | 0.05123841670983986 at tick 769 |

Clearance is the logical distance from the walker's capsule to each platform's
profile. It stays positive, and its minima occur at launch and landing, where the
5 cm hover dominates. This is geometry only, not a collision check.

The table below gives speed per tick × 60 in m/s.

| Link | Before launch | Launch tick | Landing tick | After landing | Vertical into landing |
|---|---|---|---|---|---|
| jump-up | 1.4000000000000015 | 2.3242277640207254 | 2.037739423733349 | 1.4000000000000001 | −0.8428732014575813 |
| bridge forward | 1.4000000000000001 | 1.5287945273548091 | 3.237248966427872 | 1.4000000000000057 | −2.306670847266388 |
| bridge reversed | 1.4000000000000057 | 2.155401522743506 | 2.181493453732589 | 1.3999999999999995 | −0.9910362734289246 |
| return drop | 1.4000000000000001 | 1.4380022113377309 | 3.4565302268001257 | 1.3999999999999988 | −2.5321786469987053 |

The highest single-tick speed is 3.5483848405776794 m/s at tick 586, the first
rising tick of the reversed bridge. That is a 5.9 cm displacement, so no frame
jumps or snaps.

### Snapped versus authored endpoints

| Endpoint | Snapped | Delta from authored |
|---|---|---|
| West, both links | (−3.9000000953674316, 0.05000000819563866, ±1) | (−9.536743172944284e-08, 0.05000000819563866, 0) |
| Centre, west side | (−1.2999999523162842, 0.5500000715255737, ±1) | (**0.05000004768371591**, 0.05000007152557373, 0) |
| Centre, east side | (1.3000001907348633, 0.5500000715255737, 0) | (**−0.04999980926513681**, 0.05000007152557373, 0) |
| East | (3.9000003337860107, 0.05000000819563866, 0) | (3.3378601083100534e-07, 0.05000000819563866, 0) |

Reversed traversal uses the same snapped points, swapped. The bridge's launch in
the reversed route is exactly its authored end.

### Save, reopen and repeat

| Check | Result |
|---|---|
| Save tick, midway through the return drop | 801, jump fraction 0.49295996328721786, feet (−2.581695927381539, 0.9034011412409306, 1) |
| Reopened run's start tick | 801 |
| Final entity state, uninterrupted versus reopened | identical |
| Log lines, uninterrupted versus prefix plus reopened | 1018 = 814 + 204, all identical except one |
| The one differing line | tick 943 `out-after-lock` generation: 3 uninterrupted, 2 reopened |
| Script state | identical except `generations` [1, 2, 3] versus [1, 2, 2] |
| Reopened capture window frames, ticks 801–849, versus uninterrupted window | 49 of 49 byte-identical |
| Logs over the same window | 48 of 48 debug rows identical |
| Repeat set A versus set B frames | 593 of 593 byte-identical |
| Repeat set A versus set B logs | 26 of 26 files byte-identical |
| Repeat set A versus set B stdout states | identical after replacing the random scene and asset ULIDs from `init`/`import` |
| Failed play attempts from the 50 ms wall-clock budget | 0 in both recorded sets, run under `caffeinate -s -i` sleep prevention |

Sets A and B are two independently authored test projects, each built from
scratch through `init`, `import` and `rpc`, and played on the same immutable
binary. They are not separate engine builds; nothing was recompiled.

The generation difference is the documented session-local behavior. The
reopened session rebuilds its mesh at generation 1 with the return link already
enabled, so disabling it yields 2. Behavior state kept no generation-dependent
logic, so motion is unaffected.

## Rendered-motion review

- **Walking versus gap traversal is unmistakable.** Walking leaves dark floor
  dots. Each gap traversal leaves a coloured floating arc, and the body's shadow
  detaches onto the trench floor. The profile view shows all three arcs at true
  shape.
- **Directionality reads correctly.** The visor leads in every frame reviewed.
  The amber chevron and missing reverse chevron mark the one-way jump-up. Teal
  chevrons at both bridge ends match its bidirectional use, and the reversed
  return arc retraces the forward arc exactly.
- **The toggle is visible and matches the data.** Return pads are grey at tick
  480, green at 488 and grey again at 1000. The swap tick is the same tick whose
  rebuild advanced the generation.
- **Launch.** The walker leaves the snapped launch point on the same tick it
  arrives, with no pause and no positional snap. There is no anticipation crouch.
  Speed changes instantly from 1.4 m/s to the airborne velocity.
- **Turning while airborne.** Yaw is slew-limited to 9° per tick. On the jump-up
  the walker approaches at −66.501432° and reaches −90° at tick 110, two ticks
  after launch. The bridge forward and return drop finish turning at ticks 294
  and 772. Visible only in the per-tick windows, and acceptable.
- **Landing.** Position is continuous, but vertical motion stops in one tick
  with no ease or squash. The drop landings are the hardest: −2.306670847266388
  and −2.5321786469987053 m/s into a dead stop. It reads as a hard, stiff landing
  at 60 Hz.
- **Grounding.** The walker hovers on every platform; see the first finding. In
  the three-quarter views the hover is a slight lift above the pads. In the
  near-level feet view it is obvious: a lit gap under the capsule and a shadow
  that does not touch it.
- **Profile coincidence.** The jump-up and the return drop lie on identical
  profile curves, because a symmetric logical parabola reversed is the same
  curve. The three-quarter views separate them by lane. A gravity-timed arc with
  a higher apex on the rising jump would be more natural. That is behavior work,
  not engine work.
- **Cameras.** All views are fixed and were not changed to hide anything. The
  feet camera was added specifically to expose the hover.

**Verdict.** Route metadata, snapped endpoints, toggles, replans, null queries
and saved traversal behave correctly and deterministically in real frames. The
motion is legible and professional. The remaining visible defect is the
quantized route height, which leaves the walker about 5 cm above the surface.

## Findings for Astra

1. **Route heights sit one `cell_height` above flat source surfaces.** The
   measured offsets are listed below. They apply to walking points and to
   snapped link endpoints, so landings also stop above the surface.

   | Platform | Feet height minus true top (m) |
   |---|---|
   | West, top 0 | 0.05000000819563866 |
   | Centre, top 0.5 | 0.05000007152557373 |
   | East, top 0 | 0.05000000819563866 |

   This matches the known limitation that walking heights are quantized
   approximations; source-conforming heights are separate work. Until then a
   behavior must ground with physics or a raycast, which this scene deliberately
   does not do. The feet screenshot shows it.
2. **The Centre endpoints snapped 5 cm toward the platform centre.** The authored
   x values of ±1.35 m became −1.2999999523162842 and 1.3000001907348633. Both
   authored points are 0.4 m from the Centre platform's edges at ±1.75 m, which
   is more than the 0.25 m agent radius. The West and East edges at ±3.5 m lie
   on 0.1 m cell boundaries, and their endpoints snapped within 3.4e-07 m in x.
   This is consistent with the
   Centre's walkable boundary being eroded to |x| = 1.3, possibly because
   its edges at ±1.75 m fall mid-cell. That is an inference: I cannot confirm
   the boundary without debug draw. Behavior using snapped points is unaffected.
3. **Outbound bridge launch z is −1.1102230246251565e-16, not 0.** It is
   harmless, but it shows the snapped point is computed in floating point rather
   than copied. Comparisons must not assume exact authored values.
4. **The tile-reuse report is ambiguous on later ticks.** Unchanged-stamp ticks
   merge rebuilt into reused. A consumer cannot tell "reused because unchanged"
   from "reused by a link-only rebuild" without the generation. Consider
   documenting this or exposing the last rebuild's own counts.

## Reproduce

From the repository root:

```
npm ci
shasum -a 256 artifacts/tools/incant_headless   # must equal artifacts/tools/binary.json
caffeinate -s -i bash handoffs/0028-off-mesh-lookdev/tools/run_set.sh artifacts/0028-off-mesh/A
caffeinate -s -i bash handoffs/0028-off-mesh-lookdev/tools/run_set.sh artifacts/0028-off-mesh/B
python3 -I handoffs/0028-off-mesh-lookdev/tools/trace_check.py summary artifacts/0028-off-mesh/A --json handoffs/0028-off-mesh-lookdev/evidence/summary-A.json
python3 -I handoffs/0028-off-mesh-lookdev/tools/trace_check.py resume artifacts/0028-off-mesh/A
python3 -I handoffs/0028-off-mesh-lookdev/tools/trace_check.py window artifacts/0028-off-mesh/A ret-full ret-mid
python3 -I handoffs/0028-off-mesh-lookdev/tools/trace_check.py compare artifacts/0028-off-mesh/A artifacts/0028-off-mesh/B
caffeinate -s -i python3 -I tools/probes/navigation-links.py artifacts/0028-off-mesh/probe --binary artifacts/tools/incant_headless
```

Each set runs these plays:

- **Reference.** One log-only run of the full 1000 ticks.
- **Reports.** Eleven short log-only runs ending around each toggle.
- **Save and resume.** A prefix to tick 801 with save, then a reopened run to
  finish.
- **Full captures.** Overview and profile, every 8 ticks, 126 frames each.
- **Windows.** Each window saves at its start tick, then reopens with the camera:

| Window | Camera | Ticks | Capture every | Frames |
|---|---|---|---|---|
| up-launch | west-gap | 100–180 | 1 | 81 |
| bridge-reverse | east-gap | 577–657 | 1 | 81 |
| ret-full | west-gap | 761–849 | 1 | 89 |
| ret-mid, reopened mid-drop | west-gap | 801–849 | 1 | 49 |
| goal-feet | goal-feet | 416–496 | 2 | 41 |

All runs are 960×540, within 128 frames and 256 MiB raw. Helpers refuse an
existing output directory.

### Run results

| Command | Result |
|---|---|
| `npm ci` | 0 vulnerabilities, success |
| strict `tsc -p tools/tsconfig.json`, inside helper | pass |
| `run_set.sh` A and B, each under `caffeinate -s -i` | both complete, about 1 min 3 s together; 0 failed attempts from the 50 ms budget |
| `trace_check.py compare` | all logs, frames and states identical |
| `trace_check.py window` A and B | 49 of 49 frames identical, 0 log mismatches |
| `trace_check.py resume` A and B | identical except session-local generation |
| `tools/probes/navigation-links.py` against the same binary | `passed: true`, mid-link progress 0.30555555555555564 |

Raw runs stay in the ignored `artifacts/0028-off-mesh/`, at 33 MB per set. Three
early composition trials, `try1`, `comp1` and `comp2`, are also there. They
predate the bolder chevrons and the return-arrow swap fix and are not evidence.

## Screenshots

All are unmodified 960×540 engine frames copied from set A. Hashes are in
`evidence/screenshots.sha256`.

| File | Shows |
|---|---|
| `screenshots/overview-t0000-start-return-link-disabled.png` | Start; grey disabled return pads |
| `screenshots/overview-t0136-directed-jump-up.png` | Mid jump-up, with its amber arc |
| `screenshots/overview-t0480-goal-return-still-disabled.png` | Paused at the goal; return still disabled |
| `screenshots/overview-t0488-return-link-enabled.png` | Return pads green after the enable command |
| `screenshots/overview-t1000-home-return-disabled-again.png` | Home; return disabled again; all arcs |
| `screenshots/profile-t0616-bridge-reversed-apex.png` | Reversed bridge apex in true profile |
| `screenshots/profile-t1000-all-arcs.png` | Jump-up and return drop coincide in profile |
| `screenshots/west-gap-t0108-launch-up.png` | Launch tick |
| `screenshots/west-gap-t0140-mid-jump-up.png` | Mid-gap, shadow on the trench floor |
| `screenshots/west-gap-t0173-landing-up.png` | Landing on the snapped Centre endpoint |
| `screenshots/east-gap-t0617-bridge-reversed-mid.png` | Reversed bridge mid-air; visor leading west |
| `screenshots/west-gap-t0801-reopened-mid-drop.png` | First frame after reopening mid-drop, identical to the uninterrupted frame |
| `screenshots/west-gap-t0834-landing-after-reopen.png` | Return-drop landing after reopening |
| `screenshots/goal-feet-t0470-hover-above-surface.png` | The 5 cm hover and detached shadow |

## Changed paths

- `handoffs/0028-off-mesh-lookdev/result.md`
- `handoffs/0028-off-mesh-lookdev/tools/off_mesh_course.py` builds the scene and runs plays.
- `handoffs/0028-off-mesh-lookdev/tools/off_mesh_course.ts` is the strict TypeScript behavior.
- `handoffs/0028-off-mesh-lookdev/tools/tsconfig.json`
- `handoffs/0028-off-mesh-lookdev/tools/run_set.sh` is the exact evidence-set invocation.
- `handoffs/0028-off-mesh-lookdev/tools/trace_check.py` is the independent checker.
- `handoffs/0028-off-mesh-lookdev/evidence/` holds the summaries, resume, window and repeat comparisons, events, geometry, helper record and screenshot hashes.
- `handoffs/0028-off-mesh-lookdev/screenshots/` holds 14 PNGs.

## Limitations

- **No physics.** There are no Rapier bodies, collisions or physics grounding.
  Clearance numbers are logical geometry only.
- **Jumps are behavior-owned.** The parabola is a logical transition and not a
  validated or safe trajectory.
- **Session-local generation.** Generation values are session-local, so
  reopened runs report different numbers by design.
- **One machine.** Determinism was measured on one machine, one GPU and one
  binary. No cross-device claim is made.
- **No native UI.** Native editor capture and link debug draw were not
  exercised. The Mac's lock state was not touched.
- **Script budget.** The two recorded runs had zero 50 ms wall-clock failures.
  Both ran with temporary sleep prevention under `caffeinate -s -i`, applied to
  the outer ACP runner and to each `run_set.sh` command. This does not show
  robustness under host sleep or CPU contention. The full-course reference run
  took 1899.465292 ms wall time for 1000 log-only ticks.
- **Unconfirmed boundary.** The erosion explanation in the second finding is an
  inference.

## Open questions

1. Should behaviors be expected to raycast-ground walking agents until route
   heights conform to the source? Or should quantized heights be corrected below
   cell height in the engine?
2. Should rebuild reports expose the last actual rebuild separately from
   unchanged-tick reporting?
3. Should a later handoff add gravity-timed jump arcs and landing absorption, or
   do they belong in a shared gameplay helper?
