# Pearl Cove

The Planar Sample's presentation: a small low-tide cove. A beachcomber in a red
raincoat collects three pearls and reaches the moored rowboat. Sea urchins sting.
All art is original and generated from checked source in `art/`.

## Build

```sh
python3 -B samples/planar/scene.py <new-output> --binary <incant_headless> --play
incant_headless screenshot <new-output>/project/planar.json out.png \
  --camera 01PSEN00000000000000000003 --width 1280 --height 720
```

`scene.py` writes the art sheets, imports `art/cove.atlas.json` with the public
importer and authors the project in three RPC command transactions: controller
and input, terrain and camera, actors. It checks Undo/Redo, save and journal
reopen. `--play` runs seven native-play scenarios on this layout through
`playtest.py`: WASD, arrow-key, left-stick and d-pad wins, which must match
exactly, an urchin loss, the locked boat, and stops at the shore and the cliff. It also checks
that Play leaves the authored files unchanged. `python3 -B -m art.build <dir>`
from this directory writes the sheets alone.

## Visual rules

- Walkable sand has the highest values. Moss-topped basalt, boulders, sea and
  the tide pool are darker and outlined, so blocked space reads at a glance.
- Only pearls are white, only urchins are dark purple and spiky, and only the
  beachcomber is saturated red. The boat shares that red, marking it as the goal.
- Light comes from the upper left. Sprites carry their own contact shadows.
  Rock shadows and wet sand margins are a translucent tile layer.
- Every color keeps each channel at or below 231 (linear 0.8), where the
  preview tone map is the identity, so rendered pixels equal the palette. The
  pearl sparkle is the one deliberate exception.
- The camera is `pixel_perfect`: 16 px per metre and a 320 x 180 logical frame
  at a fixed position. The engine scales it by whole pixels and letterboxes with
  its backdrop. The cove fits one frame, so every goal stays in view and the
  controller's optional follow camera is unused.

| Output | Integer scale |
|---|---|
| 1280 x 720 or 1920 x 1080 | 4x or 6x, exact fill |
| 640 x 480 pt window at 2x (1280 x 960) | 4x, bars above and below |
| iPhone 13 landscape (2532 x 1170) | 6x, computed |
| Pixel 6 landscape (2400 x 1080) | 6x, computed |
| iPhone 13 portrait (1170 x 2532) | 3x, computed, heavily letterboxed |

## Tiles

One tilemap entity (`01PSEN00000000000000000002`, 20 x 12 one-metre cells) uses
one atlas. Each tile depends only on its kind, its eight neighbours (reduced by
the 47-variant blob rule) and a variant index, so the frames form a reusable
autotile set. Frame IDs are `01PSFR` plus 20 characters of SHA-256 of the
frame name (for example `rock/moss/e-n-ne-nw-w/1`), so they stay stable while
frames are added.

| Layer | ID | Order | Solid | Content |
|---|---|---|---|---|
| ground | `01PSTM00000000000000000001` | 0 | no | sand, ripples, pebbles, kelp |
| shade | `01PSTM00000000000000000002` | 1 | no | wet margins, cast shadows |
| water | `01PSTM00000000000000000003` | 2 | yes | sea, tide pool, water under sea rocks |
| rock | `01PSTM00000000000000000004` | 3 | yes | moss cliffs, stone boulders |
| pier | `01PSTM00000000000000000005` | 4 | no | planks over a gap in the water layer |

A cliff face is drawn inside its solid cell, so the collider and the art agree.
Rounded rock corners reveal the ground layer beneath.

## Sprites and roles

| Entity | ID | At (m) | Sprite (px) | Pivot | Collider | Flipbook |
|---|---|---|---|---|---|---|
| Beachcomber | `01PSEN00000000000000000001` | 6.5, 4.5 | 16 x 16 | 0.5, 0.25 | circle 0.30, dynamic | paused: default, stung, cheering |
| Pearls | `01PSEN0000000000000000000B`, `...0C`, `...0D` | 3.5, 8.5; 16.5, 8.5; 17.5, 4.5 | 16 x 16 | centre | sensor 0.35 | twinkle, 8 fps, staggered |
| Sea urchins | `01PSEN0000000000000000000N`, `...0P`, `...0Q` | 5.5, 8.5; 14.5, 7.5; 13.5, 3.5 | 16 x 16 | centre | sensor 0.40 | spines sway, 2 fps |
| Rowboat exit | `01PSEN00000000000000000005` | 10.5, 2.0 | 32 x 16 | 0.5, 1.0 | sensor 0.40 | paused: lantern dark, lit |
| Water glints | `01PSEN0000000000000000000Z`, `...10` to `...12` | on the sea | 8 x 8 | centre | none | 6 to 7 fps |

The camera is `01PSEN00000000000000000003` at (10, 6, 20). The controller,
`01PSEN00000000000000000004`, has only its `Script` with `player`, `exit`,
`pickups`, `hazards` and `speed: 3`. It has no Transform, so authoring previews
draw no placeholder for it. Physics entities are unparented at unit scale, and
pearls have no children. The exit sensor sits on the pier's seaward edge, so the
boat floats in the next row of solid water.

Map (`art/cove.py`): `#` cliff, `b` boulder, `~` sea, `p` tide pool, `=` pier,
`.` sand (`:` pebbles, `;` kelp). The win route is 37 m, about 13 s at 3 m/s.

```
####################
####################
##....;.######....##
#........####....:.#
##.................#
#..bb...pp.....bb..#
#..bb....p.....bb.##
#...:.............;#
##.....~~~=~~....~~#
#~~~..~~~~=~~~~~~~~#
~~~~~~~~~~~~~~~~~~~~
~~~~~~~~~~~~~~~~~~~~
```

`move` is an axis2 action: WASD, the arrow keys, the left stick (inverted Y,
so up is positive) and the d-pad.

## Integration contract (not yet implemented)

The controller currently changes only velocity and pickups. These hooks add
state feedback. Apply each one only when its state changes.

| Controller state | Beachcomber frame | Boat frame | Status text | Prompt |
|---|---|---|---|---|
| playing, k < N pearls | 0 | 0 | `hud.collecting` (k, N) | empty |
| playing, all N pearls | 0 | 1 | `hud.ready` | empty |
| won | 2 | 1 | `hud.won` (N) | `hud.restart` |
| lost | 1 | 1 if all collected, else 0 | `hud.lost` | `hud.restart` |

**State frames** need no new engine API: use
`context.commands.controlFlipbook(id, 'seek', frame)` with the frames above. The
native frame review exercised these seeks.

**HUD.** No script API mutates UI widgets today, and a scene with UI nodes
cannot run in headless native-play. The HUD is therefore not authored. Bind it
with these stable IDs once a bounded text-binding command exists:

| ID | Node | Layout |
|---|---|---|
| `01PSUI00000000000000000001` | Root | padding 16/12/16/12, column, space between |
| `01PSUI00000000000000000002` | Label: status | top left, 18 px text, 24 px line |
| `01PSUI00000000000000000003` | Container | bottom row, centred |
| `01PSUI00000000000000000004` | Label: prompt | in the bottom row, 16 px text, 22 px line |

The status line sits over the dark cliff band and the prompt over the sea, so
neither needs a panel, border or glow. Strings for table
`01PSST00000000000000000001` (`en`):

| Key | Text |
|---|---|
| `hud.collecting` | Pearls {collected} of {total} |
| `hud.ready` | All pearls found. Board the boat. |
| `hud.won` | Sailing home with {total} pearls |
| `hud.lost` | Stung by a sea urchin |
| `hud.restart` | Press R or Start to play again |

**Restart.** There is no restart yet. Add a `restart` button action (KeyR,
Enter, gamepad Start and South), accepted only when won or lost. It needs a
session reset that rebuilds the disposable world from the cooked scene at the
next sync point. The reset clears `context.state`, respawns the despawned pearls
and resets the flipbooks. Until it exists, `hud.restart` stays empty.

## Budget

Measured from the cooked review package (ci-profile player, macOS). These are
desktop measurements, not reference-phone results.

| Item | Value |
|---|---|
| Atlas | one 200 x 144 RGBA8 sRGB page, 8 mips, 88 frames (73 tile, 15 sprite) |
| Texture data | 153,972 B KTX2 per color role (2 roles) |
| Source sheets | 3,317 B tiles PNG, 997 B sprites PNG |
| Scene | 455 tile cells in 5 layers, 12 sprite entities, 79 flipbook frames |
| Cooked data | 14,136 B `world.scene`, 331,063 B in total |

The art is pixel art at 16 px per metre. Sprites and tiles use nearest
sampling, and every color comes from one atlas texture.
