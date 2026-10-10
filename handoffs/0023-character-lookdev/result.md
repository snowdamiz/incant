# 0023 character movement look-dev: result

## Status

Scoped verdict: **headless real-engine look-dev and rendered motion review are
complete for this handoff.** All five behaviours read correctly on screen and in
the logs: walk, low-step climb, tall-obstacle stop, wall slide and jump/landing.
There is also a ramp/snap-down lane. **Three real defects or questions are reported
to Astra below.** Numeric fixes belong to Astra. No native static review was done,
because this session has no native CUA. Specific captures are requested below.
This verdict does not approve a phase gate. It does not certify the movement
runtime, the CLI or platform performance, and it does not claim production
character art.

Exact model and transport: Claude Opus 5.5, model ID `claude-opus-5-5`, through
the director's Claude subscription via ACP. No model substitution occurred.

Packet check: the current brief (`c094719`) has no priority-revision or
director-feedback section, so nothing new needed acknowledging. This was a fresh
attempt. The worktree was clean at `e8dd4d0`, with no files from an earlier attempt.
No Inspector, shell or design-system files were touched. The left Assets
workspace, the diagnostics-only bottom dock and the neutral shell are unchanged.

Binary: `artifacts/tools/incant_headless`. Its sha256 was checked against
`artifacts/tools/binary.json`:
`b4f5387f…46ca1c0` (source `e8dd4d0`). No Rust was compiled. No other worktree's
target directory was used.

## What was built

Everything is under `handoffs/0023-character-lookdev/tools/`.

- `character_lookdev.py` is the reproducible helper. It refuses an existing output
  directory (verified: `Output directory already exists; choose a new directory.`
  and exit 1). It works only through engine interfaces:
  1. `init`
  2. For each piece, writes a glTF with the dimensions baked into the vertices,
     then calls `import`.
  3. One `rpc` `command.execute` transaction, with a journal, creates all 22
     entities (18 bodies/pieces, 3 cameras, 1 sun), followed by `project.save`.
  4. `validate`
  5. `node tools/build_script.mjs`
  6. `play --compiled-script --camera --log-output` at 960×540, once per camera.

  Every physics root has unit scale. Each collider box or capsule matches its own
  mesh exactly; the ramps are rotated boxes, not scaled ones.
- `character_course.ts` is the behaviour, typechecked under `strict`.
  - Every `Character · <lane>` entity calls `api.computeCharacterMotion` once per
    tick. All lanes share one option set: offset 0.02, slide on, 45° climb/slide
    limits, snap 0.3 m, and autostep with max 0.25, min width 0.15, no dynamic bodies.
  - It applies `translation/dt` as an ordinary `set_component Velocity` command.
  - The behaviour owns gravity (9.81 m/s²) and a single jump (4.0 m/s, triggered
    at x ≥ −0.65).
  - It logs one compact per-tick debug row plus jump, land and contact-change
    events. It uses no solver handles and no other mutation path.
- `trace_summary.py` summarizes or slices the per-tick logs.
- `tsconfig.json` is the strict typecheck config for the behaviour.

### Scene design

There are five parallel lanes along +X, 1.8 m apart. Every capsule starts at
x = −3.5 and walks at 1.6 m/s for 5 s (300 ticks at 60 Hz). Rendering is
`material_preview`, with one shadowed directional light and no environment map.

| Lane (near → far) | Test piece and colour | What it demonstrates |
|---|---|---|
| jump (z 3.6) | 0.35 m amber hurdle at x 0 | Takeoff, clearance, landing |
| step (z 1.8) | 0.20 m blue platform, x 0…2.8 | Autostep climb, walk, step-off with snap |
| block (z 0) | 0.60 m red block at x 0…0.8 | Stops (taller than the 0.25 autostep) |
| ramp (z −1.8) | Green 17° up-ramp, flat top, 27° down-ramp | Slope climb, snap-down on descent |
| wall (z −3.6) | 1.0 m slate wall along X | Diagonal input slides along the wall |

Characters are a single warm off-white capsule (1.6 m tall, radius 0.3). Test pieces
use one saturated, distinct hue each, so the obstacle type reads without labels.
The floor is neutral grey, with darker 5 mm lane strips. The strips are visual
only and have no collider; they sit inside the 0.02 m skin offset, below the
capsule. The background is the renderer's clear colour.

I chose the cameras after rejecting several attempts, all kept under ignored
`artifacts/0023-character/a1`, `a2`, `r2` and `t3`–`t5`:

- A front-on camera stacks all five characters into one column, because every
  lane moves in lockstep.
- A steep top-down camera hides the step and jump heights.
- **overview** is a high oblique view of the whole course.
- **contact** is a low oblique view of the jump and step-climb lanes.
- **exit** is a low view from behind the step's far end, to show the step-off.

## Commands and results (run from the worktree root)

```sh
shasum -a 256 artifacts/tools/incant_headless          # matches binary.json
npm ci                                                  # SWC local modules; 0 vulnerabilities
node_modules/.bin/tsc -p handoffs/0023-character-lookdev/tools/tsconfig.json   # exit 0
python3 -I handoffs/0023-character-lookdev/tools/character_lookdev.py artifacts/0023-character/f1 \
  --run overview:overview:3 --run contact:contact:3 --run exit:exit:3      # 10.7 s wall
python3 -I handoffs/0023-character-lookdev/tools/character_lookdev.py artifacts/0023-character/f1-repeat \
  --run overview:overview:3 --run contact:contact:3 --run exit:exit:3      # 9.8 s wall
python3 -I handoffs/0023-character-lookdev/tools/trace_summary.py artifacts/0023-character/f1/overview.logs.jsonl
artifacts/tools/incant_headless script f1/character.incant.json f1/character_course.js --ticks 300   # ×3
artifacts/tools/incant_headless play f1/character.incant.json --seconds 5 --compiled-script f1/character_course.js  # ×3, no GPU
```

Capture plan per run:

- 300 ticks with `--capture-every 3` gives 101 frames.
- 101 × 960 × 540 × 4 = 209,433,600 bytes (199.7 MiB) of raw pixels. This is
  under both caps of 128 frames and 256 MiB.
- Every report has `completed: true`, 300 ticks, 1500 script commands (5 per
  tick), adapter `Apple M5 Pro`, 18 model entities and 8156 triangles.

### Repeat-run stability (local equality only)

`f1` and `f1-repeat` were each built from scratch by the helper.

- **Frames:** 303/303 frames across all three cameras are byte-identical PNGs.
- **Logs:** each camera's log JSONL is identical across the two runs. It is also
  identical across the three cameras within a run.
- **Reports:** after the engine-minted scene and asset ULIDs are normalized, each
  `report.json` is equal except for `wall_ms`. The scene and asset IDs are the only
  differences between the two project files.

This shows local equality on one machine (Apple M5 Pro, macOS 25.6) with one
binary. It is **not** evidence of cross-platform or cross-GPU determinism.

## Motion verdict

These values come from the logs, checked against the frames. The rest height is
y = 0.82 (capsule half-height 0.8 plus the 0.02 offset).

- **Walk:** 0.0267 m/tick (1.6 m/s) on flat ground. Grounded on every walking tick.
- **Low step (0.20 m):**
  - The character climbs and stays on top at y = 1.02 (+0.20 exactly).
  - Disabling autostep (diagnostic `artifacts/0023-character/diag/no_autostep.js`)
    stops it at x = −0.297, which shows autostep is what climbs it.
  - At the far end, the snap keeps it grounded through the 0.2 m step-off. It rolls
    over the edge on its hemisphere in about 10 ticks and returns to y = 0.82 with
    no airborne tick. This reads well in the `exit` frames.
- **Tall block (0.60 m):** stops at x = −0.320, which is the face minus the radius
  minus the offset, exactly. It holds there for 177 ticks (124–300) with zero translation and
  no jitter, at y between 0.819 and 0.82.
- **Wall slide:**
  - The diagonal input (1, −0.6) reaches the wall at tick 54.
  - From then on z is pinned at −3.93, which is the face minus the radius minus the
    offset, exactly.
  - The tangential x motion continues at 0.023 m/tick, the full tangential share
    of the input.
- **Jump:**
  - Takeoff at tick 108 (x = −0.647); airborne for ticks 108–154 (0.78 s).
  - The apex puts the capsule bottom 0.80 m above the floor, so it clears the
    0.35 m hurdle with no contact.
  - Lands at tick 155 at −3.85 m/s and is at rest height the next tick. No bounce,
    and at most 1 mm of penetration.
- **Ramp:**
  - Climbs the 17° ramp at 0.024 m/tick horizontally, about 10% slower than on
    flat ground (consistent with slope projection).
  - Grounded across the top.
  - Follows the 27° descent at −0.013 m/tick vertical with no airborne ticks, so
    the snap-down works.

### Real issues for Astra (numeric/runtime; not fixed here)

1. **Intermittent one-tick stalls (zero translation) during ordinary walking.** The
   query returns `[0,0,0]` for a single tick, then resumes. In `f1`:
   - block lane, tick 6: flat floor, one contact, x = −3.367
   - wall lane, tick 59: sliding along the wall
   - ramp lane, tick 248: flat floor after the descent, x = 2.822

   With autostep disabled, the wall lane stalls at ticks 59, 138, 180 and 215.
   Each stall costs 2.7 cm and leaves Velocity at zero for that tick. This is a
   one-frame hitch. It is not obvious in captures taken every 3 ticks, but it
   would show in a 60 Hz camera follow. The two flat-floor stalls (ticks 6 and
   248) coincide with `sliding_down_slope` being false; the wall stall (tick 59)
   does not, so that flag is not a reliable predictor.

   Repro: run the helper, then
   `trace_summary.py <run>/overview.logs.jsonl block 4 8`.
2. **`sliding_down_slope` is true on flat ground most of the time.** It was true on
   218–295 of 300 ticks per lane, including 291 of 300 for the mostly
   stationary block lane. On the flat top of the step it flickers between true
   and false from tick to tick. Behaviours cannot rely on it to
   mean "on a too-steep slope". Either the semantics need documenting or the flag
   needs gating (for example on the slope angle exceeding `min_slope_slide_angle`).
3. **The autostep climb is gradual and slows forward motion sharply.** The 0.20 m
   step takes about 18 ticks (0.3 s) to climb. Horizontal speed drops to 15–40% of
   input (0.004–0.011 m/tick) while the capsule rises about 0.012 m/tick. It reads
   as a smooth ride-up rather than a pop, which is visually acceptable for a
   capsule. A game designer expecting the stair step to keep walk speed would see
   a hitch. Also, enabling autostep changed wall-slide progress: the final x was
   3.226 with it and 3.134 without, because of the extra stalls described above.

### Performance observations (this Mac only; not platform budgets)

- **Behaviour plus physics only** (`script --ticks 300`, 5 characters, 80 of 256
  query units per tick, including one JSON debug log per tick): p95 was 0.74, 0.87
  and 0.88 ms per tick over three runs; maximum 1.12–1.24 ms.
- **`play` with no GPU** (300 ticks, including report and logs): 190–199 ms wall.
- **`play` with 101 GPU captures at 960×540:** 2.3–3.7 s wall per run.
- **Full helper** (build, import and three captured runs): about 10 s.

## Screenshots (public, real engine frames from `f1`; no accounts or credentials)

All are under `handoffs/0023-character-lookdev/screenshots/`. Each is an unedited
`play` frame, byte-identical to the same tick in `f1-repeat`.

| File | Camera | Tick | Shows |
|---|---|---|---|
| `overview-t000-start.png` | overview | 0 | Course and lanes, all at the start line |
| `overview-t132-jump-apex-step-climb.png` | overview | 132 | Jumper near apex over hurdle; step climb; block stop |
| `overview-t204-ramp-descent-wall-slide.png` | overview | 204 | Ramp descent; wall slide; step lane on top |
| `overview-t300-end.png` | overview | 300 | Final poses; blocked character waits at the red block |
| `contact-t108-takeoff.png` | contact | 108 | Jump takeoff before the hurdle |
| `contact-t129-over-hurdle-step-climb.png` | contact | 129 | Airborne over hurdle; step lane mid-climb |
| `contact-t156-landed.png` | contact | 156 | Landed beyond hurdle; contact shadow |
| `exit-t240-step-edge.png` | exit | 240 | Step lane at the platform's far edge |
| `exit-t255-rolling-off.png` | exit | 255 | Snapped step-off, mid-descent |
| `exit-t270-on-floor.png` | exit | 270 | Back on the floor, grounded |

All other raw output is in ignored `artifacts/0023-character/`:

- projects, journals and imported models
- compiled JS and source maps
- all 606 frames of `f1` and `f1-repeat`
- reports, logs, the autostep diagnostic and contact sheets
- earlier camera iterations

### Visual notes and limitations

- The camera angles were chosen for readability, not to hide defects. The stalls
  above come from the logs, because one-tick hitches fall between captures taken
  every 3 ticks.
- Characters move in lockstep. Low angles therefore overlap them at some ticks,
  which is why the review uses three cameras rather than one.
- There is no facing indicator, animation or character art. A capsule is the
  correct proxy for this primitive-collider review.
- `material_preview` shading has no environment light. The floor and pieces read
  slightly flat. This is acceptable for motion review, but it is not look-dev
  for a shipping environment.
- The behaviour finds lanes by entity name (`Character · <lane>`). This is a
  demo convention, not a proposed API.

## Native captures requested from Astra (no native CUA in this session)

Optional static check. Open `artifacts/0023-character/f1/character.incant.json` in
the native editor and capture:

1. The viewport through `Camera · overview`
   (`01JA2CHAR0000000000000002T`) at 960×540, to compare against
   `overview-t000-start.png` for material and shadow parity.
2. The Outliner/Inspector with `Character · step` selected, to confirm that the
   kinematic RigidBody, capsule Collider and Velocity render with the 0022 physics
   presentation. No new component was added.

Neither capture is required for this verdict, and no native play controls are
claimed.

## Changed paths

- `handoffs/0023-character-lookdev/result.md`
- `handoffs/0023-character-lookdev/tools/character_lookdev.py`
- `handoffs/0023-character-lookdev/tools/character_course.ts`
- `handoffs/0023-character-lookdev/tools/trace_summary.py`
- `handoffs/0023-character-lookdev/tools/tsconfig.json`
- `handoffs/0023-character-lookdev/screenshots/*.png` (10 files, 436 KB)

## Unavailable or not run

- Native editor capture, because there is no CUA in this ACP session (requested
  above).
- Cross-machine and cross-GPU determinism, and other platforms' performance.
- Rust tests and clippy, because no Rust was changed and the binary was reused as
  directed.
- Hosted CI.

## Open questions

1. Should `sliding_down_slope` be gated by `min_slope_slide_angle`, or documented
   as Rapier's raw flag?
2. Is the one-tick zero-translation stall known Rapier behaviour (for example,
   from the snap or offset interaction)? If so, should the runtime retry or
   document it?
3. Should the default step-climb keep horizontal speed, for example by stepping
   in one tick when the landing is clear?
