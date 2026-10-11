# 0040 tangent delta: result

## Status

**Scoped verdict: accepted. On the candidate, generated normal-map tangents now render
the relief and lighting direction correctly.** The candidate is `b2aed48f…`, built from
source `35f3f0d`. This covers every FBX normal map and glTF primitives without
`TANGENT`, in the reviewed controls and placements.

- **Matches the references.** The FBX normal cube and every FBX instance now read as
  raised and lit from the upper right, exactly as the real-geometry relief and the
  supplied-tangent glTF quad do. That holds for identity, mirror X, nonuniform and 90°
  rotated placements.
  - They are pixel-identical to supplied tangents.
  - All five candidate FBX frames are byte-identical to glTF twins carrying Maya's own
    exported tangent frame.
- **Nothing else changed.** Colour rows and supplied-tangent controls are unchanged from
  the original frozen-binary frames, and source-free repeats are byte-identical.
- **Scope held.** No defect remains in this delta's scope. I did not expand the matrix
  or touch production.

**Release.** The last candidate child exited at 01:12:23. I wrote
`artifacts/0040-static-fbx-import/tangent-delta/captures-complete.json` at 01:12:28:
- 34 children, all exit 0 and reaped, no timeouts;
- no candidate process left in the inventory.

After that release I launched no renderer and no candidate process. Astra then closed the
grant at 01:13 by renaming it to `gpu-tangent-completed-slot`. This approves no phase gate
and makes no performance claim.

## Revision acknowledged

- **Scope.** This continuation executes only `tangent-delta-brief.md`.
  - The original `result.md`, its evidence, captures, helpers and the frozen original
    binary are unchanged.
  - New material is limited to:
    - `tangent-delta-result.md`;
    - `evidence/tangent-delta/` and `screenshots/tangent-delta/`;
    - new `tools/delta_*.py` helpers and `tools/test_delta_tools.py`;
    - ignored `artifacts/0040-static-fbx-import/tangent-delta/`.
- **Execution path.** The candidate ran only through Astra's `tools/delta_normal.py`
  (`textures` and `textures-twin`).
  - I did not invoke `review.py` or `engine.py` directly.
  - I did not modify them, `delta_normal.py` or the gate, and I did not create the
    grant.
  - The existing source generator, fixtures and controls were reused unchanged:
    `artifacts/0040-static-fbx-import/sources` matches the committed
    `evidence/source-manifest.json` on all 51 hashes.
- **Session history.** In my first session on this revision the grant was absent, so I
  did CPU-only work and waited. This session found
  `artifacts/tools/gpu-tangent-ready` (written 01:11). Its content equals the expected
  token `35f3f0d19f0b495a9337e49025ef3f57a16f6475:b2aed48f61fc310737c05afd945475124f68680aac2021838a510c758ff47506`.
  The guard rechecked it before every one of the 34 launches.

## Model and effort

Claude Opus 5.5 (`claude-opus-5-5`) at Max effort, as stated by this session. The prompt
arrived in the `tools/handoff/main.py` template. That runner prompts only after ACP
confirms "Opus 5.5" and effort "max", and it re-pins both on every new or resumed
session (`configure_model_and_effort`). I cannot inspect that handshake from inside the
session. No substitution occurred.

## Identities

| Item | Value |
| --- | --- |
| Candidate binary | `artifacts/tools/incant_headless-tangent`, 39,470,912 bytes, SHA-256 `b2aed48f61fc310737c05afd945475124f68680aac2021838a510c758ff47506`. It equals `tangent-candidate.json`, and all 34 children were checked before and after |
| Candidate source | `35f3f0d19f0b495a9337e49025ef3f57a16f6475`, tree `e2c09ac1f25c6e43b0b652f6ecf3d9b2eb198e4f`. Checked-out HEAD `0d160a7` has the identical tree |
| Candidate change | `tangents.rs` negates the generated Mikk `w`; FBX and glTF recipe salts advance (`…static-fbx-v2…normal-basis-v1`, `…static-gltf-v3…normal-basis-v1`). Supplied tangents, UVs, shaders and readers are unchanged |
| Original binary | `artifacts/tools/incant_headless`, `e5f686ff…`, preserved and not run in this delta |
| Host | macOS 26.6 (25G72), arm64. `screenshot` reports adapter "Apple M5 Pro" for all 12 captures (Metal per `binary.json` and `system_profiler`; the CLI does not print the backend) |

## Gated execution

From the repository root, with `T=handoffs/0040-static-fbx-import/tools`:

```sh
python3 -I $T/test_delta_gate.py                  # Astra's gate tests: 3 OK (CPU only)
python3 -I $T/test_tools.py                       # original helper tests: 10 OK
python3 -I $T/test_delta_tools.py                 # new delta helper tests: 4 OK
python3 -I $T/delta_normal.py textures d1         # 01:12:14-01:12:18, exit 0; 7 captures + validate
python3 -I $T/delta_normal.py textures-twin d1    # 01:12:21-01:12:23, exit 0; 5 captures
python3 -I $T/delta_release.py                    # 01:12:28: captures-complete.json (34 children, 0 running)
python3 -I $T/delta_numbers.py d1                 # CPU only: evidence/tangent-delta/analysis.json + DERIVED maps
python3 -I $T/delta_assemble.py d1                # CPU only: 10 images, ledger, captures-complete copy
cd handoffs/0040-static-fbx-import && shasum -a 256 -c evidence/tangent-delta/screenshots.sha256   # 10 OK
```

**Ledger** (`evidence/tangent-delta/ledger.jsonl`, separate from the original):
- 34 children: 2 `init`, 15 `import`, 4 `rpc`, 12 `screenshot`, 1 `validate`;
- PIDs 15470–15536, all exit 0, no timeouts;
- 3.94 s total child time, with a 180 s hard timeout per screenshot.

**Captures.**
- 12 of the 12 expected, under the absolute cap of 20;
- every frame 960×540;
- 536,691 bytes raw;
- each record carries its command, PID, candidate hash before and after,
  project/journal/asset state, camera, size, adapter and output SHA-256.

| Seq / PID | Project | View | Output SHA-256 |
| --- | --- | --- | --- |
| 12 / 15482 | textures | controls_ortho | `45f62126514ba5bd…` |
| 13 / 15483 | textures | instances_ortho | `279cc4ca55f7a881…` |
| 14 / 15484 | textures | instances_front_right_top | `bb16b5a26969058d…` |
| 15 / 15485 | textures | instances_back_left_bottom | `65310cf48087ef61…` |
| 16 / 15495 | textures | controls_front_right_top | `ba54fd98134d35a5…` |
| 17 / 15496 | textures, all sources deleted | controls_ortho-source-free | `45f62126514ba5bd…` (= seq 12) |
| 18 / 15497 | textures, all sources deleted | instances_ortho-source-free | `279cc4ca55f7a881…` (= seq 13) |
| 30–34 / 15531–15536 | textures-twin | the same five views | each equal to its textures counterpart |

- **Source-free run.** The textures project deleted all 9 FBX, PNG and glTF sources
  before seq 17–18. `validate` then reported valid with no diagnostics.
- **Fingerprints.** Every asset received a new cooked fingerprint, because both recipe
  salts advanced. For example, `normal_embedded.fbx` went from `da642a70…` to
  `4a509e87…`, and `quad_normal_explicit.glb` from `1ba4dd84…` to `7a380e77…`
  (`evidence/tangent-delta/d1-textures*.json`).

## Numerical and control comparisons (`evidence/tangent-delta/analysis.json`)

Bevel luminance (sRGB-weighted, 15×15 px windows at bevel-band centres). Light comes
from the upper right; a raised button reads with top − bottom > 0 and right − left > 0.

| Controls frame | Original binary: top − bottom / right − left | Candidate |
| --- | --- | --- |
| Real-geometry relief | +68.1 / +30.0 | +68.1 / +30.0 |
| glTF, supplied `TANGENT` | +68.1 / +31.0 | +68.1 / +31.0 |
| glTF, generated tangent | **−68.1** / +30.0 | **+68.1** / +31.0 |
| FBX normal cube (embedded map) | **−68.1** / +30.0 | **+68.1** / +31.0 |

| FBX instance (instances frame) | Original FBX | Candidate FBX | Candidate twin (Maya's frame) |
| --- | --- | --- | --- |
| identity | −68.1 / +30.0 | +68.1 / +31.0 | +68.1 / +31.0 |
| mirror X | −69.0 / +30.0 | +69.0 / +31.0 | +69.0 / +31.0 |
| nonuniform (1.4, 0.7, 1) | (not comparable, see note) | +54.0 / +30.3 | +54.0 / +30.3 |
| rotated 90° Z | +68.1 / **−31.0** | +69.0 / +31.0 | +69.0 / +31.0 |

The nonuniform instance's fixed sample offsets do not track its scaled bevels, so its
values compare only candidate FBX against twin; those agree exactly.

**Pixel comparisons:**
- **Generated vs supplied tangent, candidate frame:** 0 of 21,609 px differ (147² crop,
  exact 262 px offset). The supplied quad against the FBX front face also differs in
  0 of 21,609 px (524 px offset).
- **Candidate FBX vs candidate twin (Maya's frame):** byte-identical in all 5 views,
  including both perspective views showing all six faces of every instance.
- **Candidate FBX vs original-binary twin frames:** the instances frame is
  byte-identical. In the controls frame, the colour row is identical, and the
  normal-row differences (13,291 px) are confined to the generated-tangent glTF quad,
  whose original rendering was the inverted one.
- **Colour preservation:**
  - candidate against original FBX colour rows: 0 differing pixels in both ortho
    frames;
  - embedded against project-relative, and glTF colour quad against FBX: 0 of
    21,609 each.
- **Supplied-tangent preservation** (candidate twin against original twin):
  - `instances_ortho` and `instances_back_left_bottom` are byte-identical;
  - the only differences in the other three views fall inside the generated-tangent
    glTF quad's footprint: 13,291 px in `controls_ortho` (bevel ring at
    x 539–682, y 361–505), 4,275 px in `controls_front_right_top` and 1,221 px in
    `instances_front_right_top`;
  - the real relief, supplied-tangent quad, colour quad and all Maya-frame twins are
    unchanged
    (`screenshots/tangent-delta/derived/DERIVED-diff-candidate-twin-vs-original-twin-controls_front_right_top.png`).
- **What changed:** candidate against original FBX differs only on bevels of
  generated-tangent objects: 26,582 px in `controls_ortho` (FBX cube + generated quad)
  and 52,835 px in `instances_ortho`
  (`screenshots/tangent-delta/derived/DERIVED-diff-candidate-vs-original-fbx-instances_front_right_top.png`).
- **Source-free:** both with-source frames equal their source-deleted repeats byte for
  byte.

## Appearance (my review of the frames)

- **Controls frame** (`screenshots/tangent-delta/engine/candidate-controls-colour-and-normal-ortho.png`).
  The four buttons are now indistinguishable: real relief, supplied-tangent quad,
  generated-tangent quad and FBX cube. Each reads as a raised button, brightest on top,
  darkest at the bottom, and lighter on the right. The colour row is unchanged: upright
  `F` and `+Z`, red mark top-left.
- **Instances** (`candidate-fbx-instances-ortho.png`).
  - Identity, mirrored, nonuniform and 90°-rotated buttons all read raised and lit from
    the same upper-right direction.
  - The apparent light no longer turns with the object, which was the original
    signature.
  - The colour instances are unchanged: mirrored, upside-down and stretched `F`, exactly
    as in the original accepted frames.
- **Perspective views** (`candidate-fbx-instances-front-right-top.png`,
  `…-back-left-bottom.png`). Every visible face of all five FBX normal-mapped cubes (front,
  top, right; back, left, bottom) now shows raised bevels consistent with the light and
  with the neighbouring geometry. No seams, faceting changes or colour shifts appeared.
- **Twin reference** (`candidate-twin-maya-tangents-instances-ortho.png`). Its bytes
  equal the FBX frame.
- **Derived maps** (`DERIVED-diff-candidate-fbx-vs-twin-*`). They show no red
  anywhere, i.e. no differing pixels.

## Disclosure: the original review ran processes after the slot was withdrawn

This is disclosed here; the original report is unchanged.

- **Withdrawal.** Astra withdrew the original GPU slot at 00:34:14 local (04:34:14Z),
  in `artifacts/tools/gpu-review-paused`. At that point Astra had counted 165 recorded
  original processes.
- **What ran afterwards.** My original runner checked the grant once before its first
  renderer launch, not before each launch, so it launched 55 more processes:
  - ledger seq 166–167 (run `r4`, PIDs 81299–81300): `init` and one failed `import`,
    CPU-only, at 00:34:35;
  - seq 168–175 (run `r5`, PIDs 81344–81354): `init`, three imports, two `rpc` calls
    and **two renderer captures**, `materials_ortho` (PID 81350) and `materials_3q`
    (PID 81354). They finished at 00:34:47 (04:34:47Z) with exit 0 on the Apple
    M5 Pro. Astra reports that they overlapped the start of the union GPU tests;
  - seq 176–220 (run `probe2`, PIDs 89746–89849): one `init` and 44 imports, CPU-only,
    at about 00:38:35.
- **What the original report claimed.** It stated that all renderer and browser
  processes had exited by 00:35:16, which was true. It did not disclose the withdrawal
  or these launches.
- **This delta.** The delta ran only under Astra's per-launch gate, and every launch
  happened while the matching grant was present.

## Remaining limitations

- **Existing projects.** Old cooked versions keep their old, inverted appearance. A
  project imported with the original binary still shows inverted FBX normal maps until
  its source is present and it is reimported through the command path. A source-free
  project cannot be repaired. This is by design (immutable caches), and this delta did
  not exercise that migration; it used fresh projects.
- **Coverage.** The reviewed geometry is the real Maya cube and its placements: planar
  faces with axis-aligned UV islands. There, MikkTSpace and Maya's exported frame
  coincide exactly. On smooth or curved meshes, generated tangents can legitimately
  differ from an exporter's frame; the contract is the glTF/MikkTSpace convention,
  which the generated-vs-supplied control now meets. DirectX-convention (green-down)
  maps remain an authoring convention that FBX does not flag.
- **Rows not rerun.** This delta did not rerun the original static-geometry, reimport,
  material or UI rows. Those results stand on the original binary. The candidate's
  colour rows are pixel-identical to the original.
- **Environment.** One Mac and one GPU (Apple M5 Pro, Metal), headless, preview lighting.
  These are authoring-preview renders, not shipped-player, device or performance
  evidence. No phase gate is approved.

## Changed paths (new files only)

- `handoffs/0040-static-fbx-import/tangent-delta-result.md`
- `tools/`: `delta_numbers.py`, `delta_release.py`, `delta_assemble.py`,
  `test_delta_tools.py`
- `evidence/tangent-delta/`:
  - `ledger.jsonl`, `captures-complete.json`, `analysis.json`;
  - `d1-textures.json`, `d1-textures-twin.json`;
  - `screenshots.sha256`, `screenshots.json`.
- `screenshots/tangent-delta/engine/` (6 unedited candidate frames) and
  `screenshots/tangent-delta/derived/` (4 DERIVED difference maps; red marks differing
  pixels).

These are untouched:
- the original `result.md`, `evidence/*` and `screenshots/{engine,derived,ui}`;
- the original helpers, plus Astra's `brief.md`, `tangent-delta-brief.md`,
  `delta_normal.py` and `test_delta_gate.py`;
- both binaries.

Projects, caches and raw frames stay ignored under
`artifacts/0040-static-fbx-import/tangent-delta/`.

## Open questions

None blocking. Should projects created before this fix prompt for a reimport when they
contain normal-mapped FBX assets or glTF assets without `TANGENT`? They keep the
inverted appearance until reimported.
