# 0023 character movement look-dev: result

## Status

Scoped verdict: **I verified the corrected movement engine in actual rendered
motion and every-tick logs. The two reported runtime defects are resolved in
this course.** The remaining step-climb slowdown matches rounded-capsule
geometry; it is not a numerical defect. It is still a game-feel limitation,
explained below.

This verdict does not approve a phase gate. It does not certify the movement
runtime, the CLI, platform performance or cross-device determinism. It does not
claim production character art.

Exact model and transport: Claude Opus 5.5, model ID `claude-opus-5-5`, through
the director's Claude subscription via ACP. No model substitution occurred.

### Priority revision acknowledged

- The brief at `2de0c06` asks for verification of Astra's corrections:
  - box-face normal recovery from the face witness
  - the downhill flag derived from final support and actual downward travel
- It asks for a fresh course run plus an independent repeat into new directories.
- It asks for every-tick log review, a step-climb quantification, replaced
  screenshots and a rewritten result.

All of this was done. There was no other new director feedback. No UI changes
or native capture were requested, and none were made. The left Assets workspace,
the diagnostics-only bottom dock and the neutral shell are untouched. No evidence
below reuses frames or logs from the old engine, except where it is explicitly
labelled as the "before" comparison.

Binary: `artifacts/tools/incant_headless`. Its sha256 is
`8de710ffa59e97123aa598f62b2a2a5bb3f2faccbab7e600b8b3ac130502d99a`, which matches
`artifacts/tools/binary.json` (source `2a510cf`). No Rust was compiled. No other
worktree's target directory was used.

## Original bugs and their resolution

The first review used binary `b4f5387f…46ca1c0` (source `e8dd4d0`), in run
`artifacts/0023-character/f1`.

| # | Reported issue | Before (`f1`) | After (`v2` and `v2-repeat`) |
|---|---|---|---|
| 1 | One-tick stalls while walking | 16 ticks below 50% of input progress, across all five lanes. Three were exact zeros (block t6, wall t59, ramp t248); 13 were 1–2 mm ticks (for example, wall t7/20/27/40, step t73/105/155/208). With autostep off, the wall lane had 3 more exact zeros. | **0** outside the designed slowdowns, on every tick of every lane, with autostep on and off. The wall lane keeps 0.023 m/tick (100% of its tangential input) on every tick from first contact to the end. |
| 2 | `sliding_down_slope` true on flat ground | True on 218–295 of 300 ticks per lane, including 291 for the mostly stationary block lane | **False on every tick of every lane, except step lane ticks 252–253.** That is the rounded capsule moving downward over the step's 90° edge (see below). It is never true on flat floor, the wall, the ramp or the landing. |
| 3 | Slow autostep climb | 18 ticks; horizontal speed 15–40% of input | Unchanged in shape; quantified and classified below |

Side effects of the fix that can be observed in this course:

- Characters reach obstacles 2–3 ticks earlier, because they no longer lose
  distance to stalls. The block-lane contact moved from t123 to t120.
- Final x positions increased by 0.03–0.11 m.
- Wall-slide progress no longer depends on autostep: the final x is 3.337 in both
  configurations. Before the fix it was 3.226 with autostep and 3.134 without.

## Step-climb slowdown: quantified and classified

The 0.20 m step lane (`v2/overview.logs.jsonl`, ticks 118–140):

- Contact at t121, x = −0.300.
- On top at t139, x = −0.022, y = 1.020. The climb takes **18 ticks (0.30 s)**.
- Covering the same 0.278 m at walking speed would take 10.4 ticks, so the step
  costs **about 7.6 ticks (0.13 s)**.
- Horizontal progress during the climb is 0.005–0.026 m/tick, that is **19–97%**
  of input. It ramps up smoothly: 19% at contact, 50% at t130 and 90% by t136.
- One irregular tick, t128, moves (0.027, 0.027): a 2.7 cm upward pop between
  1.2 cm ticks. At this camera distance and a 3-tick capture interval it is not
  visible in the frames.

Classification:

- **The path is the rounded-capsule trajectory around the step edge.** The lower
  hemisphere's centre stays r + offset = 0.32 m from the corner.
  - At x = −0.247, the predicted height is y = 0.82 − 0.12 + √(0.32² − 0.247²) = 0.903.
  - The log has 0.905.
  - The per-tick horizontal fraction matches projecting the walking input onto
    the arc's tangent. At contact the tangent is 68° from horizontal, which gives
    an expected 37% along-arc speed, about 0.004–0.005 m/tick horizontally. The
    log has 0.005–0.006.
- **Autostep is what permits it.** With autostep disabled on the new binary
  (`diag-v2`), the character stops at x = −0.295, as expected, since the edge
  normal is steeper than the 45° climb limit.
- **So this is not a stall or numerical defect.** It is the controller sliding
  the input along a curved contact, with no speed preservation. I see **no reason
  to change the runtime for correctness.**
- **As game feel, it is a real limitation.** A 0.13 s hitch on every low step
  would read as "sticky stairs" in a third-person game. Fixing it needs a design
  decision rather than a numerical fix: either a one-tick vertical step when the
  landing is clear, or a behaviour-side speed preservation option. The decision
  belongs to Astra and the director.

The step-off at the far end:

- Over ticks 247–255 the capsule rolls down over the edge, staying grounded
  through the snap, and is at rest height (0.82) at t256.
- `sliding_down_slope` is true for t252–253, when the support is the step's
  edge, its normal is steeper than 45°, and the capsule is moving down.
- That fits the new documented definition ("moving downward along a contacted
  slope steeper than the configured limit"). A game that drives a slide animation
  from this flag would see a 2-tick blip on every step-down, though. I'm noting
  it as a semantic edge case, not a defect.

## Motion verdict (corrected engine; `v2`, identical in `v2-repeat`)

The rest height is y = 0.82 (capsule half-height 0.8 plus the 0.02 offset).

- **Walk:** 0.0267 m/tick (1.6 m/s) on every flat-floor tick of every lane;
  grounded throughout.
- **Tall block (0.60 m):** contact at t120. Stopped at x = −0.320, which is the
  face minus the radius minus the offset. Zero translation from t121 to t300,
  with y fixed at 0.820.
- **Wall slide:**
  - First contact at t54.
  - z is pinned at −3.930 (the face minus the radius minus the offset).
  - x advances 0.023 m/tick on every tick. There are no stalls, so the earlier
    tick-59 hitch is gone.
- **Jump:**
  - Takeoff at t108 (x = −0.647); airborne for t108–154 (0.78 s).
  - The capsule bottom peaks 0.80 m above the floor and clears the 0.35 m hurdle.
  - Grounded at t155 and at rest height from t156. No bounce; at most 1 mm below
    rest.
- **Ramp:**
  - Uphill horizontal progress on the 17° ramp is 0.023–0.024 m/tick (about
    88–90%). That is consistent with projecting the input onto the slope:
    cos 17° × 0.0267 = 0.0255, minus mm rounding.
  - The top is flat and grounded.
  - The 27° descent is grounded on every tick through the snap, with no downhill
    flag (it is under the 45° limit). Back on the floor at full speed.
- **Frames.** I reviewed the frames for all three cameras:
  - no penetration, popping or jitter on the step, block, wall or landing
  - contact shadows stay attached
  - the climb reads as a smooth ride-up over the edge, and the step-off as a
    short roll-down

  The motion matches the logs.

## Repeat-run stability (local equality only)

`v2` and `v2-repeat` were each built from scratch by the helper.

- **Frames:** 303/303 frames across the overview, contact and exit cameras are
  byte-identical PNGs.
- **Logs:** identical across the two runs, and across all three cameras within
  each run.
- **Reports:** each `report.json` is equal once the engine-minted scene and asset
  ULIDs are normalized, apart from `wall_ms`.
- **Old versus new:** 300 of the 303 frames differ from the old-engine `f1`
  frames. The identical ones are tick 0 of each camera. So the retained
  screenshots genuinely come from the corrected engine.

This is local equality on one machine (Apple M5 Pro, macOS 25.6) with one
binary. It is **not** evidence of cross-platform or cross-GPU determinism.

## Commands and results (run from the worktree root)

```sh
shasum -a 256 artifacts/tools/incant_headless      # 8de710ff…502d99a = binary.json
node_modules/.bin/tsc -p handoffs/0023-character-lookdev/tools/tsconfig.json   # strict, exit 0
python3 -I handoffs/0023-character-lookdev/tools/character_lookdev.py artifacts/0023-character/v2 \
  --run overview:overview:3 --run contact:contact:3 --run exit:exit:3          # 6.8 s wall
python3 -I handoffs/0023-character-lookdev/tools/character_lookdev.py artifacts/0023-character/v2-repeat \
  --run overview:overview:3 --run contact:contact:3 --run exit:exit:3          # 7.3 s wall
python3 -I handoffs/0023-character-lookdev/tools/character_lookdev.py artifacts/0023-character/v2   # refused, exit 1
python3 -I handoffs/0023-character-lookdev/tools/trace_summary.py artifacts/0023-character/v2/overview.logs.jsonl --check
python3 -I handoffs/0023-character-lookdev/tools/trace_summary.py artifacts/0023-character/v2/overview.logs.jsonl step 118 140
artifacts/tools/incant_headless script v2/character.incant.json v2/character_course.js --ticks 300   # ×3
artifacts/tools/incant_headless play v2/character.incant.json --seconds 5 --compiled-script v2/character_course.js   # ×3
artifacts/tools/incant_headless play v2/character.incant.json --seconds 5 --compiled-script diag-v2/no_autostep.js \
  --log-output diag-v2/no_autostep.logs.jsonl                                  # autostep-off diagnostic
```

The `npm ci` from the first session was still present (SWC 1.16.13, TypeScript 5.9.3).

The compiled behaviour JS is byte-identical to the first session's, so only the
engine changed. Each report has `completed: true`, 300 ticks, 1500 Velocity
commands, adapter `Apple M5 Pro`, 18 model entities and 8156 triangles.

Capture plan per run:

- 300 ticks with `--capture-every 3` gives 101 frames.
- 101 × 960 × 540 × 4 = 209,433,600 bytes (199.7 MiB). This is under both caps of
  128 frames and 256 MiB.

`trace_summary.py --check` scans every tick of every lane and reports:

- spans with +X progress below 95% of input (allowing for 1 mm log rounding)
- every tick with `sliding_down_slope` true
- airborne spans

On `v2` the only slow spans are the designed ones: the step climb t121–136, the
block stop t120–300 and the ramp ascent t71–137.

## Performance observations (this Mac only; not platform budgets)

- **Behaviour plus physics** (`script --ticks 300`; 5 characters; 80 of 256 query
  units per tick; one JSON debug log per tick): p95 was 0.54, 0.55 and 0.56 ms per
  tick over three runs; maximum 0.77–0.94 ms. The first session's binary measured
  p95 0.74–0.88 ms. These were single-session measurements, so I am not claiming
  a speed-up.
- **`play` with no GPU** (300 ticks): 142–143 ms wall.
- **`play` with 101 GPU captures at 960×540:** 1.7–2.2 s wall per run.
- **Full helper** (build, import and three captured runs): about 7 s.

## What was built (unchanged from the first review except `--check`)

Everything is under `handoffs/0023-character-lookdev/tools/`.

- `character_lookdev.py` is the reproducible helper. It refuses an existing output
  directory. It works only through engine interfaces:
  1. `init`
  2. For each piece, writes a glTF with the dimensions baked into the vertices,
     then calls `import`.
  3. One `rpc` `command.execute` transaction, with a journal, creates all 22
     entities (18 bodies/pieces, 3 cameras, 1 sun), followed by `project.save`.
  4. `validate`
  5. `node tools/build_script.mjs`
  6. `play --compiled-script --camera --log-output` at 960×540, once per camera.

  Every physics root has unit scale. Each collider matches its own mesh exactly;
  the ramps are rotated boxes, not scaled ones.
- `character_course.ts` is the behaviour, typechecked under `strict`.
  - Every `Character · <lane>` entity calls `api.computeCharacterMotion` once per
    tick. All lanes share one option set: offset 0.02, slide on, 45° limits,
    snap 0.3 m, and autostep with max 0.25 and min width 0.15.
  - It applies `translation/dt` through an ordinary Velocity `set_component`
    command.
  - The behaviour owns gravity (9.81 m/s²) and one jump (4.0 m/s).
  - It logs one compact row per tick, plus jump, land and contact events.
- `trace_summary.py` summarizes, slices and (with `--check`) scans every tick.
- `tsconfig.json` is the strict typecheck config.

### Scene

There are five lanes along +X, 1.8 m apart. Every capsule (1.6 m tall, r 0.3,
warm off-white) starts at x = −3.5 and walks at 1.6 m/s for 5 s at 60 Hz.

| Lane (near → far) | Piece and colour | Demonstrates |
|---|---|---|
| jump (z 3.6) | 0.35 m amber hurdle | Takeoff, clearance, landing |
| step (z 1.8) | 0.20 m blue platform, x 0…2.8 | Autostep climb, step-off with snap |
| block (z 0) | 0.60 m red block | Stop at an obstacle taller than the autostep |
| ramp (z −1.8) | Green 17° up, flat top, 27° down | Slope climb, snap-down descent |
| wall (z −3.6) | 1.0 m slate wall | Diagonal input slides along the wall |

The rest of the scene:

- The floor is neutral grey, with darker 5 mm lane strips. The strips are visual
  only and have no collider; they sit inside the skin offset.
- One shadowed sun; `material_preview` shading; no environment light.
- Three cameras:
  - **overview**: high oblique view of the whole course.
  - **contact**: low view of the jump and step climb.
  - **exit**: low view from behind the step's far end.
- Lower or front-on single views overlap the lockstep characters. The rejected
  angles remain in the ignored artifacts.

## Screenshots (public; current corrected-engine frames from `v2`)

All are under `handoffs/0023-character-lookdev/screenshots/`. Each is an unedited
`play` frame, byte-identical to the same tick in `v2-repeat`. The first review's
old-engine frames were removed.

| File | Camera | Tick | Shows |
|---|---|---|---|
| `overview-t000-start.png` | overview | 0 | Course and lanes at the start line |
| `overview-t132-jump-step-climb-block-stop.png` | overview | 132 | Jumper airborne over the hurdle; step lane mid-climb; block stop |
| `overview-t204-ramp-descent-wall-slide.png` | overview | 204 | Ramp descent; wall slide; step lane on top |
| `overview-t300-end.png` | overview | 300 | Final poses; blocked character held at the red block |
| `contact-t108-takeoff.png` | contact | 108 | Jump takeoff in front of the hurdle |
| `contact-t129-over-hurdle-step-climb.png` | contact | 129 | Airborne over the hurdle; step lane on the edge arc |
| `contact-t156-landed.png` | contact | 156 | Landed at rest height beyond the hurdle |
| `exit-t243-on-step.png` | exit | 243 | Step lane walking on the platform before the edge |
| `exit-t252-over-edge.png` | exit | 252 | Rolling over the edge, grounded through the snap |
| `exit-t258-on-floor.png` | exit | 258 | Back on the floor at rest height |

All raw output is in ignored `artifacts/0023-character/`:

- `v2`, `v2-repeat` and `diag-v2`: the corrected engine
- `f1`, `f1-repeat` and `diag`: the original engine, kept for the before/after
  comparison
- earlier camera iterations and contact sheets

## Limitations

- The course and its checks only cover this geometry: flat floor, box step,
  block, wall, a 17°/27° box ramp, and one capsule size. Moving platforms, mesh
  terrain, other shapes, high speeds and large-world coordinates were not
  exercised here. Astra's regression covers sphere and box characters.
- Captures every 3 ticks cannot show one-tick events. Per-tick claims come from
  the logs, which are rounded to 1 mm.
- Characters move in lockstep, so low cameras overlap them at some ticks.
- There is no facing indicator, animation or character art. `material_preview`
  without an environment light reads slightly flat. That is fine for motion
  review, but it is not shipping look-dev.
- Stability was checked on one machine and one GPU only. No hosted CI, Rust tests
  or Clippy were run here (no Rust changed). Native static capture was not
  requested for this query-only increment.

## Changed paths

- `handoffs/0023-character-lookdev/result.md` (rewritten)
- `handoffs/0023-character-lookdev/tools/trace_summary.py` (added `--check`)
- `handoffs/0023-character-lookdev/screenshots/*.png`: the old-engine frames were
  removed and replaced by 10 current frames (436 KB)

## Open questions for Astra and the director

1. Should low-step traversal preserve walking speed (game feel)? For example, a
   one-tick vertical step when the landing is clear, or an opt-in option. The
   current behaviour is geometrically correct but costs about 0.13 s per 0.20 m step.
2. Should `sliding_down_slope` exclude brief edge or corner support (the 2-tick
   blip at a step-down)? Or should the docs warn behaviours to debounce it?
3. Is the single 2.7 cm vertical tick mid-climb (t128) expected from Rapier's
   autostep, or worth smoothing?
