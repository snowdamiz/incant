# Core Sample presentation

The playable level, original art, localized HUD and music for the Core Sample:
a small survey robot crosses a red-rock canyon at late afternoon, recovers four
glowing rock cores while ember mites close in, and rides the lift on the north
rim. Gameplay is the current native `gameplay.ts`; this layer only authors content
and existing role properties for it. Nothing here claims a device, performance or phase
gate.

## Build

```sh
./tools/cargo build --locked --profile ci -p incant_headless
python3 -B samples/core/scene.py OUT --binary target/ci/incant_headless \
    [--locale en|en-XA|ar|zh-Hans] [--state playing|collected|won|lost] [--art ART_DIR]
```

`scene.py` creates `OUT/project` with `incant init`, generates the source art
(`art/world.py`, about two minutes; `--art` reuses a generated directory after
checking its manifest digests), imports every model, image, font and WAV with
`incant import`, and authors everything else through RPC `command.execute`
transactions: stable asset identities, scene/script/input/string table, the
level, audio, and the HUD. It then undoes and redoes every transaction against
recorded snapshots, saves, reopens in a fresh process, validates, and writes
`OUT/receipt.json` (IDs, digests, transactions, localized text, glyph coverage).
No project file is written directly. `--locale`/`--state` add one undoable
presentation transaction: HUD text in that locale and the chosen HUD state.

Package and play with the ordinary routes:

```sh
python3 tools/export_native.py OUT/project/core.json PKG --mode window --camera 01CSEN00000000000000000003
PKG/player/incant_game_player --package PKG/player/data --window
python3 -B samples/core/playtest.py OUT OUT-playtest --player PKG/player
```

`playtest.py` runs three recordings on the packaged player's CPU route: the win
route in `routes/win-input.json` (planned against this level's real simulation;
all four samples, one hit, the lift at tick 2066), an idle run that loses by
contact, and a live three-sample arrival that dwells inside the exit without winning. It also checks that
every chaser plans a navmesh route and that authoring is unchanged.
Current `incant native-play --camera ... --ui-viewport 640x480` supports the
scene for CPU checks. Whole-session restart uses the ordinary packaged player
or native window owner; headless Play explicitly refuses that request.

## Art direction

One sedimentary column runs through every cliff, hoodoo and slab: the same band
sits at the same world height everywhere, and the walkable 1.6 m bench is a pale
resistant layer. The camera faces north, toward an open rim where the lift
headframe stands against the sky; the canyon walls step down toward that rim so
the frame funnels to the goal. Beyond the rim the land drops to a valley floor
whose emissive distance gradient and flat-topped buttes (210-330 m out) supply
the aerial perspective the renderer has no fog pass for.

Light is a warm sun from the south-south-west (38 degrees) with cool sky fill
from an environment image that is the same picture as the visible sky dome. The
upper sky stays an even deep blue where the HUD sits.

Colour carries meaning and nothing else does: safety yellow marks the
surveyor's own equipment (canister, sample stands, pad rim, rover deck), cyan is
reserved for the samples (emissive veins and a small cyan light each), ember
orange for the mites, and the land keeps the strata reds and ochres.

Readability choices: the surveyor reads from behind (yellow canister, antenna,
hover ring) because the controller never rotates bodies; mites are radially
symmetric for the same reason; every actor model is centred on its collider;
pickups have no child entities because the controller despawns them. The
camera follows 7.4 m behind and never turns, so no tall prop stands in the
corridor south of a pickup; the floor hoodoo near the start sits against the
west wall.

## Layout and flow

```
            far buttes / valley (beyond the rim)
   rim ------------------------------------------- rim
   |  S4 .            [lift]          hoodoo  |   north plateau (1.6 m)
   |west ramp      centre ramp            S3  |   east shelf
   |   .  hoodoo   promontories   S2 pocket   |
   |  wash      hoodoo        slab  east ramp |   canyon floor
   |  S1  boulders          cairns            |
   | hoodoo start          camp (tent, rover) |
```

Three sand ramps join the floor to the mesa; there are no off-mesh links. The
low slab (0.44 m) can be jumped by the player but is an obstacle for chasers.
Speeds are 3.4 m/s (player), 1.7 m/s (mites) and a 1 m jump.

The navigation mesh is generated from the three collision entities only
(ground, walls, obstacles). Its settings were chosen with a sweep of about 39k
start/end pairs using the controller's own query (snap 2 m, 2048-visit budget):
no errors, worst case 1155 visits and 44 route points. Finer settings produced
the engine error `smoothed navigation segment left its polygon` on ramps; see
the handoff result for the reproduction.

## HUD

Plain type over the scene in the engine's fixed game-UI theme: light text, no
panels, and the engine's dark label halo around every glyph (1/16 em, 1 px for
the 12-17 px text, 1.4 px for 22 px, 2 px for the 32 px title). The halo keeps
text legible where it crosses bright haze, sand or the headframe, so placement
serves composition rather than contrast. The engine prepares each halo with the
glyph rasters when the HUD text changes and draws it as one more glyph, so it
adds no per-frame dilation work. Status top-left, progress top-right.
Fonts chain Inter, Noto Sans Arabic and a Noto Sans SC subset (`art/fonts`, OFL,
with recipe and provenance).

The Arabic health message includes U+200F right-to-left marks around its plural
result. Its dot symbols have no strong reading direction by themselves; keep
these marks so filled health starts at the right and damage empties the left.
English and Chinese keep their left-to-right health order.

When a run ends, the title and a restart prompt appear together, centred in the
upper third of the screen above the surveyor: "Samples secured" or "Surveyor
down", then "Press R or Start to play again". R and gamepad Start only restart
after a result, so the prompt appears exactly when they work, and names both.
The top bar never moves; after a win the objective clears. The legend names
only what works during play: "WASD move · Space jump". It stays keyboard-only,
like the move and jump bindings it describes.

Each label has 2 px of padding left and right and 1 px top and bottom, room for
its halo; the root's padding and the gaps are reduced by the same amounts, so
text keeps its 22 px side and 16 px top inset. When status and progress do not
fit on one row, the top bar wraps progress onto a second row, still
end-aligned, so no label wraps inside its column.

| Index | ID | Role | Content |
|---|---|---|---|
| 1 | `01CSWD00000000000000000001` | root | viewport |
| 2 | `01CSWD00000000000000000002` | container | top bar (wrapping row; `row_reverse` for RTL) |
| 3 | `01CSWD00000000000000000003` | container | status column |
| 4 | `01CSWD00000000000000000004` | label | health pips, `hud.health {health}` |
| 5 | `01CSWD00000000000000000005` | label | objective, `hud.objective.collect` / `.exit` |
| 6 | `01CSWD00000000000000000006` | container | progress column |
| 7 | `01CSWD00000000000000000007` | label | `hud.samples {collected} {total}` |
| 8 | `01CSWD00000000000000000008` | label | `hud.controls` (move, jump) |
| 9 | `01CSWD00000000000000000009` | container | result box: 40% of the content height under the top bar, contents centred |
| 10 | `01CSWD00000000000000000010` | container | zero-size stash (clips its children) |
| 11 | `01CSWD00000000000000000011` | label | `hud.won`, parked in 10 |
| 12 | `01CSWD00000000000000000012` | label | `hud.lost`, parked in 10 |
| 13 | `01CSWD00000000000000000013` | label | `hud.prompt`, bound as `prompt`, parked in 10 |

`ui/strings.json` is the string table (`01CSST00000000000000000001`, source
`en`, plus `ar` and `zh-Hans`); `en-XA` is the engine's pseudo-locale. Display
text is never typed by hand: the builder resolves every key through the engine's
own `localization.localize()` in a disposable probe project and refuses text the
font chain cannot cover. `ui/hud.json` declares the widgets and the four
presentation states.

## Integration contract

Widget, entity and asset IDs are stable across builds. `bindings.py` connects
the existing controller roles, without changing widget style or authored sound:

- Win/loss: `setParent(WON or LOST, RESULT)` shows the existing localized title
  once, on transition, and `setParent(PROMPT, RESULT)` shows the prompt with it.
  The other title stays in the authored inactive container; a fresh session
  parks the title and prompt again.
- Audio: `restartAudio` plays the existing first pickup (`01CSEN...43`) and
  hurt (`...47`) source on each occurrence, including after the voice ends.
  Won (`...50`) and lost (`...51`) play once at transition; music (`...42`)
  stops. Historical extra pickup/hurt sources stay unchanged and silent.
- Text: `setUiText` receives the existing `localization.localize()` results for
  health, samples and objective (empty after a win). The controller updates
  changed values and locale changes, and localizes both result titles and the
  prompt.
- Restart: R or gamepad Start while won/lost requests a complete fresh session.
  World, script, audio, UI and held input reset together; it is not a save load.
  These actions are ignored while playing, which is why only the result-state
  prompt names them.

`verify_bindings.py prepare BUILD OUT --binary ...` copies an authored build and
connects its roles through one reversible public transaction. It checks the real
native full routes, separate test-only HUD readback and paired cue PCM controls.
`verify_bindings.py ship OUT SHIP` exports the production scene, deletes its own
source/maps and checks exact baseline/restarted PCM plus shaped text. Execution
results must be retained before calling the new bindings verified.

## Limitations

- The four `--state` choices remain authored presentation snapshots; gameplay
  starts from its normal fresh state. They do not pre-complete gameplay.
- Handoff 0065 measured result titles over the horizon haze at 2.7-3.2:1 and a
  wrapped second row at 2.6-3.9:1 with the earlier one-pixel shadow. This
  layout and the engine's label halo replace both; their rendered contrast and
  readability, at normal and narrow sizes and in every locale, are pending the
  granted handoff 0069 review. Nothing here claims them yet.
- With 1 px of vertical label padding, a glyph whose ink meets its line box
  (a deep descender) can lose the outer part of its halo at the label's border
  box. The rendered review must check this in every locale.
- The theme stays fixed: no colour, panels or per-widget text styles.
- The legend is keyboard-only; the game has no input-device-aware copy.
- In the pseudo-locale at the narrowest sizes, the title wraps and the result
  box starts under a two-row top bar, so the prompt can reach the surveyor.
- The original 0060 offscreen helper source hash remains unresolved. New
  correctly bound source evidence does not retroactively change that claim.
- Desktop (Apple M5 Pro, Metal, ci profile) evidence only; no phone, shipping
  performance or phase-gate claim.
