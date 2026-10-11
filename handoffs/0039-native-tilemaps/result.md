# 0039 retained tilemap appearance and atlas placement review

## GPU slot

**The last GPU capture process ended at 2026-10-10 23:21:57 −04:00 (2026-10-11 03:21:57
UTC).** That was the source-free capture of state `s8`. No `incant_headless` process has
run since then and none is running, so the device slot can be released. Everything after
that time (analysis, figures, evidence, this report) ran on the CPU and made no engine
calls.

## Verdict

**Accepted, scoped to tile-specific appearance, and conditional on the shared renderer's
documented limitations. I found no tile-specific defect.** I ran 86 headless authoring
captures of the frozen executable on one Apple M5 Pro, under the conditions below.

- **Tiles match the ordinary-sprite control exactly.** The tilemap project is
  byte-identical to a control that draws the same placements as 226 ordinary sprites:
  10/10 views in s0 and 6/6 in s1. Each control sprite is one cell, made a child of the
  map's Transform at (x·cw, y·ch) with pivot (0, 0). Its entity ID encodes
  (map ID, layer ID, y, x), so ordinary sprite ordering reproduces the documented tile
  order.
- **The independent model finds 0 unexplained pixels.** I compared 1,958,400 pixels in
  11 views against a CPU model built from the written contract and the original source
  images (not the cooked atlas or engine code):
  - 1,956,122 match exactly;
  - 2,278 lie within 0.01 px of an edge or texel tie, and 2,262 of those resolve as the
    documented rule predicts;
  - the other 16 are the shared tie limitation (see "Ties").
- **Grid contract.** Cell (0,0) is at the lower left and rows are stored bottom to top.
  Holes show what lies beneath. Every frame, whatever its size, is stretched across its
  whole cell with image rows top-down. Neither a frame's packed origin nor any pivot
  offsets the grid.
- **Painter order** is (layer, order, entity ID, layer ID, y, x):
  - two layers with equal (layer, order) sort by stable layer ID, not document order;
  - ordinary sprites interleave between maps and between layers of one map;
  - mixed atlas and ordinary textures keep their order.
- **Inherited transforms** apply once. I tested translation, 30° parent rotation, x
  reflection on the map, y reflection through a parent, nonuniform scale with rotation,
  shear from a nonuniformly scaled parent, and a parent with scale 2 plus a child
  translation. There are no seams, cracks, clipped frame edges or sorting changes in any
  view, including partly clipped maps.
- **Atlas repack.** An accepted v2 repack moved all 13 frames (page 42×68 → 94×56,
  padding 1 → 2) and changed no rendered pixel (10/10). The cooked pages are exact
  texel for texel.
- **Rejected reimport.** Removing a referenced frame is rejected. Project, journal,
  fingerprint and preview are unchanged (3/3).
- **History.** Undo and Redo reproduce exact frames, both for the command edit (7/7 each)
  and for the repack import (3/3 each).
- **Repeatability.** Fresh-process repeats are identical (10/10), and source-free
  captures equal the cached baseline (10/10).

**Not claimed:** native game execution, the retained native path, any performance,
phone, provider, interactive-player or phase-gate result, arbitrary zoom, or acceptance
of the shared renderer. The authoring screenshot command feeds the canonical renderer
through its authoring adapter. These captures are not native game execution.

## Director feedback and packet

- **A new isolated packet.** The worktree held only Astra's `brief.md` in this packet. No
  earlier handoff file was changed. I imported 0032's PNG reader/writer and 3×5 glyph
  shapes read-only and recorded their hashes. No bytecode was written there.
- **Bounds kept:**
  - only the supplied executable, hash-checked before and after every call;
  - no Cargo, Rust, shader, schema, SDK, editor or production-source edits;
  - no native application, native UI or system-wide input;
  - no Play, accounts, credentials, publishing or merging.
- **GPU gate:** `artifacts/tools/gpu-review-ready` was present before the first
  screenshot. It reads: "Root and integration coordinator released the GPU slot after
  numerical tests. Headless-only review authorized."

## Model, effort and provenance

- **Model:** Claude Opus 5.5, model ID `claude-opus-5-5`, as the session reports it.
- **Effort:** Max, the session's reasoning-effort setting.
- **Transport:** the prompt arrived in the exact template of `tools/handoff/main.py`
  (lines 235–242). That runner sends a prompt only after `configure_model_and_effort`
  confirms "Opus 5.5" and effort "max" over ACP (`tools/acp/config.json`:
  `claude-opus-5-5`, `max`). I cannot inspect that handshake from inside the session.
  No other model or effort was used.

| Item | Value |
| --- | --- |
| Worktree HEAD | `44671f77b8139376fc8fec76dad1bac6626b903f`; differs from `b2d0bad3580dccdccc4a3c7733bcbf872bad8bd6` only by this packet's `brief.md` |
| Source candidate | `b2d0bad` (tree `cfb7ba8045a93c2d93569cbabd5ab61416bb2d89`), the reviewed implementation `420f866cef252308f0e82014b3ab8abf63883f7d` plus main64 integration |
| Renderer source | `git diff 420f866 b2d0bad` is empty for `incant_render`, `incant_render_data`, `incant_render_authoring`, `incant_render_native` and `incant_2d` |
| Binary | `artifacts/tools/incant_headless`, 39,258,752 bytes, sha256 `0445adae515a2c23b4d0d4924db00507023817ea3571c7164b6c0c40d8219b80`, equal to `binary.json` (release, rust 1.99.0, built from `b2d0bad`) |
| Hash checks | all 132 engine calls matched `binary.json` before and after (`evidence/engine-calls.jsonl`) |
| Device | adapter "Apple M5 Pro", reported by every screenshot; macOS 26.6 (25G72), arm64 |
| Backend | Metal, per `binary.json` ("Apple M5 Pro / Metal") on this macOS host; the screenshot reply names the adapter but not the backend |
| Tooling | Python 3.14.8, standard library only; no Node or Cargo |

## What I built

All project state was built and changed only through `init`, `import`, `rpc`
(`command.execute`, `history.undo`, `history.redo`, `project.save`, `project.read`,
`history.read`), `validate` and `screenshot`. My tools never write a project or journal
file. Source PNGs and `.atlas.json` descriptors are copied in as authoring inputs.

### Original art (`helpers/tile_art.py`, committed in `art/`)

All art is drawn from code for this packet and regenerates byte for byte.

- **Opaque frames A–H.** Each has unequal dimensions: 8×8, 12×10, 10×14, 16×16, 6×9,
  20×12, 7×11 and 13×7. Each has the same asymmetric 1-texel ring:
  - sides: red top, blue bottom, amber left, green right;
  - corners: light grey, cyan, pink and near-black.

  Each also has a frame-unique interior and a 3×5 glyph naming the frame. A wrong frame,
  flip, quarter turn, row-order error or one-texel shift is therefore visible.
- **T** (12×12): a transparent outer ring around a coloured inner ring.
- **X** (16×16): a mostly transparent diagonal cross, with a cyan block at the top left.
- **K** (4×4): a checker with a marked top-left texel.
- **W** (24×8): a ruler whose every column has a distinct colour, with a red top row and
  a blue bottom row.
- **Hidden RGB and guards.** Every zero-alpha texel hides magenta RGB, and an opaque
  magenta guard surrounds every source region in `tiles.png`. Magenta in a frame would
  therefore mean bleeding or sampling outside the frame.
- **Ballast:** 30×6 for v1 and 70×22 for v2, used to force relocation.
- **Plates and badge:** `plates.png` (board backgrounds) and `badge.png` (an ordinary
  texture for interleaved sprites).
- **All colours stay within sRGB 0–234,** the display's measured identity range (0032).

| Source | sha256 |
| --- | --- |
| `tiles.png` (96×38) | `d73c0724eddbeaa116e1fce03956a45b0a2b986e7c8a918d5c5303a90cc12191` |
| `ballast-v1.png`, `ballast-v2.png` | `ec992abfbf98ac6a2e94e289fdf964ea4db10da50369361bd7433df0730c53cd`, `cf6fd5b3163a1da0682bdf9a7664c6814cfb4100df373971e68a18f27f5d7c0a` |
| `plates.png`, `badge.png` | `65b34ef0ff8db38c60302bd637eb9a54dc3b62a9a539966f43d84b6fdb6bc5e1`, `2d0a32a6cdd1100e9f8e1837de6e8d93a9751e04ebccc44c1cd4b13ee37fe983` |
| `tiles-v1.atlas.json` (padding 1, max 256×256) | `6ca00f8d41fa045be3f9e615f2806e0496deebc53147996ded17f174eaf04a66` |
| `tiles-v2.atlas.json` (padding 2) | `05f3d9cbc53a9ae60460f82ff2a2c00f6a89cafe7704f4bb3f7fea788934a728` |
| `tiles-fail-missing.atlas.json` (v2 without frame C) | `5019f8fcd35b070eb7f9f79554f7662f3d50f8cec39d9700ed258a9e1d68c743` |
| Reused read-only: 0032 `pngio.py`; `sprite_art.py` for `FONT` only | `5b73bef8…9e86`; `455d286e…eff1` |

Frame IDs are `01JA39FR000000000000000001` to `…00C` (A to W) and `…033` (ballast).

### Scene, projects and views (`helpers/tile_scene.py`)

At 16 px/m, each board has a plate on layer −100 and its own cameras.

| Board | Content |
| --- | --- |
| A, grid and identity | A1 6×4 at 1 m with three holes; its top row "ABCDEF" differs from its bottom row "CBAHGW", so a flipped row order shows. A2 4×3 non-square 1.25×0.75 m cells. A3 6×1 offset by half a pixel on both axes, so every cell edge passes through pixel centres. A4 8×7 non-dyadic 0.7×0.6 m cells. A5 6×3 minified 0.5 m cells. A6 3×1 magnified 2 m cells. A7 12×1 strip that the 2.5× view clips |
| B, layers and order | B1 6×4 with five layers in deliberately non-painter document order: decor (0,0, ID 0x30); ground (0,0, ID 0x20); under (−1,0); mid (0,3); top (2,−7, smallest ID). Ordinary sprites: one below B1 by entity ID, "tie-after" above B1's (0,0) cells and below its (0,3) cell, one at order 5, one on layer 1, and two on layer 2 at orders −8 and −7. Map B2 overlaps B1 with a higher entity ID than tie-after |
| C, transforms | C1 under a parent rotated 30°; C2 with scale (−1, 1); C3 under a parent with scale (1, −1); C4 with rotation 15° and scale (1.25, 0.6); C5 rotated 40° under a parent with scale (1.3, 0.8), so its cells shear; C6 with 0.5 m cells and translation (0.5, 0.25) under a parent with scale 2. Badge sprites lie just above C1 and just below C2 by entity ID |

- **Projects:**
  - `tile`: 37 entities, 15 maps, 215 occupied cells, 11 sprites, 0 diagnostic stand-ins.
  - `control-s0` and `control-s1`: the same entity IDs, with each map replaced by an
    empty parent with the map's Transform. They have 252 and 256 entities: 226 and 230
    sprites, and no tilemaps.
- **Views:**
  - `pp1`: pixel-perfect at 16 px/m and 320×180;
  - `pp3`: the same camera at 960×540, an integer 3× upscale;
  - `of25`: orthographic at 2.5× (vertical 9 m, 640×360), offset +4.15 / −1.96 px;
  - `o3-C`: exact 3× orthographic on board C (vertical 11.25 m, 960×540).
- **States**, each captured in fresh `screenshot` processes:

  | State | Change |
  | --- | --- |
  | s0 | initial scene, v1 atlas; captured, then repeated |
  | s1 | one `command.execute` with three `set_component Tilemap` commands: four A1 cells changed (one emptied, two holes filled, one frame swapped); A2 resized 4×3 → 5×3; B1 ground order 0 → 4 |
  | s2, s3 | `history.undo`, then `history.redo` |
  | s4 | v2 descriptor and v2 ballast imported (repack) |
  | s5 | `tiles-fail-missing` import, which must be rejected |
  | s6, s7 | `history.undo`, then `history.redo` of the repack import |
  | s8 | all five sources removed, cooked cache kept |

## Results

All values come from `evidence/analysis.json`. Two analysis runs produced byte-identical
output.

### Differential, byte comparison

| Comparison | Identical | Notes |
| --- | --- | --- |
| Tile against ordinary-sprite control, s0 | 10 / 10 | all views and magnifications |
| Tile against ordinary-sprite control, s1 | 6 / 6 | boards A and B after the edit |
| Fresh-process repeat of s0 | 10 / 10 | |
| s1 edit against s0 | 1 / 7 | as intended: board C is identical; A and B change 1,492 and 1,195 px at 1× |
| Undo against s0; Redo against s1 | 7 / 7; 7 / 7 | transaction `01M4MFMR9H0Y6W811859ESZMSH` |
| v2 repack against before | 10 / 10 | 13 of 13 frames relocated |
| Rejected missing-frame reimport against before | 3 / 3 | |
| Repack Undo (v1 page); Redo (v2 page) | 3 / 3; 3 / 3 | transaction `01M4MFNH487T31178XF7BWXBH8` |
| Sources removed against cached baseline | 10 / 10 | |

- **Integer upscale:** every 3× pixel-perfect frame equals its own 1× frame replicated
  3×3. There are 23 such pairs, with 0 mismatching pixels.
- **Magenta:** 0 of 86 frames contain a magenta-like pixel (R ≥ 200, G ≤ 40, B ≥ 200).
  No guard texel and no hidden RGB under zero alpha reached the screen.

### Independent model (`helpers/tile_model.py`)

The model predicts each pixel from the scene description and the source images, using:
- exact nearest sampling;
- binary alpha compositing in documented painter order;
- top-left coverage for edges through a pixel centre;
- the shader's nearest tie rule: the side of increasing screen coordinate on the
  dominant axis, with a 1/1024 px bias.

A pixel counts as a tie if any relevant edge, or any colour-changing texel boundary, lies
within 0.01 px of its centre.

| View | Pixels | Exact matches | Ties as rule | Ties other side | Unexplained |
| --- | --- | --- | --- | --- | --- |
| s0 pp1-A | 57,600 | 56,736 | 858 | 6 | 0 |
| s0 pp1-B | 57,600 | 57,600 | 0 | 0 | 0 |
| s0 pp1-C | 57,600 | 57,554 | 45 | 1 | 0 |
| s0 of25-A / B / C | 230,400 each | 230,320 / 230,394 / 230,289 | 80 / 6 / 111 | 0 | 0 |
| s0 o3-C | 518,400 | 518,259 | 138 | 3 | 0 |
| s1 pp1-A / pp1-B | 57,600 each | 56,662 / 57,600 | 932 / 0 | 6 / 0 | 0 |
| s1 of25-A / of25-B | 230,400 each | 230,320 / 230,388 | 80 / 12 | 0 | 0 |
| **Total** | **1,958,400** | **1,956,122** | **2,262** | **16** | **0** |

- **What the exact matches establish:**
  - bottom-to-top rows;
  - holes;
  - whole-frame stretching across unequal sizes and non-square cells;
  - minified 0.5 m and magnified 2 m cells;
  - every layer and sprite order relation on board B, before and after the s1 reorder;
  - all six transform cases;
  - the half-pixel map's tie edges, which resolve by the top-left rule;
  - the clipped strip.
- **Seams:** a seam pixel would be unexplained, because no candidate colour for a
  shared-edge tie is the background. There are none.

### Ties resolved against the documented rule (shared limitation)

There are 16 such pixels: 10 in s0, and the same 6 again in s1, where A4 is unchanged.
`figures/ties-off-rule-zoom12.png` rings each one.

| Where | Pixels | Tie | Engine choice |
| --- | --- | --- | --- |
| A4 non-dyadic map, pp1-A column 78 (cells (3,6) D, (3,5) X, (3,0) X) | 6 | texel boundary 7 of 16 exactly at the pixel centre (0.7 m cell = 11.2 px, cell origin −5.4 m) | lower texel (6) instead of the +x side (7) |
| C1, rotated 30°, pp1-C | 1 | texel boundary 0.00033 px away | lower texel |
| C1, rotated 30°, o3-C | 3 | one texel boundary and two coverage edges, each 0.00094 px away | other side |

The ordinary-sprite control shows the same pixels: its frames are byte-identical. These
are 0032 round five's documented non-dyadic and two-axis tie limitations; tiles add
nothing to them. Each is a one-pixel choice on a tie line, not a crack or an offset.

### Cooked pages and relocation

I decoded each page's KTX2 level 0 (`rgba8_srgb`) from the cooked cache and checked every
packed, extruded and empty texel against the authored sources. Each page was matched to
its descriptor and ballast by the cache's dependency hashes.

| Page | Fingerprint | Size, padding, mips | KTX2 sha256 | Content / extrusion / overlap / empty mismatches |
| --- | --- | --- | --- | --- |
| v1 (13 frames) | `135f00844748000573b8b9cae4cdb7028d1252c376fc741e4795c2029ed7cf30` | 42×68, 1, 7 | `2e72de4a2878…` | 0 / 0 / 0 / 0 |
| v2 (13 frames) | `a78215d554ac3fa8fb588c11403913e378317cede0becd35e2468f1b1ae4a64f` | 94×56, 2, 7 | `220f2892bd94…` | 0 / 0 / 0 / 0 |
| Rejected cook (12 frames, never registered) | `e9cd20afd65818d13a9124304e611f5ab6329c367da546f37b7218fc684b40b3` | 94×53, 2, 7 | `3ada2865504d…` | 0 / 0 / 0 / 0 |

- **Relocation is real:** all 13 frames moved, at unchanged sizes. Examples: D
  (1,33) → (76,2), X (1,51) → (76,22), C (27,9) → (54,28) and A (33,42) → (45,46).
- **Every v1 frame has a nonzero packed origin.** The smallest are K at (33,1) and D
  at (1,33).
- **Rendering is unaffected.** v1, v2, the v1 page after Undo and the v2 page after Redo
  all render identically. Packing offsets therefore never reach the grid.
- **Figure:** `figures/atlas-pages-v1-v2-zoom6.png` shows both decoded pages, with frames
  outlined.

### Public workflow

| Step | Result |
| --- | --- |
| Build | Atlas asset `01M4MFBBDJ4HRCCCX98FBM0VC9`, fingerprint v1 `135f0084…`. The scene is one transaction `01M4MFBBMCTQWDVH325M497Z9A` of 37 `create_entity` commands (request sha256 `47cc5061…`; exact entities in `evidence/run/entities-tile-s0.json`). All three projects pass `validate` with 0 diagnostics |
| s1 edit | Revision 5. The exact request, with full Tilemap values, is in `evidence/run/steps/01-s1-edit.json`. `validate` passes |
| Undo, Redo | Revisions 6 and 7; frames as in the table above |
| Repack import | `changed: true`, `cache_hit: false`, no diagnostics; fingerprint v1 → v2; revision 8 |
| Rejected import | Exit 1, "Error: import rejected: 12 invalid sprite/tilemap reference(s)", then one line for each map that references frame C (A1–A5, A7, B1, B2, C1–C3, C6): "scene … entity … texture … frame 01JA39FR000000000000000003: referenced tile frame does not exist in the cooked atlas". Project and journal bytes, fingerprint and history are unchanged; it leaves 2 orphan cache files, which are allowed; `validate` passes afterwards |
| Repack Undo, Redo | Fingerprint v2 → v1 (revision 9) → v2 (revision 10) |
| Source-free | `badge.png`, `ballast.png`, `plates.png`, `tiles.atlas.json` and `tiles.png` removed. Cached `validate` passes with 0 diagnostics. A reimport then fails with "could not cook art/tiles.atlas.json: asset IO: No such file or directory (os error 2)" and leaves the project and journal unchanged |

Exact source fingerprints, per-state asset fingerprints and authored-file hashes are in
`evidence/run/build.json`, `evidence/run/steps/*.json` and
`evidence/run/states/*/record.json`.

## Appearance review

I viewed the engine frames and the derived figures, not only the statistics.

- **Grid and identity.** At 1× and 3×:
  - every cell shows its whole frame: red top, blue bottom, amber left, green right,
    with every corner in place;
  - glyphs are upright and read correctly, and rows appear exactly as authored top-down;
  - holes show the plate;
  - stretched frames keep hard nearest edges.
  - Non-square cells and other non-integer stretches show texel widths that differ by
    one pixel. The 20- and 24-texel frames in 16-pixel cells skip some interior columns
    but keep both edge columns, as nearest minification must.
  - Minified 0.5 m cells drop texels, so a ring side can thin to one pixel and the
    exactly 2:1 D frame alternates. This is expected nearest minification. For pixel
    art, keep cell pixels at or above frame texels.
- **Transparency.** T's transparent ring shows the cell below as a clean one-texel frame,
  and X's cross shows the layer beneath. There is no magenta, fringe or dark rim.
- **Order** (`figures/board-b-order-s0-s1-zoom6.png`):
  - In s0, decor draws over ground despite its document position.
  - The checker under-layer shows only through holes.
  - The lower-ID badge shows through a ground hole.
  - Tie-after covers half of E but sits under the mid-layer ruler.
  - B2 covers B1's (0,0) cells but sits under B1's mid X, and the order-5 badge covers
    that X.
  - The top-layer X sits over the layer-1 ruler, between the two layer-2 badges.
  - In s1, ground at order 4 hides decor, mid X and B2's overlapped cells exactly as
    intended (`figures/s1-changes-pp1-A-B-zoom2.png`).
- **Transforms.**
  - Reflect-x reverses column order and mirrors each glyph and ring (green on the left).
  - Reflect-y reverses rows (blue on top).
  - Rotated and sheared maps step cleanly, with neighbouring cells sharing edges and no
    cracks or overlaps.
  - The badge above C1 and the badge partly under C2 keep their order.
  - At 2.5×, pixel art shows uneven 2–3 px texel widths, the usual reason to use
    pixel-perfect cameras. Exact 3× is crisp.
- **Clipping.** The 2.5× view cuts strip A7 at its top edge with no artefact, and frame
  edges are never cut inside the view.
- **Relocation and history are invisible.** Repack, rejection, Undo, Redo and source
  removal change no pixel.

## Observations for Astra (no defect)

1. **Non-dyadic shared edges are not bit-identical in f64 (informational).**
   - **What:** the tile path computes a vertex as corner + offset, where
     corner ∈ {0, cw} and offset = x·cw. The right edge of cell x is therefore cw + x·cw,
     while the left edge of cell x+1 is (x+1)·cw. For cw = 0.7, x = 5 they differ by
     1 ULP (4.2 against 4.199999999999999).
   - **Effect:** the difference disappears at f32 and in fixed-point rasterization. No
     seam appeared in A4 or any other view. The control builds the same sums, so the
     comparison cannot detect it.
   - **Option, only if bit-identical neighbour edges are wanted:** compute cell corners
     as f64::from(x + i)·cw for i ∈ {0, 1}.
2. **Rejection text names entity IDs only (low).** The CLI lists one line per affected
   map with entity, texture and frame. It gives neither the entity name nor the first
   offending `layers/N/cells/M` path, although the structured diagnostic carries that
   path. Printing the path would let an author find the cell without searching every
   layer.
3. **Not observable here.** Each `screenshot` prepares a fresh scene, so this harness
   cannot reach any of these:
   - retained geometry and Arc identity across motion;
   - single world-matrix slots;
   - atomic rebuilds and callback rollback.

   Astra's numerical native tests remain the only evidence for them; I did not
   independently verify them.

## Capture budget and timing

| Item | Value |
| --- | --- |
| Captures | 86 of 128, all written, none failed |
| Raw decoded bytes | 87,321,600 (83.3 MiB) of 256 MiB |
| Largest per invocation | 10 frames, against a limit of 16; one frame at most 2,073,600 bytes (960×540) |
| Engine calls | 132 in total: run `r2` 128 (init 3, import 12, rpc 20, validate 7, screenshot 86); run `r1` 4 (init 1, import 3) |
| Expected failures | 2 imports (the rejection and the source-free reimport) |
| Last capture ended | 2026-10-10 23:21:57 −04:00 |

- **`r1`** stopped after its fourth engine call on a wrong assertion in my helper (I
  expected the atlas asset ID to equal the descriptor ID). It made no capture, and its
  directory is kept as a record.
- **Overwritten evidence:** none. Every frame is in `evidence/ledger.jsonl`.

## Changed paths

All changes are inside `handoffs/0039-native-tilemaps/`. `brief.md` is Astra's and
unchanged.

| Path | Purpose |
| --- | --- |
| `result.md` | this report |
| `helpers/tile_art.py` | original art, atlas descriptors and the art manifest |
| `helpers/tile_scene.py` | scene description, tile expansion and ordinary-sprite control expansion |
| `helpers/tile_review.py` | public-path engine driver: build, shoot and step; hash checks, GPU gate and budget ledger |
| `helpers/tile_model.py` | independent CPU reference with tie classification |
| `helpers/tile_analyse.py` | differential, model, upscale, magenta, cooked-page and workflow checks |
| `helpers/tile_figures.py` | the 4 derived figures |
| `helpers/tile_evidence.py` | evidence assembly; never replaces a file with different content |
| `helpers/test_tile_tools.py` | 7 self-tests |
| `art/` | 9 committed source files, which regenerate byte for byte |
| `evidence/` | analysis, run records, call log, ledger, frames, figures, `images.sha256` and `artifact-manifest.json` |

## Commands and results

Run from the repository root, with `H=handoffs/0039-native-tilemaps/helpers`,
`R=artifacts/0039-native-tilemaps/r2`, `ALL=pp1-A,pp1-B,pp1-C,pp3-A,pp3-B,pp3-C,of25-A,of25-B,of25-C,o3-C`
and `AB=pp1-A,pp1-B,pp1-C,pp3-A,pp3-B,of25-A,of25-B`:

```sh
python3 -I -B $H/tile_art.py handoffs/0039-native-tilemaps/art
python3 -I -B $H/tile_review.py build $R                  # 3 projects; art matches committed
python3 -I -B $H/tile_review.py shoot $R tile s0 $ALL
python3 -I -B $H/tile_review.py shoot $R control-s0 s0 $ALL
python3 -I -B $H/tile_review.py shoot $R tile s0-repeat $ALL
python3 -I -B $H/tile_review.py step $R s1-edit
python3 -I -B $H/tile_review.py shoot $R tile s1 $AB
python3 -I -B $H/tile_review.py shoot $R control-s1 s1 pp1-A,pp1-B,pp3-A,pp3-B,of25-A,of25-B
python3 -I -B $H/tile_review.py step $R undo   && python3 -I -B $H/tile_review.py shoot $R tile s2-undo $AB
python3 -I -B $H/tile_review.py step $R redo   && python3 -I -B $H/tile_review.py shoot $R tile s3-redo $AB
python3 -I -B $H/tile_review.py step $R repack && python3 -I -B $H/tile_review.py shoot $R tile s4-repack $ALL
python3 -I -B $H/tile_review.py step $R reject && python3 -I -B $H/tile_review.py shoot $R tile s5-rejected pp1-A,pp1-B,pp1-C
python3 -I -B $H/tile_review.py step $R import-undo && python3 -I -B $H/tile_review.py shoot $R tile s6-import-undo pp1-A,pp1-B,pp1-C
python3 -I -B $H/tile_review.py step $R import-redo && python3 -I -B $H/tile_review.py shoot $R tile s7-import-redo pp1-A,pp1-B,pp1-C
python3 -I -B $H/tile_review.py step $R source-free && python3 -I -B $H/tile_review.py shoot $R tile s8-source-free $ALL
python3 -I -B $H/tile_analyse.py $R artifacts/0039-native-tilemaps/analysis/r2-final-1.json   # and -2: identical
python3 -I -B $H/tile_figures.py $R artifacts/0039-native-tilemaps/analysis/r2-final-1.json artifacts/0039-native-tilemaps/figures-2
python3 -I -B $H/tile_evidence.py $R artifacts/0039-native-tilemaps/analysis/r2-final-1.json artifacts/0039-native-tilemaps/figures-2
python3 -I -B $H/test_tile_tools.py                                    # 7 tests OK
cd handoffs/0039-native-tilemaps/evidence && shasum -a 256 -c images.sha256   # 20 OK
```

- Every step and capture command above exited 0, except the two expected import
  failures recorded inside the `reject` and `source-free` steps.
- The figures rebuild byte for byte (`figures-1` equals `figures-2`).
- Before any capture, CPU previews of the predicted views (`tile_model.py`, kept in the
  ignored `preview/`) checked the layout. They are not evidence.
- **Not run:**
  - Cargo, Clippy, the Rust and GPU test suites;
  - the UI suite and `tools/tests`, since no file outside the packet changed;
  - Play, the native player or native app, hosted CI and other GPUs or phones.

## Artifact manifest

- **Committed in this packet:**
  - `art/`;
  - `helpers/`;
  - `evidence/analysis.json`;
  - `evidence/run/`: build, scenes, exact tile entities, steps, state records, and
    control-list hashes with a byte-for-byte regeneration check;
  - `evidence/engine-calls.jsonl` and `evidence/ledger.jsonl`;
  - `evidence/frames/`: 16 unedited engine frames, all of s0 and the A/B views of s1;
  - `evidence/figures/`: 4 derived figures, which are not engine images;
  - `evidence/images.sha256`;
  - `evidence/artifact-manifest.json`, which lists every committed file and all 217
    ignored artifact files with sizes and SHA-256.
- **Ignored, under `artifacts/0039-native-tilemaps/`:**
  - runs `r1` and `r2`, with projects, journals and cooked caches;
  - all 86 frames;
  - analyses, figure builds, previews and art checks;
  - the ledger and the call log.
- **Not committed:** no binary artifact (executable, cooked cache or project) is
  committed.

## Limitations

- **Hardware and path:** one GPU and backend (Apple M5 Pro, Metal), headless authoring
  screenshots only. This is neither native game execution nor the retained native
  renderer, and no phone, performance or shipping-profile claim is made.
- **Coverage:**
  - pixel-perfect at 16 px/m at 1× and 3× integer output, orthographic at 2.5× with a
    sub-pixel offset, and exact 3×;
  - cells from 0.5× to 4× of their frames;
  - no perspective camera, camera roll or camera follower;
  - no magnification above 4× per texel.

  This does not establish arbitrary zoom.
- **Content:** tiles are nearest, white-tinted and binary-alpha by contract. Neither
  translucent tile art nor map/scene limits (1,024 maps, 262,144 slots) were exercised.
- **Model:** ties use a 0.01 px band. Explanations of the off-rule ties are inferred
  from distances and from the identical control, not from hardware documentation.
- **Shared limits, not re-measured here:**
  - ties above 128×;
  - two-axis frame-edge ties at magnification;
  - camera-following near ties.

  They remain as 0032 round five and 0034 recorded them.
- **Out of scope:** solid tilemap simulation and Rapier2D, legacy Play, tile painting and
  Inspector work. No phase gate is approved or claimed.

## Open questions

1. Should adjacent tile edges be bit-identical in f64 for non-dyadic cell sizes, by
   computing corners from integer multiples? No visible effect was found.
2. Should import rejections print the entity name or the first offending cell path,
   alongside the entity ID?
