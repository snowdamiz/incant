# 0034 cooked atlas review: identity, relocation, filtering and public workflows

## Status

**Accepted, scoped, conditional on the shared wide-region review. I found no
atlas-specific rendering defect.** I reviewed actual cooked atlas frames through the
frozen headless binary on one Apple M5 Pro with Metal:

- **Stable frame identity holds.** Every sprite that references a frame ULID renders
  that frame. This holds through a relocation of 129 of 130 frames, a full repack of all
  130, Undo/Redo, a live watcher reimport and source removal. Lookup is scoped to the
  sprite's texture: a second atlas reusing two frame IDs shows its own content.
- **The cooked pages are exact.** Five cooked pages were decoded from KTX2 level 0.
  Every packed texel, every extruded edge texel and every empty texel matches the
  authored sources, with no overlap.
- **Nearest atlas frames render identically to their sources.** In every camera, they
  are byte-identical to a control project that draws the same entities from the
  original images with explicit rectangles. That covers pixel-perfect 1× and 3×, 2.5×
  orthographic with a sub-pixel offset, 7° and 3° rolls, 1.125× and 0.875×, and exact 3×.
- **Relocation and page-size changes leave nearest pixels untouched.** Only two things
  change by design: the resized ballast image, and one sprite I deliberately authored
  with a raw pixel rectangle.
- **Linear atlas frames are smooth and bleed-free, but not byte-stable.** Across
  repacks and against the control, a few hundred pixels differ by at most 2 levels.
- **The span cuts render cleanly.** 112 strip rows decode exactly against the
  documented nearest rule at 1× to 32×. They include frames packed at y = 1163 to
  1273, flips, exact and near ties, and the 32× leading texels that failed in 0032
  round three. Nine more rows decode exactly on a frame packed at x = 6266.

Findings for Astra, detailed under "Requests for Astra":

1. **Packing** (cook time, not rendering): the packer rejects a set that fits as "frames
   do not fit", and a larger `maximum_size` can yield a 60% larger page for the same frames.
2. **Diagnostics:** `import` prints Rust Debug text for failures. An accepted reimport
   can make every render of the project fail while `validate` reports valid. The
   missing-frame error names no entity.
3. **Authoring hazard:** an explicit pixel rectangle on an atlas texture silently shows
   unrelated page content after any repack.
4. **Linear repack stability:** at most 2 levels, as above. This is informational.
5. **Shared renderer, not atlas-specific:** atlas frames inherit 0032's tie limit above
   16× exactly (atlas = control byte for byte). On the source-Y axis I also measured
   failures at the span cut and the next texels after it, up to strip texel 526, where
   0032 reports none.

**Not claimed:** acceptance of the shared renderer, phase-gate approval, native editor
presentation, hosted CI, other GPUs, phones or performance. The pixel-camera near-tie
follower defect is not tested here and remains open.

## Director feedback acknowledged

The packet's status paragraph says: begin the independent atlas review on the frozen
executable now, keep overall acceptance conditional on 0032's shared review, use no
other binary, and run no Cargo. I applied it:
- **Binary:** only `artifacts/tools/incant_headless` ran, hash-checked before and after
  each scripted call. No other binary, live rebuild or Cargo was used.
- **Shared renderer:** I do not label it accepted. 0032's round-four result arrived on
  its branch during this session (`b9eca69`, "accepted, scoped, up to 16×"). I read it
  read-only and measured how atlas frames relate to it. Its acceptance and integration
  remain Astra's and the director's.
- **Scope:** no `editor/ui`, Rust, shader, SDK, transport, docs or account file changed.
  No merge or publish.
- **Interrupted attempt:** none. The worktree held only `brief.md` in this packet.

## Model, effort and ACP

- **Model:** Claude Opus 5.5, model ID `claude-opus-5-5`, as stated by the session.
- **Effort:** Max.
- **Transport:** the prompt arrived in the exact template of `tools/handoff/main.py`.
  That runner sends a prompt only after `configure_model_and_effort` confirms model
  "Opus 5.5" and effort "max" over ACP (`tools/acp/config.json`). The pinned adapter
  is `@agentclientprotocol/claude-agent-acp` 0.88.0 with `@anthropic-ai/claude-code`
  2.1.295.
- **What I cannot verify:** the ACP handshake itself, from inside the session. No model
  or effort substitution occurred.

## Provenance

| Item | Value |
| --- | --- |
| Worktree HEAD | `3825439d9e1f746999487ada3bb866a443ca2683`, the binary's source commit |
| Source tree (`binary.json`) | `28d089c1c8a4acf65bfe0c9798f0e2737ad4612c`; runtime sources match atlas merge `34d5044` |
| Binary | `artifacts/tools/incant_headless`, 38,108,864 bytes, sha256 `b72389ee24276459938705850b5cfe3efe2ffb1b744ba94ecf1f5d2ba8cc27a2` |
| Hash checks | Matched `binary.json` before and after every scripted call: 285 per final run, 1,005 in total |
| GPU and host | Apple M5 Pro (adapter reported by `screenshot`), macOS 26.6 (25G72), arm64 |
| Tooling | Python 3.14.8 standard library only; no Node, TypeScript or Cargo |

- **Earlier build hash in the docs.** `docs/spikes/sprite-atlases.md` cites headless
  `0257f761…` for Astra's probes. That is a different build, so I used only the frozen
  `b72389ee…`.
- **Unbracketed exploratory calls.** Ten import-only calls during design (smoke and
  layout probes) were not hash-bracketed individually. They fall between checked calls,
  and the binary file is read-only.

## What I built

All authoring went through `init`, `import`, `rpc command.execute` / `history.undo` /
`history.redo` / `project.save`, `watch-assets` and `validate`. No project or journal
file is ever written by my tools. Source art and descriptors are copied into the
project as authoring inputs.

### Original art (`tools/atlas_art.py`)

The new art is designed so that atlas failures are easy to see:
- **ID cards:** twelve 12×14 cards (0–9, A, B), each with a unique colour, inside an
  opaque magenta guard. A wrong or shifted frame shows a wrong digit or magenta.
- **Edge probe:** 11×9, with a different colour on each side and corner and an "F"
  glyph, so flips, turns and one-texel offsets are unambiguous.
- **Self-naming strips:** 1100-texel horizontal and vertical strips in which every texel
  names its index through (R, G), inside the display's identity range. The ramp phase
  puts the span cuts at texel 512 and 1024 mid-ramp, so a seam cannot hide in a colour
  wrap. They also supply 1-texel rows/columns and 700-texel sub-rectangles starting at
  source texel 100.
- **Halo:** a soft halo with garbage green RGB under zero alpha.
- **Ballast:** a night panorama in two sizes, 1200×40 and 1240×150, whose change
  relocates the other frames.
- **Labels:** 94 label words in 0032's 3×5 font. Labels are atlas frames too, so a wrong
  frame shows a wrong word.

Reused byte for byte, with provenance in `evidence/art/manifest.json`: 0032's
`sheet.png` (`7451ef95…`) and `badge.png` (`648441d4…`), plus its font and PNG writer
(`pngio.py` `5b73bef8…`, `sprite_art.py` `45aec9a2…`). The sheet supplies the Wisp
frames, the asymmetric dialog panel, the key, the 7×7 unique-texel ruler, the orb, the
glow, the crate, the tiles, the solid and the marker.

`evidence/art/` holds every source, descriptor and the manifest. The art regenerates byte
for byte; all three fresh runs produced identical art.

### Atlases

| Descriptor | Change | Frames |
| --- | --- | --- |
| `atlas-v1` | maximum 2048×2048, padding 2; 129 optional source rectangles; the ballast omits its rectangle (whole image) | 130 |
| v2 | only `ballast.png` grows to 1240×150; same descriptor and frame IDs | 130 |
| `atlas-v3` | padding 5 and maximum 1600×2600; v1 sources | 130 |
| `alt` | a second atlas reusing card 0's and card 1's frame IDs with swapped content | 2 |

Failing descriptors: `fail-nofit`, `fail-bounds-area` (4096×2048), `fail-bounds-axis`
(8193×256), `fail-padding0`, `fail-padding17`, `fail-identity` (lower-case ULID),
`fail-rect`, `fail-frames257` and `fail-texels` (4,194,305 decoded source texels).
`render-fail-shrink-panel` and `render-fail-drop-cardB` are accepted by import but leave
the scene unable to render. `limit-texels-exact` has exactly 4,194,304 decoded source
texels.

### Projects, boards and cameras (`tools/atlas_scene.py`, `tools/atlas_lookdev.py`)

- **Atlas project:** one transaction creates 445 entities: 411 sprites, 6 mirror parents
  and 28 cameras. The engine reports 0 diagnostic stand-in entities.
- **Control project:** the same 445 entity IDs and transforms, drawn from 9 ordinary
  textures with explicit rectangles.

Boards are at 16 px per metre:

| Board | Content |
| --- | --- |
| S | all 12 cards at 1× and 2×, Wisp frames, key, ruler 1× and 3×, crate, probe 1× and 3×, star, marker, 18-tile rows on grid and half-pixel placements |
| T | probe at 1× and 2× and the 7×7 ruler on tie (1×, 3×) and aligned placements, in nine orientations: none, flip X, flip Y, both, mirror parent, 90°, 180°, 270°, 90° + flip X; five pivots with origin markers |
| N | asymmetric nine-slice panel (borders 3, 6, 4, 8): native, 72×44 against a simple stretch, four compressed targets, flips, 180°, mirror parent, densities 8 and 32, 12° rotation, a 1-1-1-1 probe slice |
| O | layer, order, ID tie, Z ignored, interleaved atlas and ordinary textures, 50% overlaps on dark and light, nearest orb/glow/halo with transparent edges, a Wisp stack |
| L | linear: orb 4× on dark and light, halo 3× and 2.5×, glow, Wisp 2×, crate at 30°, nine-slice panel, probe 3× and flipped, card 7 at 2.5× and 1× |
| B | ballast; card 7 by raw v1 pixel rectangle, by stable ID, and through the alt atlas |
| W, V | 1× strips: aligned, tie, flips, ±1/256 px near ties, 700-texel sub-rectangles |
| X, Y | 44 magnified rows or columns each: 2× to 32× windows on the 512 cut, 1024-cut rows, 700-texel sub-rectangles, near ties at 8×, 32× leading and trailing edges |

Cameras:
- pixel-perfect at 320×180 (1×) and 960×540 (3×);
- ordinary orthographic at 2.5×, offset by (+4.15, −1.96) px;
- 7° rolled orthographic;
- exact 3× orthographic on boards S and N;
- for W and V: 1.125× and 0.875× with an offset, and a 3° roll.

States, in order, each rendered in fresh `screenshot` processes:

| State | What changes |
| --- | --- |
| s0 | v1, rendered twice |
| s1 | ballast image resized: relocation |
| s2 | Undo |
| s3 | Redo |
| s4 | the live watcher reverts only `ballast.png` |
| s5 | v3 descriptor |
| s6 | ten rejected replacements |
| s7 | the live watcher sees a failing descriptor, then the restored one |
| s8 | three accepted changes that cannot render, each undone |
| s9 | every source removed |

The whole sequence ran twice from scratch (run-2 and run-3).

## Results

All values come from `evidence/analysis-run-2.json` (cross-checked against run-3)
unless another file is named.

### Cooked pages: decoded from KTX2, independent of the renderer

| Version | Fingerprint | Page | Pad | Mips | KTX2 sha256 | Packed + extruded texels | Mismatches (content/extrusion/empty) |
| --- | --- | --- | --- | --- | --- | --- | --- |
| v1 | `336afbb5b077…` | 1204×1865 | 2 | 11 | `97f9ed64b7ec…` | 121,548 (5.4%) | 0 / 0 / 0 |
| v2 | `cdf14e3ddd87…` | 1244×1975 | 2 | 11 | `d5f511c1db9a…` | 260,148 | 0 / 0 / 0 |
| v3 | `919685dfac12…` | 1210×2284 | 5 | 12 | `1ff4a458c88f…` | 195,696 | 0 / 0 / 0 |
| shrink panel | `02adc0713400…` | 1204×1865 | 2 | 11 | `1c18cde3704b…` | 120,908 | 0 / 0 / 0 |
| drop card B (129 frames) | `3e3763c27c49…` | 1204×1865 | 2 | 11 | `fb9489fe3871…` | 121,260 | 0 / 0 / 0 |

- **Coverage:** every frame keeps its authored size and name, and no extrusion
  rectangles overlap.
- **Format:** pages are `rgba8_srgb`. Every 8-bit texel in these sources survives the
  cook's decode-to-linear and re-encode exactly. Values absent from the sources are not
  tested.
- **Determinism:** both fresh runs produced identical fingerprints and identical bytes
  for all 7 cooked KTX2 files.
- **Negative controls** (`evidence/controls-run-2.json`): flipping one bit in a packed,
  an extruded or an empty texel each fails verification. The unmodified page passes.

### Stable identity and relocation

Packed origins, (x, y); every frame keeps its size:

| Frame (size) | v1 | v2 | v3 |
| --- | --- | --- | --- |
| hstrip (1100×4) | 2, 46 | 2, 156 | 5, 55 |
| hrow (1100×1) | 10, 54 | 10, 164 | 19, 69 |
| hmid (700×1, from source x 103) | 15, 59 | 15, 169 | 19, 80 |
| vstrip (4×1100) | 2, 54 | 2, 164 | 5, 69 |
| vcol (1×1100) | 10, 59 | 10, 169 | 5, 1179 |
| vmid (1×700, from source y 103) | 10, 1163 | 10, 1273 | 19, 91 |
| card 7 | 346, 82 | 343, 192 | 91, 115 |
| panel (nine-slice) | 15, 82 | 15, 192 | 30, 721 |
| probe | 442, 82 | 439, 192 | 91, 257 |
| 7×7 ruler | 457, 82 | 454, 192 | 74, 1107 |
| halo | 43, 82 | 43, 192 | 30, 755 |
| orb | 151, 82 | 151, 192 | 30, 1055 |

- **v1 → v2:** 129 of 130 frames move, some in both axes. Only the ballast at (2, 2)
  stays.
- **v1 → v3:** all 130 move.
- **Page size:** 1204×1865 → 1244×1975 → 1210×2284.
- **Scoped lookup:** frame IDs are scoped by texture. Through the alt atlas, card 0's ID
  shows card 1 and card 1's ID shows card 0, both exact.

### Rendered invariance (differential, byte comparison)

| Comparison | Identical frames | Differences |
| --- | --- | --- |
| v1 rendered twice in fresh processes | 34 / 34 | none |
| Control (original images) against atlas v1 | 30 / 34 | only board L (linear), in 4 views: 1, 255, 46 and 9 px, at most 1, 1, 2 and 1 levels |
| v2 (relocation) against v1 | 20 / 26 | board L: 2, 230 and 65 px, at most 2 levels. Board B: only the ballast and raw-rectangle regions |
| Undo against v1 | 10 / 10 | none |
| Redo against v2 | 10 / 10 | none |
| Watcher dependency revert against v1 | 10 / 10 | none |
| v3 (padding and page change) against v1 | 20 / 26 | board L: 3, 352 and 136 px, at most 2 levels. Board B: only the raw-rectangle sprite (1,512 px at 1×) |
| After ten rejected replacements, against v3 | 6 / 6 | none |
| After the three render failures are undone, against v3 | 6/6, 6/6, 1/1 | none |
| Every source removed, against v3 | 26 / 26 | none |
| run-2 against run-3, from scratch | 229 / 229 | none |

- **Where the linear differences are.** Attribution by projected sprite quads puts every
  board-L difference inside a linear sprite: orb and halo gradients, the 30° crate,
  card 7, the probe, the panel and the Wisp. The card-7 pixels lie 1.4 to 2.7 texels
  inside its edges, at glyph transitions. They are UV rounding, not bleed from atlas
  neighbours.
- **Pixel-perfect 3× against exact 3× orthographic.** They differ only in two places on
  board S, the half-pixel tile row and its rim, and in six cases on board N:
  non-integer stretches, compressed borders, density 32, the 1-1-1-1 probe slice, and
  the 12° panel with its rim. Pixel-perfect output replicates the 1× frame.

### Decoded checks, measured from engine pixels

- **Strips across span cuts.** Every pixel of 112 strip rows was checked against the
  documented nearest rule: 109,088 along-axis pixels, 0 mismatches. The rule is
  region-local texels, a bias of 1/1024 screen px toward increasing screen coordinate,
  and clamping to the region.
  - Interior runs are exactly M px at every M from 1 to 32. Steps are ±1, with no holes
    or extra pixels.
  - At every visible cut the texels on each side are consecutive. Examples are
    511 | 512, and 588 | 587 for a flipped strip, whose cut sits at w − 512.
  - The 700-texel sub-rectangle gives 611 | 612, and 288 | 287 flipped.
  - At 32×, where 1100 × 32 = 35,200 > 32,768, the leading and trailing texels are
    exactly 32 px. That includes the flipped leading texel 1099, at aligned and tie
    placements on both axes.
  - Negative controls fail as they must: a wrong tie side fails 44 of 112 rows (every
    exact-tie row), an origin one texel off and inverted flips fail 112 of 112.
  - Figures: `screenshots/figures/decoded-x-cuts-1024x832.png` and
    `decoded-y-cuts-832x1024.png`.
- **Exact predictions at exact placements, at 1× and 3×.** Each sprite is predicted from
  its source image and compared pixel for pixel. All 152 sprites match with 0
  mismatching pixels:
  - board S: 84 sprites;
  - board B: 9, including the raw, stable and scoped card 7;
  - board T: 45 orientation cases and 5 pivots, including tie rows;
  - board N: 9 nine-slices, covering native, 72×44, three flips, 180°, mirror,
    density 8 and the simple 24×24.

  Pixels whose source value exceeds the display's identity range (sRGB 234) are skipped,
  83 at 1×; they are the Wisp eyes, the star and the markers.
- **Painter order.** Six groups, predicted from (layer, order, stable entity ID) for
  opaque and binary-alpha sprites, all match exactly: layer, order, ID tie, Z ignored,
  atlas/ordinary batch interleave, and the Wisp stack. Drawing in reverse order
  mismatches 232, 232, 232, 232, 50 and 232 px.
- **Transparent edges and fringes.** The halo hides green RGB and the orb hides magenta
  RGB under zero alpha. With linear sampling at 3× and 2.5× on dark and light plates,
  and nearest at 2×, no pixel is tinted toward the hidden colour: 0 in every region.
- **Level-0 sampling, minified.** On board Z, the halo is minified to 6, 9 and 12 px
  and card 7 to 3 and 6 px, both linear and nearest. They are byte-identical to the
  control, so no neighbour or mip content reaches them.

### Public workflows

| Step | Result |
| --- | --- |
| Undo / Redo | `history.undo` reverts the v2 import (transaction `01M4KSP0PPE0KFQQPTPH0ZMGMY`). The fingerprint returns to v1 and the frames equal v1. Redo returns v2 exactly. |
| Live watcher, dependency only | Copying the v1 `ballast.png` back while `watch-assets` runs reimports the atlas: actor `source-watch`, `changed: true`, `cache_hit: true`, fingerprint equal to v1. The runtime atlas generation goes 1 → 4, and the frames equal v1. |
| Live watcher, failing descriptor | An unfittable descriptor yields "could not cook art/atlas.atlas.json: frames do not fit in one atlas within the requested dimensions". The runtime keeps the v3 atlas at generation 1 through every event. Restoring the file clears the diagnostic, the watcher exits 0, and project and journal bytes are unchanged. |
| Ten rejected `import`s | `NoFit`, `Bounds` ×4, `Identity`, `Invalid` (rectangle exceeds image), `Frames` (257), `Limit` (decoded source texels) and `Invalid` (linear usage) all exit 1. Project and journal are unchanged, and later frames are identical. |
| Exact texel limit | 4,194,304 decoded source texels are accepted (a 20×80 page); one more is rejected. |
| Shrunken nine-slice frame | Import exits 0 and `validate` reports valid. Then every screenshot fails, including boards without the sprite: "sprite entity 01JA34EN000000000000000056 (N:native), texture …, source region [429, 82, 8, 8], cooked texture size [1204, 1865]: nine-slice borders must fit…". Undo restores exact frames. |
| Removed frame (card B) | Import exits 0. Every screenshot fails with "sprite atlas frame 01JA34FR00000000000000000V is unavailable in texture …"; no entity is named. Undo restores exact frames. |
| Frame ID on an ordinary texture | The command is accepted. The render fails explicitly, "…unavailable in texture <badge>". Undo restores. |
| Source-free | 11 source files removed (both descriptors and all images); 26 of 26 frames are identical. Reimport then fails with `Io(NotFound)`. |
| Repeated fresh processes | Every screenshot is a new process. 34 of 34 repeats are identical, and run-2 equals run-3 across 229 frames. |

Retained prepared GPU scenes across a replacement cannot be reached through the public
harness. Each `screenshot` prepares one scene in a fresh process, and no public call
holds a prepared scene across a reimport. I verified only two forms of retention:
- old cooked versions stay loadable, through Undo and the watcher's cache hit;
- the CPU runtime keeps the old atlas on a failed reload, through the watcher.

Astra's retained-scene GPU test is not independently verified.

### Large packed origin probe (`tools/atlas_extra.py`, `evidence/extra/`)

A maximum height of 8 forces seven 1040-texel frames side by side. The page is
7308×5, and frame w6 sits at (6266, 2), sourced from strip texels 60–1099, so its cut is
at page x 6778.
- **Nearest:** all 9 rows decode exactly. That covers 1× aligned, tie, flipped and
  flipped-tie, the neighbouring frame w3, and 8× windows on the cut in four placements.
- **Control and repeat:** the pixel-perfect frame equals the control byte for byte, and
  the fresh repeat is identical.
- **Linear:** in the fractional and rolled views, 10 and 2 px differ by 1 level, only
  in the 2.5× linear strip at the large origin.

### Shared renderer: upper-span ties on atlas frames (`tools/atlas_upper_probe.py`, `evidence/upper/`)

0032 round four reports exact-tie failures at span texels 236–511 from 20×. I drew the
atlas's 1100-texel rows and columns at 36×, 40×, 48× and 56×, at exact ties, with windows
over span texels 236 to about 548, in 1920×1080 pixel-perfect frames. The probe ran
twice, with the boards' world positions swapped.

- **The atlas inherits the limit exactly.** Atlas and control are byte-identical in
  all four frames, so atlas packing adds nothing.
- **Every failure is a tie** and resolves to the lower texel. There are no non-tie errors.

| Source axis, board centre | Failed exact ties at 36× / 40× / 48× / 56× | Failing span texels |
| --- | --- | --- |
| x, origin | 93/324, 278/288, 219/320, 269/315 | 236–511 |
| x, (500, 500) | 53/324, 262/288, 198/320, 262/315 | 239–511 |
| y, (500, 500) and origin, identical | 124/300, 185/297, 36/322, 232/320 | 239–511, **and 512–526 just after the cut** |

On the source-Y axis, exact ties at strip texels 512 to 526 also fail at 36×, 40× and
56×. That includes texel 512 itself, the first texel after the cut, which shows 511 at
40× and 56×. Both board positions give identical counts, so this follows the axis, not
the world position.

0032 states "None is on a span cut" and "span texels 0 to 57 never failed, up to 128×",
and its 64-texel-span option leans on that observation. My measurements contradict
both statements for this axis. Figure: `screenshots/figures/decoded-upper-y-40x-cut-ties.png`.

### Packing efficiency (import only, no frames; `evidence/packing/`)

The same 130 frames and sources were cooked under several maximum sizes:

| maximum_size | Page | Texels |
| --- | --- | --- |
| 2048×2048 | 1204×1865 | 2,245,460 |
| 1204×1170, 2048×1170 or 1300×1200 | 1204×1166 | 1,403,864 |
| 1600×2600 | 1204×2260 | 2,721,040 |
| 1204×1148 | rejected, NoFit | – |
| 1204×1147 and 4096×1024 | rejected, NoFit (genuinely impossible) | – |

- **Why the page grows.** A taller bound makes best-short-side-fit drop the 700-texel
  column below the 1100-texel one instead of beside it. The packer searches several
  widths but never heights.
- **Memory.** Sprites upload level 0 as RGBA16Float, so the GPU copy grows from about
  10.7 MiB to 17.1 MiB for identical frames.
- **Minimal reproduction of the false NoFit:** three solid frames, 1200×40, 1100×4 and
  4×1100, with padding 2. Under maximum 1204×1148 import reports "frames do not fit". A
  valid layout exists by the engine's own rules: wide at (2, 2), the column at (2, 46),
  the row at (10, 46). With maximum 1204×2048 the page is 1204×1156.
  `tools/atlas_packing_probe.py --repro` regenerates this exactly.

## Appearance review

I viewed the engine frames themselves, not only the statistics.
- **Atlas frames are indistinguishable from their sources at pixel-perfect scales.**
  - Edges are hard and texel blocks uniform.
  - Each card shows its own digit and colour, and labels read correctly.
  - The magenta guards never appear, and extrusion is invisible.
  - Zero-alpha texels show the background exactly.
- **Relocation is invisible.** v1, v2 and v3 frames of every nearest board are
  pixel-identical. The only visible change is the raw-rectangle sprite:
  - v1: card 7;
  - v2: night sky from the grown ballast;
  - v3: a band of repeated strip-extrusion texels over empty page.

  That is the hazard stable IDs exist to avoid (`screenshots/figures/board-b-v1-v2-v3-zoom3.png`).
- **Nine-slice from the atlas is crisp.**
  - Corners keep their source density, and each corner gem and the gold tab sit in the
    correct corner after flips, the mirror and the 180° turn.
  - The 7° roll shows clean stair steps without cracks.
  - Density 32 on a 16 px/m camera drops border texels unevenly, as in 0032. Keep
    border density at or below camera density for pixel art.
- **Linear atlas frames are smooth.** Orb, halo and glow have no dark rim, halo or
  fringe on dark or light plates, and repack differences of 1 to 2 levels are invisible.
  Linear on pixel art (the Wisp at 2×, the probe at 3×, card 7) reads soft, as
  expected. Nearest remains the right default.
- **Wide strips are seamless.** Ramps run continuously across every span cut at 1× to
  32× and under the 3° roll. The decoded parity views are perfect checkers.
- **Cameras behave as expected.** At 2.5× ordinary orthographic, pixel art shows
  uneven 2 to 3 px texel widths, the expected reason to use pixel-perfect cameras.
  Exact 3× orthographic matches pixel-perfect 3× except on half-pixel placements.
- **The cooked page looks wasteful.** The decoded v1 page
  (`screenshots/figures/atlas-pages-v1-v2-v3-quarter.png`) is about 95% empty. The
  packing finding explains why.

## Requests for Astra

### 1. Packer: false NoFit, and page size that grows with the bound (low to medium)

- **Repro:** `python3 -I handoffs/0034-sprite-atlas-review/tools/atlas_packing_probe.py --repro OUT`
  imports the three frames above at 1204×1148, 1204×2048 and 1204×1200.
- **Effects:**
  - Valid sets near a tight bound are rejected with a message that claims impossibility.
  - A generous bound can enlarge the page by 60% for the same frames (1204×1166 →
    1204×1865, both measured), costing phone memory.
- **Recommendations:**
  - Search candidate heights as well as widths, for example by retrying with a
    decreasing maximum height until NoFit. Alternatively, use a max-rects heuristic.
  - Reword NoFit as "could not pack these frames within the requested maximum
    (unrotated guillotine heuristic)".
  - Document that `maximum_size` influences the layout.
- **Regression tests:**
  - The three-frame set packs within 1204×1148.
  - For a fixed frame set, raising `maximum_size` never increases the page area.

### 2. Import diagnostics (low)

- **Repro:** `import` of any `fail-*` descriptor in `evidence/art/`.
- **What happens:** `import` prints the Debug form, for example
  `Error: Cook { path: "art/atlas.atlas.json", source: Atlas(Bounds) }`. The watcher
  prints readable text for the same failure. `Atlas(Bounds)` covers four different
  mistakes: padding 0, padding 17, too many texels, and too wide.
- **Recommendation:** print `ImportError`'s Display text, as `watch-assets` does.

### 3. Accepted reimports that make every render fail (medium for authoring)

- **Repro:** import `render-fail-shrink-panel.atlas.json` or
  `render-fail-drop-cardB.atlas.json` over the v1 project.
- **What happens:**
  - The import commits, and `validate` reports `valid: true`.
  - Every `screenshot` of the project then fails, including scenes that never draw the
    affected sprite.
  - The nine-slice error is precise: it names the entity, texture, packed rectangle and
    page size.
  - The missing-frame error names only the frame and texture. A frame ID on an ordinary
    texture is also accepted at command time.
- **Recommendations:**
  - When an atlas reimport removes a referenced frame, or shrinks one below its
    nine-slice borders, list the affected sprites in the import reply, as a warning or
    a rejection.
  - Let `validate` check sprites against cached layouts.
  - Add the entity ID to the missing-frame error.
  - This extends 0032's round-one request 2 to atlases.

### 4. Explicit pixel rectangles on atlas textures (low, authoring hazard)

- **What happens:** a `[x, y, w, h]` region on an atlas asset renders correctly until
  the next repack, then shows unrelated content or extrusion, silently.
- **Recommendation:** have `validate` or import warn, or reject explicit pixel regions on
  atlas textures, steering authors to frame IDs. The Inspector could show the resolved
  frame instead.

### 5. Linear sampling is not byte-stable across repacks (informational)

- **What happens:** linear frames differ by up to 2 levels on a few hundred pixels
  between page layouts and against the original image. The cause is normalized-UV
  rounding, and it is invisible.
- **Recommendation:** golden-image tests should not expect byte equality for linear atlas
  frames across repacks. If exact stability is wanted, region-local manual bilinear
  sampling with `textureLoad` would provide it.

### 6. Shared renderer high-magnification ties (low; for 0032 and the shared fix)

- **Status:** atlas frames inherit 0032's measured limit exactly.
- **Additional repro:** `python3 -I handoffs/0034-sprite-atlas-review/tools/atlas_upper_probe.py OUT`.
  On the source-Y axis, exact ties fail just after the cut: strip texels 515 to 526 at
  36×, 512 to 522 at 40× and 512 to 525 at 56×. At 40× and 56× that includes the cut
  texel 512.
- **Recommendation:** add this case to the shared regression. Re-check the 64-texel-span
  option's premise; my measurement suggests that the span's on-screen size, not only the
  texel coordinate, matters. That cause is an inference.

## Changed paths

All changes are inside `handoffs/0034-sprite-atlas-review/`. `brief.md` is unchanged.

| Path | Purpose |
| --- | --- |
| `result.md` | this report |
| `tools/atlas_art.py` | original art, the atlas descriptors and the manifest |
| `tools/atlas_scene.py` | boards, cameras, and the atlas and control texture bindings |
| `tools/atlas_lookdev.py` | the public-path state sequence, hash checks and the packet budget ledger |
| `tools/atlas_analyse.py` | page verification, differential and decoded checks, attribution |
| `tools/atlas_figures.py` | derived figures |
| `tools/atlas_extra.py` | the large packed origin and minified-linear probe |
| `tools/atlas_upper_probe.py` | the upper-span tie probe, with `--swap`, `--decode-only` and `--stats` |
| `tools/atlas_packing_probe.py` | import-only packing probe, with `--repro` |
| `tools/atlas_controls.py` | negative controls |
| `tools/atlas_evidence.py` | evidence assembly; never replaces a file with different content |
| `tools/test_atlas_tools.py` | 11 self-tests |
| `evidence/` | analyses, helper records for both final runs, the scene, art, extra/upper/packing records, controls, the budget ledger and image hashes |
| `screenshots/engine/` | 21 unedited engine frames |
| `screenshots/figures/` | 7 derived figures |

Nothing else changed: no Rust, shader, Cargo, SDK, transport, schema, docs or
`editor/ui` file. 0032's folder is as I found it; I removed only bytecode caches my
imports had created there.

## Commands and results

From the repository root, `P=handoffs/0034-sprite-atlas-review/tools`,
`A=artifacts/0034-sprite-atlas-review`:

```sh
python3 -I $P/atlas_lookdev.py $A/run-2        # 285 calls, 229 frames, 5 expected failed shots
python3 -I $P/atlas_lookdev.py $A/run-3        # identical
python3 -I $P/atlas_analyse.py $A/run-2 $A/analysis/final-run-2c.json $A/run-3
python3 -I $P/atlas_extra.py $A/extra-1
python3 -I $P/atlas_analyse.py $A/extra-1 $A/analysis/extra-1.json --extra
python3 -I $P/atlas_upper_probe.py $A/upper-1
python3 -I $P/atlas_upper_probe.py $A/upper-2-swapped --swap
python3 -I $P/atlas_upper_probe.py $A/upper-1 --decode-only            # and upper-2-swapped
python3 -I $P/atlas_upper_probe.py $A/analysis/upper-span-stats.json --stats $A/upper-1 $A/upper-2-swapped
python3 -I $P/atlas_packing_probe.py $A/run-2/art $A/packing-probe-1
python3 -I $P/atlas_packing_probe.py --repro $A/packing-repro-2
python3 -I $P/atlas_controls.py $A/run-2 handoffs/0034-sprite-atlas-review/evidence/analysis-run-2.json $A/analysis/controls-run-2.json
python3 -I $P/atlas_figures.py $A/run-2 $A/analysis/final-run-2c.json $A/figures-3 --upper $A/upper-2-swapped
python3 -I $P/atlas_evidence.py
python3 -I $P/test_atlas_tools.py                                       # 11 tests OK
python3 -I handoffs/0032-sprite-rendering/tools/test_reference.py       # 13 OK (reused module, unchanged)
python3 -m unittest discover -s tools/tests                             # 25 OK (repository baseline)
cd handoffs/0034-sprite-atlas-review && shasum -a 256 -c evidence/screenshots.sha256   # 28 OK
```

Preliminary runs are preserved, not overwritten, and not used for the verdict:
- `run-smoke1`: the earlier palette;
- `run-1`: the scene before the 32× edge rows and the scoped atlas, with the same
  verdicts;
- `run-smoke2`: smoke of the final scene.

Two harmless slips occurred and both were fixed. A loop variable shadowed the probe
summary's filename, so `upper-2-swapped`'s summary was written as `UV:56x:506` without
end-of-run fields; `evidence/upper/provenance.json` records this. A 3× pixel-perfect
prediction initially ignored integer replication; it was my model, not the engine.

Not run: Cargo, Clippy, the Rust and GPU suites, the UI suite (no UI file changed),
`play` or PlaySession captures, native editor captures, hosted CI, other GPUs and phones.
I did not run the watcher on the render-failing descriptors.

## Budget

| Item | Value |
| --- | --- |
| Frames per invocation | 1; largest 1920×1080, 7.9 MiB raw |
| Packet total | 744 frames, 810.1 MiB raw, against limits of 2,048 frames and 1 GiB |
| Final runs | each 229 frames and 237.0 MiB raw |
| Probes | extra 9 frames; upper-span 4 + 4 |
| Overwritten evidence | none |

Expected failed screenshots wrote no frame: 15 across the three full runs. Every frame
is in `evidence/budget-ledger.jsonl`, with per-run totals in `evidence/budget.json`.

## Artifact manifest

Committed in this packet:
- `evidence/`: the files listed under "Changed paths".
- `screenshots/engine/` (21 files):
  - v1 at 3×: boards S, T, N, O, L and B;
  - the 1× strip boards W and V, and the magnified boards X and Y;
  - orthographic views: S at 2.5× with an offset, S at exact 3×, N rolled 7°, W rolled 3°;
  - v2 board B, pixel-perfect and rolled;
  - v3 board B, pixel-perfect and 2.5×;
  - board Z with packed x = 6266;
  - upper-span X and Y at 1920×1080.
- `screenshots/figures/` (7 files, derived, not engine frames): decoded X and Y cuts,
  cooked pages v1 to v3, board B in v1 to v3, the ×64 linear difference, the
  upper-span Y cut ties, and the art contact sheet.

Ignored, under `artifacts/0034-sprite-atlas-review/`: every run directory with its
projects, journals, cooked caches and all raw frames, the analyses, figures, probes and
the ledger.

## Data binding needed later

A sprite and atlas Inspector (Phase 2) will need a typed, read-only view of each atlas
asset's current cooked layout:
- frame ID → name, packed rectangle, source path and rectangle;
- page size and fingerprint.

It also needs a way to flag sprites whose frames are missing or too small, without
reading cache files. Today this data is only in the `import` reply and the cache
metadata.

## Limitations

- **Hardware:** one GPU and one backend, Apple M5 Pro with Metal, headless only. No
  other adapter, no phone, no performance claim.
- **Display transform:** the exact predictions assume the measured identity range of
  sRGB 0 to 234 from 0032; brighter pixels are skipped, not judged.
- **Upper-span sampling:** counts depend on magnification, axis, position and frame
  size. The sample is positions I chose, not an exhaustive map. Causes are inferred.
- **Packing set:** my frame set is deliberately adversarial (1100-texel strips).
  Typical sprite sets may pack better.
- **Not covered:** perspective cameras, atlas pages used as model textures (their mips
  mix frames), multiple pages, rotation, compression tiers, tilemaps, animation
  libraries and 2D physics. None of these is claimed.
- **Gameplay path:** no legacy PlaySession or native ECS gameplay was exercised.
- **Gates:** no phase gate is approved or claimed, and the shared renderer is not
  labelled accepted.

## Open questions

1. Should the packer treat `maximum_size` purely as a bound, by searching heights or
   compacting, even at some extra cook time?
2. Should an atlas reimport that breaks referencing sprites warn or be rejected at
   import time? Should explicit pixel regions on atlas textures be linted?
3. Are byte-stable linear atlas frames across repacks a requirement for golden tests?
4. Should the shared regression adopt the source-Y cut case, and should the bound be
   documented as "ties hold to 16×" for atlases too?
