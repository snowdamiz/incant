# 0060 Core Sample level, art and game UI — result

## Status

**Implemented and reviewed at native pixels.** "Mesa survey" is one small
third-person level: a survey robot crosses a red-rock canyon, recovers four
glowing rock cores while three ember mites chase it, and reaches the lift on
the north rim. It contains:

- **Authoring.** `samples/core/scene.py` authors everything through public
  `incant_cmd` transactions, with stable IDs, Undo/Redo and a reopen check.
- **Assets.** Original, deterministic art and audio generators live in
  `samples/core/art`.
- **HUD.** A localized screen-space HUD in English, pseudo-locale (`en-XA`),
  Arabic (RTL) and Simplified Chinese (CJK), using the engine's own shaping and
  font fallback.
- **Gameplay.** It runs on the unmodified `gameplay.ts`, with a win route
  planned against the real simulation.

Review routes:

- **Packaged player, CPU route.** Playtests: win, lose and locked exit.
- **Offscreen, real GPU game render path.** Every locale and every HUD state.
- **Ordinary native-window player.** Start, resize, a sample pickup, all four
  locales and the lost state.

**Two defects found and fixed during the native-window review.** After each
fix I rebuilt everything and re-ran every check.

- **f3 → f4, camera occlusion.** Taking sample 1 from the west put a 4.1 m
  hoodoo between the fixed follow camera and the robot, and it filled about
  40% of the frame. I moved it against the west wall.
- **f4 → f5, HUD wrap at 420 pt.** At the 420×320 pt minimum, the English
  controls hint wrapped inside the right-aligned column and left "jump" orphaned
  at the column's left edge (labels have no text alignment). The hint is now
  12/16 instead of 13/18 and the column gap 24 pt instead of 32. Every English,
  Arabic and Chinese label is a single line at 420, 640 and 960 pt.

Evidence below is from f5 unless it is marked f3 or f4. The scene is identical
in f4 and f5.

**Not claimed:**

- **Live bindings.** Live HUD text and audio bindings are Astra's integration.
  The four HUD states are authored presentation transactions, and no engine
  command can set label text yet.
- **Gates and shipping.** No device, performance or phase gate, and no shipping
  export.

## Model and effort

Claude Opus 5.5, model ID `claude-opus-5-5`, thinking effort **max**, for the
whole session (including resumptions after context compaction).

Root's ACP record `~/.codex/incant-evidence/claude-0060-acp.log` line 3 reads
"Claude session verified: Opus 5.5; thinking: max". I cannot inspect the ACP
transport myself.

## Brief

Brief SHA-256: `49c387a8773dac8dba42aae4c92d62dfcf6116ec5525d305d8dfb3de02d00843`.
It has no priority revisions and was unchanged at lease time, mid-session and
before commit. `brief.md` is root's, and I did not edit it.

## Identities

| Item | Identity |
| --- | --- |
| Source | HEAD `9ca63624`, tree `4a854e19`. Work is limited to `samples/core` (new files plus a README pointer) and this handoff. |
| Unchanged by contract | `gameplay.ts` `58d7007d…a393a5`, `verify.py` and `tsconfig.json`. Their git blobs equal HEAD (`evidence/sources.json`). |
| Toolchain | rustc 1.99.0 (b940084d7), cargo 1.99.0. Calls go through `./tools/cargo --locked -j2`, `ci` profile. `Cargo.lock` `ac059d1a…abcac7f`. Leased target `~/.codex/incant-native-target`. |
| Authoring tool | `incant_headless` (ci) `d482a991…92e7bc`, frozen for the whole session |
| Art | generated set v13: 30 files, 10,614,578 bytes. Digests are in `evidence/art.json`. |
| Final project | `f5-en-playing/project/core.json` `443f3243…5c719fcb01`, revision 29, 45 entities, 14 assets. The other six variants are in `evidence/builds.json`. |
| Final player | `f5-en-playing` ordinary `incant_game_player` (features `project,native-window`, ci): `565f1592…6a6cc5fd28`, 31,468,288 bytes. Every package's hash is in `evidence/packages.json`. |
| Host | Apple M5 Pro (Mac17,9), Metal, macOS 27.0.1 (26A434); Python 3.14.8, Node v22.23.3 |

## What was built

**Level.** Two walkable levels:

- **Canyon floor** at 0 m.
- **Upper mesa** (north plateau plus east shelf) at 1.6 m.

Three sand ramps join them, so the navmesh needs no off-mesh links. Stepped,
ragged canyon walls run down to an open north rim, where the lift headframe
stands against the sky. Beyond the rim, a valley floor and low flat-topped
buttes (210–330 m out) supply aerial perspective, since the renderer has no fog.

Props:

- Five hoodoos, six boulders, six fallen blocks and a low slab (jumpable by the
  player, an obstacle for chasers).
- Cairns, sage and talus rubble.
- A survey camp: dome tent, rover, crates and a lamp.

Collision is three static compound colliders (ground, walls, obstacles) built
from the same `art/level.py` layout as the visuals. The navmesh is generated
from exactly those colliders.

**Actors.**

- **Surveyor.** Reads from behind: a white shell, a safety-yellow canister, an
  antenna and a hover ring.
- **Mites.** Radially symmetric, dark crust with ember veins and orbiting shards.
  They need no facing because the controller never rotates bodies.
- **Sample cores.** Spinning and bobbing on yellow stands, with cyan emissive
  veins and a small cyan light each.

All actors have kinematic capsules with uniform scale. Each model is centred on
its collider.

**Colour code.** Colour carries meaning and nothing else does:

- Safety yellow: the surveyor's own equipment.
- Cyan: samples only.
- Ember orange: mites only.
- Strata reds and ochres: the land.

**HUD.** Plain type in the engine's fixed game-UI theme (light text, hard
one-pixel shadow, no panels):

- Health pips and the objective at top left.
- Sample count and controls at top right.
- The result title centred under the top bar.

There are twelve `UiNode` widgets with stable IDs. Display text is resolved by
the engine's own `localization.localize()`, never typed by hand.

**Audio.** All synthesized deterministically and authored as engine entities:

- An 84 BPM, D-Dorian, 8-bar loop (22.857 s) that loops without a seam.
- Pickup, hurt, won and lost cues.
- Music and effects buses, with one silent source per cue occurrence.

## Authoring path

`scene.py` steps:

1. `incant init`.
2. Generate the art, or copy it from `--art` after checking manifest digests.
3. Resolve every HUD string through the engine in a disposable probe project.
   Refuse any text the font chain cannot cover.
4. `incant import` every model, image, font and WAV.
5. Author everything else through RPC `command.execute`. The bus stamps
   provenance.

Transactions, for the `f5-en-won` variant:

| Transaction | Commands |
| --- | --- |
| Assign stable Core Sample asset identities | 28 |
| Author Core Sample scene, script and input | 7 |
| Build the mesa survey level | 21 |
| Author music and event cues | 12 |
| Author the localized HUD | 12 |
| Present HUD: won in en (only with `--locale/--state`) | 14 |

Checks after authoring:

- **History.** The builder undoes every transaction in reverse and compares each
  step with recorded `project.read` snapshots. It then redoes all of them,
  saves, reopens in a new RPC process, compares, and runs `incant validate`.
- **No direct writes.** No project file is written directly.
- **Results.** All seven f5 variants, and the earlier f4 set:
  `undo_redo_reopen: true`, `valid: true`, 0 diagnostics.

**Play does not mutate authoring.** `playtest.py` hashes every project file
before and after the three packaged runs (`authoring_unchanged: true`). The
`native-play` refusal re-run on the f5 project left its SHA-256 unchanged.

## Commands

From the worktree root:

```sh
./tools/cargo build --locked --profile ci -p incant_headless          # via tools/cargo.sh, -j2
(cd samples/core && python3 -B -m art.world ART)                       # 127 s, deterministic
python3 -B samples/core/scene.py OUT --binary incant_headless --art ART [--locale L --state S]
python3 -B handoffs/0060-core-sample-presentation/tools/package.py OUT/project/core.json PKG \
    --camera 01CSEN00000000000000000003                                # exporter build step, ci profile
python3 -B samples/core/playtest.py OUT PLAYTEST --player PKG/player
handoffs/0060-core-sample-presentation/tools/variants.sh ART f5 \
    "en:playing en:won en:lost en:collected en-XA:playing ar:playing zh-Hans:playing"
handoffs/0060-core-sample-presentation/tools/offscreen.sh PKG/player/data tools/plans/PLAN.json OUT [cooked|GUEST.js]
handoffs/0060-core-sample-presentation/tools/launch.sh RUN PKG                   # native window, supervised
handoffs/0060-core-sample-presentation/tools/burst.sh '…'                        # idle-gated interaction
handoffs/0060-core-sample-presentation/tools/nav_repro.sh --sweep                # navmesh sweep
python3 -B handoffs/0060-core-sample-presentation/tools/collect_evidence.py f5 v13
```

`package.py` follows `tools/export_native.py --mode window`'s build step with two
differences:

- The `ci` profile.
- The leased target. Building there keeps the 0060 lease boundary.

The package is assembled by the exporter's own `native_package.data_package`
and passes the exporter's CPU startup check.

## Checks and results

| Check | Result |
| --- | --- |
| Import through the real pipeline, Undo/Redo, reopen, validate (7 variants) | pass |
| Play leaves authoring unchanged | pass (`authoring_unchanged`) |
| `localization-check` en, ar, zh-Hans on f5 | complete, 0 missing each |
| Glyph coverage (builder) and `missing_glyphs` (renderer) | 0 missing in every locale |
| `tsc -p samples/core/tsconfig.json` (strict) | exit 0, no output |
| Python syntax, 26 new files | 0 errors |
| Navmesh sweep, 39,300 start/end pairs with the controller's own query | 36,964 routed, 2,336 unroutable, **0 errors**; worst case 1,155 of 2,048 visits, 44 points |
| Playtest `win` (planned route, `routes/win-input.json`, 88 frames) | pickups at ticks 127/508/1266/1842, one hit at 664, won at 2066 |
| Playtest `lose` (idle) | hits at 527/572/617, lost at 617 |
| Playtest `locked` (straight to the lift, no samples) | the exit refuses; lost at 608 |
| Every chaser plans a route at tick 1 | corners 5 / 32 / 24 |
| `incant_render --test model_gpu --include-ignored` | 50 passed |
| `incant_render_gpu_host --test ui` / `--test game`, ignored included | 2 / 5 passed |
| `incant_text --test shaping` / `--test bidi_wrap` | 16 / 2 passed |
| Golden handling | no evidence or bless variables set; `git status -- crates` identical before and after (empty) |
| Music in the engine's mixed output (`tools/pcm_check.py` on the win run's PCM) | continuous stereo signal for 35.9 s, per-second RMS −23.5 to −29.2 dBFS, peak 0.251; correlation with the loop 0.998 at 1 s (lag 0) and 0.9995 at 30 s, after the loop wrapped |

Notes on these results:

- **Playtest route.** Playtests run the ordinary packaged player with cooked
  bytecode on the CPU render-input route at 640×480 UI. Each scenario feeds an
  input file at 16 kHz.
- **Offscreen timeline.** The offscreen GPU route, running each package's own
  cooked program, reproduces the win timeline exactly. Its idle run is
  consistently one tick earlier (526/571/616); the harness feeds no input
  stream there.
- **Unroutable pairs.** These are points on cliff tops and ramp lips outside the
  walkable mesh. The count is identical before and after the hoodoo move.

## Rendered review

### Offscreen, real GPU game render path

- **Path.** `review_offscreen.rs` drives `HostedOfflineGameSession` with
  `GpuRenderConsumer` from each package's data and its own cooked program. That
  is the window player's render path without a window.
- **Captures.** 640×480 pt at scale 2 (1280×960 px), plus 960×540 and 420×320
  resizes.
- **Adapter.** Apple M5 Pro, Metal, target `Rgba8UnormSrgb`.
- **Records.** Per-capture records, including render statistics, are in
  `evidence/offscreen.json`.

What I checked at native pixels:

- **Layout.** Every locale at 640×480 and 420×320. English at 960×540.
- **States.** Each HUD state at the moment it applies.
- **Planned route.** Six points along it: sample 1 approach, sample 2 pocket,
  mite contact, sample 3 shelf, sample 4 plateau, lift approach.
- **Idle loss.** Mites closing in, then lost.
- **Reparent hook.** `guests/reparent_probe.js` moves the Won label into the
  result slot at tick 3 and back at tick 6. Re-run on f5:
  - tick 2: bounds 0×0 in the stash;
  - tick 4: 157.9,94 to 482.1,142, drawn;
  - tick 7: back to 0×0.

### Native window

**Route.** The ordinary ci-profile `incant_game_player --package data --window`
from each package. `tools/launch.sh` starts it with `env -i` and a redirected
HOME and TMPDIR, supervises it, and records the real exit status. Captures are
window-only `screencapture -o -x -l` under a separate capture lease with a
budget of 18 (all used).

**Interaction rules.**

- Interaction happens only in bursts, after at least 90 s of user idle
  (stricter than the brief's 20 s).
- Every input command first checks that the player is frontmost.
- After each burst, the previous front app and the pointer are restored.
- 24 bursts ran. 4 yielded at once to user input and were not reused when a
  repeat would change the scene (`review_run.sh` marks walks `noretry`).

**Runs.**

| Run | Package | Steps | Result |
| --- | --- | --- | --- |
| w1 | f3 en playing | start; walk W+A to sample 1 | exit 0, 572 ticks. Found the hoodoo occlusion. |
| w2 | f4 en playing | start; the walk burst yielded | exit 0, 926 ticks |
| w3 | f4 en playing | start; walk (a mite met the robot near the slab); 960×540 and 420×320 | exit 0, 1,332 ticks, 5 surface reconfigurations, 3 UI relayouts. Found the hint wrap. |
| w4 | f4 ar playing | none | **exit 1 at startup** (audio device; finding 9) |
| w5–w7 | f4 zh-Hans, en-XA, lost | start; idle to loss | exit 0 each |
| w8 | f5 en playing | start; walk west, then north into sample 1 | 2 captures, then **exit 1 while unfocused** (finding 9) |
| w9–w11 | f5 ar, zh-Hans, en-XA | start | exit 0 each |
| w12 | f5 en lost | 11.3 s focused idle until the mites win | exit 0, 1,825 ticks |
| w13 | f5 en playing | 420×320, then restore 640×512 | exit 0, 3 surface reconfigurations, 2 UI relayouts |

Every run that printed its report shows:

- Apple M5 Pro, Metal, `on_screen: true`;
- 0 dropped wall ticks;
- 0 surface recreations.

No run wrote a crash report. Each capture's frame, PID and SHA-256 are in
`evidence/window.json`.

**What the native pixels show** (built-in 2x display; a 640×512 pt window
captures at 1280×1024 px):

- **Start frame.** It matches the offscreen render. HUD type is crisp at 2x and
  sits in the deep-sky band.
- **Resize.** At 960×540 the scene reprojects and the HUD relays out to the new
  corners. At 420×320, f4 orphaned "jump"; f5 keeps every English label on one
  line.
- **Sample 1, taken walking north along its x.** This is exactly the corridor
  the old hoodoo blocked, and it is now open. The sample despawns; the static
  HUD still reads 0/4, as documented. A mite reached the robot at the same
  moment.
- **Arabic.** The layout mirrors. The mixed Arabic/Latin controls line resolves
  right-to-left.
- **Chinese and pseudo-locale.** Single lines at 640 pt. The pseudo text is
  complete, with brackets and accents.
- **Lost.** "Surveyor down" over the stopped scene, with empty pips.

**Not exercised in the window:**

- The win route: 2,066 ticks of planned analog input. It is covered on the
  packaged CPU route and the offscreen GPU route.
- Gamepad input, jumping and audible audio.

## Measured counts

**Source art (v13).** 30 files, 10.6 MB.

| Mesh | Primitives | Triangles | Vertices |
| --- | --- | --- | --- |
| canyon | 18 | 58,111 | 88,189 |
| surveyor | 8 | 1,592 | 1,206 |
| mite | 3 | 548 | 1,256 |
| sample core | 2 | 252 | 228 |
| sky dome | 1 | 2,304 | 1,225 |

- **Textures.** 15 PNGs:
  - largest 1024×1024 (canyon floor);
  - 1024×512: cliff albedo and normal, sky dome;
  - 512×512: mesa top albedo and normal;
  - 512×256: environment image;
  - 256×256 and smaller: everything else.
- **Audio.** 48 kHz, 16-bit: one 22.857 s stereo loop and four mono cues.

**Scene.**

- 45 entities, of which 12 are UI nodes and 12 are audio.
- 10 mesh renderers.
- 11 colliders.
- Lights: 9 point lights, 1 directional (4 × 1024 cascades), 1 environment.

**Game render path**, from `GpuRenderConsumer::stats()` offscreen:

| Moment | Model entities | Draw calls (= primitive instances) | Triangles |
| --- | --- | --- | --- |
| Start | 10 | 44 | 64,659 |
| Lift, all samples taken | 6 | 36 | 63,651 |

Diagnostic entities: 0.

**Game UI**, accepted resources:

| Locale | Glyph quads | UI GPU bytes |
| --- | --- | --- |
| en | 78 | 35,076 |
| en-XA | 138 | 54,042 |
| ar | 77 | 44,419 |
| zh-Hans | 40 | 40,040 |

**Package.** Data 34,323,595 bytes:

- models 21.5 MB (canyon 18.7 MB);
- audio 10.3 MB, as the exporter's raw PCM;
- fonts 1.1 MB;
- environment KTX2 0.7 MB.

The ci-profile player binary is about 31.5 MB.

**Memory.** Window-player RSS (`ps`):

- 143,104–155,616 KB when read shortly after launch (w1, w5–w11, w13);
- 86,768–94,192 KB after long unfocused periods (w2, w3, w12).

These are desktop ci-profile figures on an M5 Pro. They are not reference-phone
or shipping-profile evidence.

## Presentation decisions

- **One sedimentary column everywhere.** Every cliff, hoodoo and slab shares the
  same strata at the same world heights. The walkable 1.6 m bench is a pale
  resistant layer, so the two gameplay levels read as geology rather than as
  platforms.
- **Composed for the fixed camera.** The controller keeps the authored camera
  offset (0, 3.3, 7.4) and rotation (−15°, north). Under that constraint:
  - the walls step down toward the open rim, and the lift silhouettes against
    the sky in every frame of the route;
  - the sun is 38° high in the south-south-west, behind the camera's left
    shoulder. Faces toward the camera are lit and shadows run away from it to
    the north-east, which models the ground without silhouetting the actors;
  - tall props stay out of the corridor south of every pickup;
  - the hoodoo move after the window review applies this rule.
- **Distance without fog.** The valley floor is an emissive distance gradient and
  the buttes are pale and low. The environment light is the same picture as the
  visible dome, so fill light and sky agree.
- **HUD.** It uses the fixed theme only:
  - **Placement.** Status reads first (top left), then progress (top right).
    The 40/48 result title is centred 26 pt under the top bar.
  - **Spacing.** A 24 pt column gap and a 12/16 hint keep every English,
    Arabic and Chinese label on one line down to 420 pt. Labels word-wrap only
    for the pseudo-locale at 420 pt. The hint stays in the top-right band of
    deep sky: moved under the objective it would sit on the horizon haze, at
    about 3.8:1 median on the upper mesa.
  - **RTL.** It mirrors the row (`row_reverse`) and column alignment. The Arabic
    controls string starts with Arabic so its paragraph resolves right-to-left.
  - **Pips.** Health pips are the same plural-selected glyph string in every
    locale.
  - **Restraint.** No panels, stripes or glows. One short controls hint is
    the only instruction text.
- **CJK subset.** The Noto Sans SC subset keeps 38 glyphs (12,500 bytes). It is
  regenerated by `art/fonts/subset_sc.py` from a pinned upstream with recorded
  digests and the OFL licence. Latin and digits stay in Inter, earlier in the
  chain.

## Integration contract (for Astra)

All IDs are stable across builds. The full table is in
`samples/core/PRESENTATION.md`.

**Roles** (controller `01CSEN…01` Script props):

| Prop | Value |
| --- | --- |
| `player` | `01CSEN…02` |
| `camera` | `01CSEN…03` |
| `exit` | `01CSEN…04` |
| `navigation` | `01CSEN…05` |
| `enemies` | `01CSEN…20`–`22` |
| `pickups` | `01CSEN…30`–`33` |
| `speed` | 3.4 |
| `enemySpeed` | 1.7 |
| `jumpSpeed` | 6.0 |

**HUD widgets** (string table `01CSST00000000000000000001`):

| Widget | ID | Text |
| --- | --- | --- |
| Health | `01CSWD…04` | `hud.health {health}`, plural pips |
| Objective | `01CSWD…05` | `hud.objective.collect`, then `hud.objective.exit` once all are collected |
| Samples | `01CSWD…07` | `hud.samples {collected} {total}` |
| Won | `01CSWD…11` | parked in the zero-size stash `01CSWD…10` |
| Lost | `01CSWD…12` | parked in the zero-size stash `01CSWD…10` |

**Hook available today.** On the win or loss transition, call
`commands.setParent(WON or LOST, '01CSWD00000000000000000009')` once. This was
exercised on f5 through the real game render path.

**Hook missing.** Changing text needs a label-content command, for example
`commands.setUiText(id, text)`. It would be fed by `localization.localize({table_id,
key, arguments})` and sent only when a value changes. I did not add or simulate
one.

**Audio.**

- Music `01CSEN…42` plays from the start.
- Each cue occurrence has its own silent source, because an ended voice cannot
  be replayed. Start one with `commands.setAudioPlaying(id, true)`:
  - pickups `…43`–`46`, in collection order;
  - hurt `…47`–`49`;
  - won `…50`;
  - lost `…51`.
- Measured: the music in the engine's mixed PCM (see Checks).
- Not exercised: cue playback and audible device output.

## Findings for Astra and root (not edited)

1. **Dev Play cannot run this level.** On the f5 project:
   - `incant_headless native-play core.json --ticks 1` exits 1 with "spatial
     content requires an explicit --camera and native render host".
   - With `--camera 01CSEN00000000000000000003`, it exits 1 with "scene host
     failed: this consumer has no screen-space UI support".
   - RPC `play.run`/`play.start` refused the same way earlier.

   Development checks therefore use the packaged player.
2. **A navmesh query error aborts the whole session.**
   - **Symptom.** `navigation.findPath` failures ("smoothed navigation segment
     left its polygon") surface to TypeScript as "invalid native query", which
     ends the session.
   - **Reproduction.** Saved as `tools/nav_repro.py`/`nav_repro.sh`; a copy of
     the generated example and its output is in
     `~/.codex/incant-evidence/0060/nav/repro/`. With the original settings
     (cell 0.2, height 0.05, tiles 32, climb 0.2), 2 of 3 pairs fail. With the
     chosen settings, 0 fail.
   - **Full sweep, old settings.** 95 errors in 39,300 queries (38,174
     routed).
   - **Recommendation.** Return "no path" (null) instead of aborting.
3. **The packaged route caps runs at 1,250 ticks.**
   - `--package` passes a fixed 48,000 Hz sink (`incant_game_player/src/package.rs:125`).
   - `checkpoint::frame_range` refuses any run over 1,000,000 PCM frames
     ("output frame bound").
   - Together they cap a packaged input-file run at 1,250 ticks, shorter than
     the 2,156-tick win route.
   - The explicit positional form accepts 16 kHz (3,750 ticks). `playtest.py`
     uses that form.
4. **No UI text command.** See the integration contract.
5. **Ended audio voices cannot be replayed.** Hence one source per cue
   occurrence.
6. **Arabic numbers use Latin digits.** `localize` number formatting for `ar`
   yields "0/4". This is an observation, not necessarily a defect.
7. **Audio is packaged as raw PCM.** The 22.9 s loop is 8.8 MB. That will matter
   for mobile budgets.
8. **The fixed game-UI theme has no scrim or outline.** Measured against the
   scene (median background, WCAG):
   - top-bar labels: 4.7–5.8:1;
   - the 40 pt result titles over the pale horizon haze: 3.2–3.4:1, which
     passes the 3:1 large-text minimum but not comfortably.

   Raising the title crowds the objective line. A theme-level outline or
   backdrop would be the robust fix.
9. **Any audio-device change ends the game.** Two window players exited with
   status 1, with no native report and no crash report, while the Mac's audio
   output changed:
   - w4: at startup, `Audio(Backend("CpalError(... Failed to start audio unit
     ... OSStatus: 560227702"))` (`'!dev'`);
   - w8: about 2.5 minutes in, `Audio(Backend("default output device changed"))`.

   cpal 0.18.2 reports a default-device switch as `ErrorKind::DeviceChanged`,
   and `SceneAudio::check_device` (`incant_audio_runtime/src/scene.rs:359`)
   treats every CPAL error as terminal by design. Headphones, Bluetooth or a
   display's audio coming or going would end a shipped game. A recoverable
   stream (reopen on the new default, or continue silent) seems preferable.
10. **The startup window may be sized in physical pixels.** This is an
    observation; I did not trace the cause.
    - On the external 1x display, runs w1–w3 and w9–w13 opened at 640×512 pt
      (640×480 content plus a 32 pt title bar).
    - While that display was unavailable, runs w5–w8 opened on the built-in 2x
      display at 320×272 pt. That is 640×480 physical pixels.

    Each burst set the frame to 640×512 before capturing.

## Known limitations

- **Static HUD text.** The text stays static until a text-setting command
  exists. The `playing`, `collected`, `won` and `lost` HUDs are separate
  authored variants.
- **No audible playback evidence.** Only the engine's mixer output was measured
  (see Checks); nothing was listened to or captured from a device. Cue sources
  stay silent because nothing starts them yet.
- **Fixed follow camera.** The camera has no occlusion handling. The layout
  avoids tall props south of pickups, but a robot hugging the west wall just
  north of the wall-side hoodoo is still partly hidden by it.
- **Hits are not shown in the world.** Health changes appear only through the
  HUD (once bound). The controller has no knockback or flash.
- **Desktop evidence only.** M5 Pro, Metal, ci profile. No phone, shipping
  profile or performance claim.

## Changed paths

- `samples/core/README.md`: a pointer to the presentation layer (the only
  modified file).
- `samples/core/PRESENTATION.md`: build, art direction, layout, HUD IDs,
  integration contract, limitations.
- `samples/core/scene.py`, `hud.py`, `playtest.py`.
- `samples/core/ui/hud.json`, `ui/strings.json`.
- `samples/core/routes/win-input.json`.
- `samples/core/art/`: `__init__`, `actors`, `fontcheck`, `gltf`, `ids`,
  `level`, `mesh`, `music`, `noise`, `pngio`, `props`, `raster`, `sky`,
  `terrain` and `world` (`.py`), plus `fonts/` (`CoreSampleSC-Regular.ttf`,
  `OFL-NotoSansSC.txt`, `subset_sc.py`, `provenance.json`).
- `handoffs/0060-core-sample-presentation/tools/`: lease guard, cargo ledger,
  packaging, offscreen harness and plans, navigation sweep and reproduction,
  route planner and replayer, window control, evidence collector.
- `handoffs/0060-core-sample-presentation/evidence/`,
  `screenshots/`, `result.md`.

Unchanged: `gameplay.ts`, `verify.py` and `tsconfig.json`, plus every engine
crate. Generated art, builds, packages and raw captures stay in
`~/.codex/incant-evidence/0060`.

## Screenshots

`evidence/screenshots.json` lists each file's SHA-256, pixel size and source
capture. Every raw capture, including those not copied, is hashed in
`evidence/window.json` and `evidence/offscreen.json`.

Native window, captured at 2x:

- `screenshots/window-en-start.png`: start, 640×480 content (f5).
- `screenshots/window-en-sample1-south.png`: sample 1 taken from the south,
  the corridor the old hoodoo blocked. The stand is empty and a mite is in
  contact (f5).
- `screenshots/window-en-960x540.png`: resized to 960×540 pt. This one is f4;
  the scene is identical, with a 13 pt hint and a 32 pt gap.
- `screenshots/window-en-420x320.png`: resized to 420×320 pt, single-line HUD
  (f5).
- `screenshots/window-ar-start.png`: Arabic, mirrored HUD (f5).
- `screenshots/window-zh-Hans-start.png`: Simplified Chinese (f5).
- `screenshots/window-en-XA-start.png`: pseudo-locale (f5).
- `screenshots/window-lost.png`: lost after idle; "Surveyor down" with empty
  pips (f5).

Before each fix:

- `screenshots/window-before-fix-sample1-west.png`: f3, the hoodoo fills the
  left of the frame at sample 1.
- `screenshots/window-before-fix-420x320.png`: f4, the controls hint wraps and
  orphans "jump".

Offscreen, the real game render path at 1280×960, along the planned win route
(f5):

- `screenshots/offscreen-route-pocket-s2.png`: tick 500, sample 2 in the
  pocket.
- `screenshots/offscreen-route-shelf-s3.png`: tick 1250, sample 3 on the east
  shelf.
- `screenshots/offscreen-route-plateau-s4.png`: tick 1830, sample 4 on the
  plateau.
- `screenshots/offscreen-collected.png`: the collected HUD state ("Return to
  the lift", 4/4).
- `screenshots/offscreen-won.png`: the won state at the lift ("Samples
  secured").

## Resources, lease and cleanup

**Target lease.** `~/.codex/incant-evidence/0060/lease.json`:

| Field | Value |
| --- | --- |
| Owner | Claude Opus 5.5 (max) |
| Started | 2026-10-12T18:05:31Z |
| Expiry | 2026-10-13T06:05:31Z |
| Marker | `released` |
| Released | 2026-10-12T23:38:22Z (`release.json`) |

- The lease was fresh for this packet. 0059's released runner and markers were
  only read.
- The exclusive warm target `~/.codex/incant-native-target` is kept as cache.
  I never used `incant-animation-target` or `incant-integration-target`.

**Cargo.** 119 guarded runs through `./tools/cargo --locked -j2` (ci profile,
Rust 1.99.0). All exited; the ledger is `commands.jsonl`.

**Capture lease.**

- Started at 20:29:35Z with a budget of 18.
- Extended once, at 23:05:19Z, by three hours, because continuous user activity
  delayed the bursts. The extension is recorded in the lease file.
- Released at 23:35:17Z (`capture-released`) after 18 window-only captures.

**Processes.**

- All 13 window players exited: 11 through the AX close with exit 0, and 2 with
  exit 1 on audio-device changes. Their `launch.sh` supervisors reaped them.
- No 0060 player, capture, harness or helper process remains.
- Cargo builds belonging to other worktrees were left running.

**Tools refuse after release.**

- `cargo.sh`, `launch.sh`, `burst.sh`, `cap.sh`, `offscreen.sh`,
  `nav_repro.sh`, `variants.sh` and `review_run.sh` exit 2 with "0060 lease was
  released; refusing".
- `package.py` raises "0060 lease was released; no build" and creates nothing.

**Temporary state.**

- Every temporary crate example copy is removed, and `git status -- crates` is
  clean.
- The worktree-root `node_modules` symlink is removed. It pointed to the parent
  checkout's `node_modules` and was used for the TypeScript cook and `tsc`.

**Retained outside the repository.** `~/.codex/incant-evidence/0060`: art sets
v9–v13, builds and packages f1–f5, offscreen renders, window runs with exit
records, logs and ledgers.

**Not done.**

- No push, PR, merge, credential read, account change or remote workflow.
- No human-only action is requested by this slice.

## Open questions

- Should the game-UI theme gain a text outline or scrim for titles over bright
  scenes, or is placement over the sky band the intended approach?
- Should the HUD text command take resolved text, or a table key plus arguments
  so the engine re-localizes on locale change?
- Which compressed audio format is intended for packaged music on mobile tiers?
