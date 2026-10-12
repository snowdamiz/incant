# Pearl Cove

The Planar Sample's presentation: a small low-tide cove. A beachcomber in a red
raincoat collects three pearls and reaches the moored rowboat. Sea urchins sting.
All art is original and generated from checked source in `art/`.

## Build

```sh
python3 -B samples/planar/scene.py <new-output> --binary <incant_headless> --play
```

`scene.py` writes the art sheets and copies the HUD font (Inter, SIL OFL 1.1,
with its license) into the project. It imports both with the public importer
and authors everything else in four RPC command transactions:

1. controller and input;
2. terrain and camera;
3. actors;
4. HUD: string table, UI nodes and the controller's presentation bindings.

It checks Undo/Redo across all four, save and journal reopen. `--play` runs
seven native-play scenarios on this layout through `playtest.py`, each with a
640 x 360 UI viewport: WASD, arrow-key, left-stick and d-pad wins, which must
match exactly, an urchin loss, the locked boat, and stops at the shore and the
cliff. It also checks that Play leaves the authored files unchanged.
`python3 -B -m art.build <dir>` from this directory writes the sheets alone.
`crates/incant_game_player/examples/hud_frames.rs` renders the live HUD
offscreen through the GPU UI consumer for review. The handoff 0061 tools drive
it.

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
Rounded rock corners reveal the ground layer beneath. The top two cliff rows use
`rock/quiet/…` frames: the same moss without light tufts, as ground for the HUD.

## Sprites and roles

| Entity | ID | At (m) | Sprite (px) | Pivot | Collider | Flipbook |
|---|---|---|---|---|---|---|
| Beachcomber | `01PSEN00000000000000000001` | 6.5, 4.5 | 16 x 16 | 0.5, 0.25 | circle 0.30, dynamic | paused: default, stung, cheering |
| Pearls | `01PSEN0000000000000000000B`, `...0C`, `...0D` | 3.5, 8.5; 16.5, 8.5; 17.5, 4.5 | 16 x 16 | centre | sensor 0.35 | twinkle, 8 fps, staggered |
| Sea urchins | `01PSEN0000000000000000000N`, `...0P`, `...0Q` | 5.5, 8.5; 14.5, 7.5; 13.5, 3.5 | 16 x 16 | centre | sensor 0.40 | spines sway, 2 fps |
| Rowboat exit | `01PSEN00000000000000000005` | 10.5, 2.0 | 32 x 16 | 0.5, 1.0 | sensor 0.40 | paused: lantern dark, lit |
| Water glints | `01PSEN0000000000000000000Z`, `...10` to `...12` | on the sea | 8 x 8 | centre | none | 6 to 7 fps |

The camera is `01PSEN00000000000000000003` at (10, 6, 20). The controller,
`01PSEN00000000000000000004`, has only its `Script`: `player`, `exit`,
`pickups`, `hazards` and `speed: 3`, plus the HUD transaction's `flipbooks` and
`hud` bindings. It has no Transform, so authoring previews draw no placeholder
for it. Physics entities are unparented at unit scale, and
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

## HUD and state feedback

`gameplay.ts` drives every element below through its optional bindings; nothing
on screen is a static label standing in for live state.

| Controller state | Status line | Beachcomber | Boat lantern |
|---|---|---|---|
| playing, k of N pearls | `hud.collecting` (k, N) | default, frame 0 | dark, frame 0 |
| playing, all N pearls | `hud.ready` | default, frame 0 | lit, frame 1 |
| won | `hud.won` (N) | cheering, frame 2 | lit, frame 1 |
| lost | `hud.lost` | stung, frame 1 | lit only if all were collected |

`flipbooks` binds the beachcomber and the boat, whose paused flipbooks hold
those frames. `hud` binds string table `01PSST00000000000000000001` and two
labels under root `01PSHD00000000000000000001`:

| ID | Node | Design |
|---|---|---|
| `01PSHD00000000000000000002` | status label | top centre, Inter 16 on a 20-point line, 6-point inset |
| `01PSHD00000000000000000003` | prompt label | bottom centre, same type; always blank (see Restart) |

Engine labels are light text with a hard one-pixel shadow and no surface. The
status line therefore sits on dark ground: the top letterbox bar on taller
windows, and otherwise the top cliff band. The band's two rows use a quiet moss
variant without light tufts, so thin strokes stay clean. At 16:9 the alcove
faces begin 28 points below the top edge, and the 6-point inset and 20-point
line keep even the longest locale above them. There are no panels, borders or
glows. Both labels are authored blank, so the first visible text is already
the controller's localized value. On very wide displays such as a landscape
phone, the line straddles the thin top bar and the moss. Both are dark, so it
stays legible; UI layout cannot see the camera's letterbox rect.

| Key | en | es | fr | de |
|---|---|---|---|---|
| `hud.collecting` | Pearls {collected} of {total} | Perlas: {collected} de {total} | Perles : {collected} sur {total} | Perlen: {collected} von {total} |
| `hud.ready` | All pearls found. Board the boat. | Tienes todas las perlas. Sube al bote. | Toutes les perles sont là. Montez dans la barque. | Alle Perlen gefunden. Ab ins Boot! |
| `hud.won` | Sailing home with # pearls | Rumbo a casa con # perlas | Retour au port avec # perles | Mit # Perlen nach Hause |
| `hud.lost` | Stung by a sea urchin | Te picó un erizo de mar | Piqué par un oursin | Von einem Seeigel gestochen |

`hud.won` uses ICU plural forms (`one`/`other`), so it stays grammatical for any
pearl count. French uses a no-break space before the colon. The engine's
pseudo-localization mode and fallback chains apply unchanged.

**Results.** The optional `results` binding is deliberately unbound. That
binding only reparents its labels and never localizes them, so any authored
headline would stay English in every other locale and in pseudo mode. It would
also repeat the status line. To add a terminal headline, have the controller
localize the result labels on each locale refresh, as the Core Sample does,
then bind `results`.

**Restart.** Whole-session restart does not exist, so no restart control is
advertised: the prompt stays blank. A restart needs:
- a `restart` button action (KeyR, Enter, gamepad Start and South), accepted
  only when won or lost;
- a session reset that rebuilds the world from the cooked scene at the next
  sync point, clears `context.state`, respawns pearls and resets the flipbooks.

## Budget

Measured from the cooked review package (ci-profile build, macOS). These are
desktop measurements, not reference-phone results.

| Item | Value |
|---|---|
| Atlas | one 126 x 244 RGBA8 sRGB page, 8 mips, 93 frames (78 tile, 15 sprite) |
| Texture data | 164,020 B KTX2 per color role (2 roles) |
| Font | Inter Regular: 310,252 B source, 310,300 B cooked (whole face, no subsetting) |
| Source sheets | 3,517 B tiles PNG, 1,000 B sprites PNG |
| Scene | 455 tile cells in 5 layers, 12 sprite entities, 79 flipbook frames, 3 UI nodes |
| HUD per frame | 10 to 28 glyph quads (en); 2.6 to 6.0 KB of UI GPU data |
| Cooked data | 15,854 B `world.scene`, 663,958 B in total |

The art is pixel art at 16 px per metre. Sprites and tiles use nearest
sampling, and every color comes from one atlas texture. The font is the largest
asset; a subset limited to the four locales and pseudo-localization would shrink
it, but the engine has no subsetting step yet.
