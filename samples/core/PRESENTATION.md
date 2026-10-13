# Core Sample presentation

The playable level, original art, localized HUD and music for the Core Sample:
a small survey robot crosses a red-rock canyon at late afternoon, recovers four
glowing rock cores while ember mites close in, and rides the lift on the north
rim. Gameplay is the unmodified `gameplay.ts`; this layer only authors content
and role properties for it. Nothing here claims a device, performance or phase
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
contact, and a sample-less run that never opens the exit. It also checks that
every chaser plans a navmesh route and that authoring is unchanged.
`incant native-play` and RPC Play refuse scenes with screen-space UI, so the
playtests use the player.

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

Plain type over the scene in the engine's fixed game-UI theme (light text, hard
one-pixel shadow, no panels). Status top-left, progress top-right, the result
title centred under the top bar. Fonts chain Inter, Noto Sans Arabic and a 38
glyph Noto Sans SC subset (`art/fonts`, OFL, with recipe and provenance).

| Index | ID | Role | Content |
|---|---|---|---|
| 1 | `01CSWD00000000000000000001` | root | viewport |
| 2 | `01CSWD00000000000000000002` | container | top bar (row; `row_reverse` for RTL) |
| 3 | `01CSWD00000000000000000003` | container | status column |
| 4 | `01CSWD00000000000000000004` | label | health pips, `hud.health {health}` |
| 5 | `01CSWD00000000000000000005` | label | objective, `hud.objective.collect` / `.exit` |
| 6 | `01CSWD00000000000000000006` | container | progress column |
| 7 | `01CSWD00000000000000000007` | label | `hud.samples {collected} {total}` |
| 8 | `01CSWD00000000000000000008` | label | `hud.controls` |
| 9 | `01CSWD00000000000000000009` | container | result slot |
| 10 | `01CSWD00000000000000000010` | container | zero-size stash (clips its children) |
| 11 | `01CSWD00000000000000000011` | label | `hud.won`, parked in 10 |
| 12 | `01CSWD00000000000000000012` | label | `hud.lost`, parked in 10 |

`ui/strings.json` is the string table (`01CSST00000000000000000001`, source
`en`, plus `ar` and `zh-Hans`); `en-XA` is the engine's pseudo-locale. Display
text is never typed by hand: the builder resolves every key through the engine's
own `localization.localize()` in a disposable probe project and refuses text the
font chain cannot cover. `ui/hud.json` declares the widgets and the four
presentation states.

## Integration contract

Widget, entity and asset IDs are stable across builds. The current native SDK
can already do the structural part:

- Win/loss: `commands.setParent(WON or LOST, RESULT)` once, on the transition.
  Exercised offscreen through the real game render path (show and re-stash).
- Audio: start one cue per occurrence with `commands.setAudioPlaying(id, true)`;
  ended voices do not replay, so each occurrence has its own silent source.
  Pickup cues `01CSEN...43`-`46` (in collection order), hurt `...47`-`49`,
  won `...50`, lost `...51`. Music `...42` plays from the start.

Text changes (health pips, sample count, objective) need an engine command
that sets a label's content, for example `setUiText(id, text)`, fed by
`localization.localize()` with the keys above, only when a value changes. No
such command exists today; this layer authors the `playing` texts and does not
claim live text binding.

## Limitations

- Static HUD text until a text-setting command exists; the presentation states
  are authored transactions, not live bindings.
- The game UI theme is fixed (no colour, panels or per-widget styles), so the
  HUD relies on placement over the deep sky band for contrast.
- `incant native-play` cannot run this level (screen-space UI); development
  checks use the packaged player.
- Desktop (Apple M5 Pro, Metal, ci profile) evidence only; no phone, shipping
  performance or phase-gate claim.
