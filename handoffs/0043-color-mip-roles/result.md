# 0043 color mip roles: result

## Status

**Scoped verdict: accepted.** The candidate is `8775c499…`, built from `4f7ecfa`. After a
command-path recook, OPAQUE base colour, emissive maps (OPAQUE and BLEND) and the Color
environment keep their RGB under minification. MASK/BLEND coverage, sprites, the atlas,
normal maps and Linear maps are unchanged. No defect was found, so no repro is needed.

- **Alpha is ignored where it should be.**
  - Every mixed-alpha, all-zero-alpha and 3-channel card renders pixel-identical to its
    all-alpha-one twin. This holds at LOD −0.32, 0.68 and 2.26 for OPAQUE base colour and
    for emissive maps on OPAQUE and BLEND materials (21 cells, 0 of 6,400 px differ in
    each).
  - The sky with alpha 0 / 0.5 / 1 lights both spheres pixel-identically to its
    alpha-one copy (0 of 49,130 px).
- **Coverage consumers are preserved.** The single mixed image shared by OPAQUE, MASK and
  BLEND materials keeps its MASK holes and BLEND translucency byte for byte. The sky's
  sprite crop and both atlas sprites are unchanged.
- **Old caches keep their old look, by design.** The candidate renders the
  baseline-cooked projects byte-identically to the baseline in all three views, so the
  historical darkening persists in retained versions until a command reimport. This is
  the documented immutable-cache policy, not a source-free runtime regression.
- **Changes are confined to the intended roles.** On the same candidate binary, recooked
  versus retained differs only in the 12 OPAQUE/emissive mixed or zero-alpha cells at
  LOD > 0: 49,880 px, with 0 elsewhere. In the environment, all 43,414 changed pixels
  lie inside the exact sphere silhouettes.
- **Source-free replay holds.** Fresh processes after deleting every source reproduce the
  recooked frames byte for byte.

**Release.** The last engine child ended at 10:32:43.561 local (2026-10-11, −04:00). At
10:33:14 I wrote `artifacts/0043-color-mip-roles/captures-complete.json` and the runner's
`RELEASED` lock, and told Astra in this session's output.
- Totals: 56 children, all exit 0 and reaped, no timeouts, and no process from either
  frozen binary.
- Astra then removed both grants (`coordination.txt`, 10:33:35).
- No engine process ran after release. Everything since has been CPU-only.

Not implied: no phase gate, reference-phone, performance or full FBX-support claim.

## Priority revisions and director feedback

The current brief has no priority-revision section, and no newer director feedback
reached this session. Two later in-worktree notes from Astra were read and followed:

- **`artifacts/tools/coordination.txt`.** "FINAL GPU RELEASE… No further engine calls
  authorized. Finish CPU-only report/evidence." It also confirms the foreign Cargo chain
  noted under timing was CPU-only.
- **`artifacts/tools/acp-verification.json`.** The model and effort proof, below.

Both are copied into `evidence/`.

## Model and effort

Claude Opus 5.5 (`claude-opus-5-5`) at Max effort.

- **Proof.** Astra's record `evidence/acp-verification.json` (sha256 `5f68339e…`) states
  that `tools/handoff/main.py` verified "Claude session verified: Opus 5.5; thinking: max"
  before the initial prompt: session exec 97358, permission mode `acceptEdits`.
- **Consistent signals.** This session's system prompt names `claude-opus-5-5`, and its
  environment has `CLAUDE_EFFORT=max` (`sdk-ts` entry point). As Astra's record notes,
  environment variables are not the proof.
- **Limit.** I cannot observe the ACP handshake from inside the session. I did not read
  the /tmp launcher log. No substitution occurred.

## Identities

| Item | Value |
| --- | --- |
| Baseline binary | `artifacts/tools/incant_headless-color-baseline`, 39,470,912 bytes, sha256 `b2aed48f61fc310737c05afd945475124f68680aac2021838a510c758ff47506` (= `color-baseline.json`). Source `35f3f0d19f0b495a9337e49025ef3f57a16f6475` (tree `e2c09ac1…`), the accepted PR77 tangent implementation |
| Candidate binary | `artifacts/tools/incant_headless-color-candidate`, 46,335,440 bytes, sha256 `8775c499001cd1cea11bbcedf739e7ba030f1efcee2cb8ee965458a00043a787` (= `color-candidate.json`). Source `4f7ecfa211add87ee207f4e5abd69161c868a609` (tree `6679c6c5…`) |
| Worktree | HEAD `4f7ecfa` (= candidate source); `crates/` and Cargo files clean. Baseline is an ancestor. Nothing was built; Cargo was not run |
| Guard | `tools/engine.py` (sha256 `10b1c222…`) and `tools/test_guard.py` (`7bf3ac9f…`), unchanged. Every one of the 56 children (CPU steps included) went through `configure()`. The grant and binary hash were rechecked before each Popen, and the hash was rechecked after each child |
| Host | macOS 26.6 (25G72), Apple M5 Pro. `screenshot` reported adapter "Apple M5 Pro" on all 13 frames (Metal inferred; the CLI does not print the backend) |

**What else differs between the binaries.** `35f3f0d..4f7ecfa` also contains renderer
integrations unrelated to this delta: pipeline prewarming, sprite plans and flipbooks,
tilemaps and the native window.
- The retained-cache frames show none of them changing a pixel in these views (byte-identical).
- The cook change is isolated by comparing two frames from the same candidate binary:
  retained versus recooked.
- Shaders, lighting, tone mapping and environment filtering are identical between the
  sources, apart from `radiance_texture()` selection.

## Policy under review (read-only summary of `9863f20`)

- **glTF base colour.** It resolves to `color_unweighted` when the material is OPAQUE.
  MASK/BLEND keep the existing alpha-weighted `color` chain.
- **Emissive.** Every emissive map, OPAQUE or BLEND, uses `color_unweighted`.
- **Linear and Normal.** Unchanged.
- **Shared sources.** A single source with both roles shares level 0 (Arc).
- **Formats and recipes.** Models emit INCMOD03, and the glTF/FBX recipe salts advance.
- **Standalone Color textures and atlases.** These emit manifest v3 with an independent
  chain. Sprites still use the primary chain (level 0 only), and environment
  preparation reads the independent chain.
- **Readers.** They keep the historical role choice for INCMOD01/02 and texture
  manifests v1/v2.

## Fixtures (3 disposable projects, public commands only)

Everything was built with `init`, `import`, `rpc` (`project.read`, `command.execute`,
`project.save`) and `validate`. No document or journal was written directly.

**Art provenance.**
- The art comes from the byte-identical 0041 reference: sha256 `46d9237d…` is checked,
  and exactly those verified bytes are executed (never edited or copied).
- 0032 `pngio` (`5b73bef8…`) is bound only while the reference imports.
- New 0043 art: the alpha twins, the metallic-roughness map, the sky, the tiled quads and
  the higher-contrast backdrop.

| Project | Content | Views |
| --- | --- | --- |
| materials | 42 cells of 80 px, pixel-aligned in an ortho board, each UV-tiled k = 1, 2 or 6 (LOD −0.32, 0.68, 2.26). One 64×64 image (the 0041 card: F glyph, alpha-0 teal triangle, alpha 0.2–0.8 stripes) is bound by **one glTF** to OPAQUE, MASK (0.5) and BLEND materials. Controls (second glTF): the same RGB with all-zero alpha, all-one alpha and as 3-channel PNG; the 0041 dome normal map on the all-one card; a metallic-roughness map carrying the card's mixed alpha (Linear). Third glTF: the mixed/zero/one cards as emissive maps on OPAQUE and BLEND (factor alpha 0.7). 0041 key light; 0041 2×1 Linear uniform environment; lit 22 px backdrop stripes | `board` (ortho), `oblique` (perspective) |
| environment | One 1024×512 **Color** equirectangular sky used by the `EnvironmentLight` and by a `SpriteRenderer` (a 256×128 crop). Its sky has alpha 0 over three quarters of its columns, a 0.5-alpha haze quarter, an opaque sun and opaque ground. At 1024 px wide, environment decode reads source mip 1. Also a matte and a polished-metal sphere; a zero-intensity authored light (disables the preview key, so the spheres show radiance only); the 0041 atlas (flag R, panel B) as sprites; unlit stripe sprites behind | `view` |
| envcontrol | The same scene with an all-alpha-one copy of the sky | `view` |

## Execution and ledger

States, all on the same project files:
1. **old:** built and imported by the baseline (INCMOD02, Color/Linear manifest v1, atlas
   v2), then rendered by the baseline.
2. **retained:** the same cache rendered by the candidate.
3. **recooked:** the candidate `import`s every source through `incant_cmd`.
4. **sourcefree:** all sources deleted, fresh processes.

| Seq / PID | Binary | State | View | Frame sha256 (prefix) |
| --- | --- | --- | --- | --- |
| 26 / 34531 | baseline | old | materials/board | `1c05e287af9484cb` |
| 27 / 34535 | baseline | old | materials/oblique | `91209d8992d1c822` |
| 28 / 34536 | baseline | old | environment/view | `816122d217ceb97e` |
| 29 / 34537 | baseline | old | envcontrol/view | `ab7cdddd5f357fca` |
| 30 / 34837 | candidate | retained | materials/board | `1c05e287af9484cb` (= 26) |
| 31 / 34839 | candidate | retained | materials/oblique | `91209d8992d1c822` (= 27) |
| 32 / 34840 | candidate | retained | environment/view | `816122d217ceb97e` (= 28) |
| 49 / 35452 | candidate | recooked | materials/board | `1aa80c7f4139a071` |
| 50 / 35453 | candidate | recooked | materials/oblique | `5c1c38594520282a` |
| 51 / 35454 | candidate | recooked | environment/view | `97b48b13407b89c7` |
| 52 / 35455 | candidate | recooked | envcontrol/view | `ab7cdddd5f357fca` (= 29) |
| 55 / 35610 | candidate | sourcefree | materials/board | `1aa80c7f4139a071` (= 49) |
| 56 / 35611 | candidate | sourcefree | environment/view | `97b48b13407b89c7` (= 51) |

- **Children.** 56 children, PIDs 34384–35611:
  - 3 `init`, 26 `import`, 6 `rpc`, 8 `validate`, 13 `screenshot`;
  - all exit 0 with empty stderr, no timeouts.
- **Timeouts and duration.** Hard timeouts were 60 s per frame and 60–120 s per CPU child.
  The longest child took 0.70 s. These are bookkeeping, not performance measurements.
- **Frame budget.** 13 of the 20-frame cap, each 960×540, 919,339 PNG bytes in total. The
  runner's cap also counts failed launches; there were none.
- **Recook receipts.** Every reimport kept its asset ID.
  - Every glTF and every Color texture/atlas got a new fingerprint, for example
    `shared.gltf` `21858612…` → `e0897f5f…` and `sky.png` `b18500a1…` → `8006aae9…`.
  - The Linear `env_uniform.png` kept `e822139b…` with `changed: false`, because its recipe
    did not change. That is also 0041's fingerprint for the same source.
  - Afterwards: INCMOD03 models, and Color manifests v3 carrying
    `unweighted_content_sha256`.
- **Sources deleted.** 15 (materials) and 7 (environment) files. Both projects validate
  with no diagnostics.

## Numerical comparisons (`evidence/analysis.json`)

**Byte identity.**

| Pair | Result |
| --- | --- |
| Baseline vs candidate on the retained old cache (board, oblique, environment) | identical ×3 |
| Recooked vs fresh source-free replay (board, environment) | identical ×2 |
| All-alpha-one environment control, baseline vs recooked candidate | identical |

**Board: alpha variant vs all-alpha-one control at the same LOD.** These are exact
per-pixel comparisons of pixel-aligned cells.

| Cell | Baseline: differing px / max channel Δ | Candidate recooked |
| --- | --- | --- |
| O-mixed k1 / k2 / k6 | 0/0 · 640/58 · 3,176/117 | 0/0 · 0/0 · 0/0 |
| O-zero k1 / k2 / k6 | 0/0 · 6,400/81 · 6,400/166 | 0 · 0 · 0 |
| O-rgb (3-channel) k1 / k2 / k6 | 0 · 0 · 0 | 0 · 0 · 0 |
| E-O-mixed k1 / k2 / k6 | 0/0 · 640/67 · 3,184/146 | 0 · 0 · 0 |
| E-O-zero k1 / k2 / k6 | 0/0 · 6,400/97 · 6,400/201 | 0 · 0 · 0 |
| E-B-mixed k1 / k2 / k6 | 0/0 · 640/56 · 3,200/112 | 0 · 0 · 0 |
| E-B-zero k1 / k2 / k6 | 0/0 · 6,400/78 · 6,400/159 | 0 · 0 · 0 |

**Board colours (sRGB means; baseline → candidate; control in brackets).**
- Teal triangle interior at LOD 0.68: 45/98/93 → 54/156/147 [54/156/147]. The candidate
  value equals the magnified triangle, which reads 54/156/147 on both builds (the
  magnified cells are byte-identical, so level 0 is unchanged).
- Emissive triangle at LOD 0.68: 35/105/99 → 50/172/162 [50/172/162].
- All-zero-alpha cell means at LOD 2.26:
  - OPAQUE: 39/39/39 → 188/162/139 [188/162/139];
  - emissive OPAQUE: 26/26/26 → 208/179/152;
  - emissive BLEND: 58/58/61 → 185/162/142.
- The baseline mixed card at LOD 2.26 is slightly *lighter* than its control
  (192/164/139 against 188/162/139). The coverage chain blacks out the triangle and also
  washes the partial-alpha stripes toward the opaque paper; OPAQUE should show them solid.

For context only, the historical 0041 report (not re-executed here) recorded the triangle
at 63/158/150 when magnified, 49/121/114 near LOD 0.47 and 33–40/33–49/32–46 near LOD
0.83–0.91, under different lighting.

**Cook-only change (same candidate binary, retained → recooked).**
- **Board.** 49,880 px change, all inside O-mixed, O-zero, E-O-mixed, E-O-zero, E-B-mixed
  and E-B-zero at k2/k6. The other 30 cells and the backdrop are unchanged: every k1
  cell, O-one, O-rgb, MASK, BLEND, the normal control, the Linear control and both
  emissive all-one controls (OPAQUE and BLEND).
- **Oblique.** 28,472 px, all within those same six materials' projected cells (cell
  outlines grown by 1.5 px). Here k1 cells also change where perspective minifies them;
  0 px change elsewhere.

**Environment.**
- Baseline vs recooked: 43,414 px differ, all inside the exact ray-traced sphere
  silhouettes. Sprites, atlas and stripes differ by 0 px.
- Recooked vs alpha-one control:
  - spheres differ by 0 of 49,130 px;
  - the atlas and stripes are identical;
  - only the sky sprite differs (29,645 px inside its inset rectangle), as intended,
    because the control's sprite is opaque.
- Upper-half sphere means:
  - metal: 66/81/108 → 137/174/239 [137/174/239];
  - matte: 115/122/148 → 138/157/201 [138/157/201].
- Lower-half means:
  - metal: 141/113/84 → 142/115/87 [142/115/87], a small change because its prefiltered
    ground reflection integrates a little sky;
  - matte: 128/118/121 → 140/138/153 [140/138/153], because sky light also reaches its
    lower hemisphere near the equator.

## Appearance review (my judgement of the frames)

I judged at full size and in enlarged crops, not from the hashes.

- **Board, magnified row.** Identical on both binaries and in every state: the four
  OPAQUE alpha variants look the same. Level 0 is preserved.
- **Board, minified rows.**
  - **Baseline.** The teal triangles turn dark at LOD 0.68 and become black specks at
    2.26. The all-zero-alpha card darkens and then goes black. Zero-alpha emissive cards
    go dark or black, and the BLEND one becomes a dark veil over the stripes.
  - **Candidate.** Each variant is indistinguishable from its all-alpha-one neighbour.
    The stripes stay solid red under OPAQUE, as glTF requires.
- **MASK and BLEND** (shared source). Unchanged and as intended:
  - MASK cuts the triangle and the 0.2/0.4 stripes and keeps 0.6/0.8. The holes sit in
    the expected places at all three LODs, with clean edges where magnified;
  - BLEND shows the backdrop through the triangle and grades the stripes;
  - the oblique view agrees.
- **Normal and Linear controls.** The dome reads raised and lit from the key's upper left
  (the PR77 fix is in both builds). The metallic F and triangle read darker, as the map
  intends. Both are unchanged at every LOD.
- **Environment.**
  - **Baseline.** The polished sphere reflects a black sky wherever the source alpha is 0,
    with only the haze sector, sun and ground visible. The matte sphere is dim and
    lacks blue sky light.
  - **Candidate.** Both spheres match the alpha-one control: a full zenith-to-horizon
    gradient, the sun glint and warm ground.
- **Sprites.** The sky sprite is unchanged and correct: transparent where alpha is 0
  (stripes visible), 50% haze, an opaque sun disc and ground. Its edges have no dark or
  light fringes, including the alpha 0 → 0.5 boundary. The atlas R glyph and panel are
  unchanged.

**Observation outside scope.** At magnification, the first kept pixel at a MASK cut edge
blends in the transparent texels' RGB: 180/181/155 next to paper 205/189/158, beside the
teal triangle. This is unchanged level-0 straight-alpha filtering (the opaque control
shows the same value), not this delta.

## Original versus new cache policy (as exercised)

- **Original (retained) versions keep the historical role choice.** That covers INCMOD02
  models and texture manifest v1 / atlas v2, whether the reader is the baseline or the
  candidate.
  - OPAQUE base colour, emissive maps and Color environments use the alpha-weighted chain.
  - Alpha-0 RGB becomes black from mip 1 onward, and partial alpha is under-weighted.
  - The candidate reproduces this byte for byte and does not silently repair it.
  - Source-free readers cannot repair it.
- **New versions require a source-backed command reimport.** Reimport cooks the
  role-separated forms (INCMOD03; Color manifest v3 with an independent chain).
  - Asset IDs are stable.
  - Linear/Normal-only standalone textures keep their v1 fingerprint, so they publish no
    new version.
  - Old cache blobs remain on disk.

## Timing and slot record

| Event (2026-10-11, −04:00) | Detail |
| --- | --- |
| Session start | Both grant files present, each equal to its `source:sha256` token. CPU-only planning, fixture generation and tests before any child |
| 10:26:43.850 | First child (baseline `init`) |
| 10:27:58–10:27:59 | 4 baseline frames |
| ~10:28 | **Foreign processes (not mine).** A name-based `pgrep incant_headless` also listed a `cargo fmt/test/clippy` chain from another worktree (`wabt-download-retry`; PIDs 34553, 34567, 34714). It was gone by 10:29:32. Astra's note confirms it was CPU-only and failed before gameplay, with no renderer, capture or GPU overlap. My own children are identified by the full frozen-binary path |
| 10:28:42–10:28:43 | 3 candidate frames on the retained cache |
| 10:30:56–10:30:57 | 4 recooked frames |
| 10:32:43.561 | Last child (source-free frame) exits |
| 10:33:14.187 | `captures-complete.json` and `RELEASED` written; Astra told in session output. Grants still present then (removing them was Astra's job) |
| 10:33:35 | Astra's `coordination.txt`: both grants removed. No engine process since; analysis, packaging and this report are CPU-only |

Other notes:
- **No other slot exceptions.** No child launched without its grant, no capture failed and
  nothing ran after release.
- **Runner fix before any engine run.** My helper test caught a defect in my own runner:
  the ledger path was bound at definition time, which would have let the frame cap read a
  stale path if it was ever relocated. I fixed it before the first engine child.
- **Analysis correction.** My first environment confinement check used approximate
  sphere discs and flagged 266 rim pixels just outside them. Exact per-pixel ray–sphere
  silhouettes place all 43,414 changed pixels inside the spheres. The committed analysis
  uses the exact test.

## Remaining limits

- **Host and output.** One Mac and adapter (Apple M5 Pro), headless authoring-preview
  `screenshot`, `Rgba8UnormSrgb` output only. No phone, shipped-player, GPU-memory or
  performance evidence. No phase gate.
- **Formats not rendered.**
  - glTF only. FBX role bindings (per ufbx texture × role) were not rendered here; they
    rest on Astra's CPU/GPU tests. No full FBX claim.
  - 8-bit power-of-two sources only (64×64, 1024×512). HDR/EXR, 16-bit, odd and thin
    extents are covered by Astra's numerical tests, not by these frames.
- **Environment coverage.** One Color environment size, where decode uses source mip 1.
  Larger sources (mip 2–3) and an alpha-carrying Linear environment were not rendered;
  the Linear path's code is unchanged.
- **Sprites and filtering.** Sprites sample level 0 only, by design, so the mip-policy
  change cannot alter them; the check confirms they still use the primary chain.
  Trilinear filtering without anisotropy blurs the oblique view equally on both builds.
- **Existing projects.** Projects cooked before this change keep the darkening until each
  source is present and reimported through the command path. A source-free project
  cannot be repaired. This delta exercised that migration on the same projects; it did
  not test Undo/Redo across the recook.

## Changed paths (new files only)

- `handoffs/0043-color-mip-roles/result.md`
- `tools/`:
  - `fixtures.py`: CPU-only art, glTF and scene specs, framing;
  - `delta.py`: guarded runner, frame cap, timeline, release lock;
  - `analyse.py`: CPU numeric evidence and DERIVED images;
  - `package.py`: copies frames and evidence;
  - `test_delta.py`: CPU-only helper tests.
- `evidence/`:
  - analysis: `analysis.json`, `screenshots.sha256`;
  - ledgers: `ledger.jsonl` (56 children), `captures-complete.json`, `timeline.jsonl`;
  - build record: `fixtures.json` (build, recook, strip and validate record with source
    and cache hashes);
  - notes: `observations.jsonl`, plus Astra's `astra-coordination.txt` and
    `acp-verification.json`, copied verbatim.
- `screenshots/engine/` (7 unedited frames; the duplicates stand for the byte-identical
  states listed in `screenshots.sha256`):
  - `old-materials-board.png`, `recooked-materials-board.png`;
  - `old-materials-oblique.png`, `recooked-materials-oblique.png`;
  - `old-environment-view.png`, `recooked-environment-view.png`;
  - `envcontrol-view.png`.
- `screenshots/derived/` (labelled DERIVED):
  - `DERIVED-board-retained-vs-recooked-diff.png`,
    `DERIVED-oblique-retained-vs-recooked-diff.png` and
    `DERIVED-environment-old-vs-recooked-diff.png`: red marks changed pixels over a dimmed
    frame;
  - `DERIVED-board-minified-cells-x3.png`: columns O-mixed, O-zero, O-one, MASK, BLEND,
    emissive OPAQUE zero, emissive BLEND zero; rows k2 baseline, k2 candidate, k6
    baseline, k6 candidate.

**Unchanged:** `brief.md`, `references/*`, `tools/engine.py`, `tools/test_guard.py`, all
0040/0041 evidence, both binaries, and all production code, styles and shaders.

**Ignored, not committed:** `artifacts/0043-color-mip-roles/` holds the projects and
caches (old and new blobs), 13 raw frames, the full ledger and the lock.

**Scratch:** previews in `/tmp/claude-0043-scratch` (outside the worktree) are mine, and I
remove them at the end.

## Commands run (worktree root)

```sh
python3 -I -B -c "...unittest test_guard..."                   # Astra's guard tests: 2 OK (CPU)
python3 -I -B handoffs/0043-color-mip-roles/tools/test_delta.py # 6 OK (CPU)
python3 -I -B handoffs/0043-color-mip-roles/tools/fixtures.py   # framing: all content in frame
T=handoffs/0043-color-mip-roles/tools
python3 -I -B $T/delta.py build                                  # baseline, 25 CPU children
python3 -I -B $T/delta.py capture old materials/board materials/oblique environment/view envcontrol/view
python3 -I -B $T/delta.py capture retained materials/board materials/oblique environment/view
python3 -I -B $T/delta.py recook                                 # candidate, 13 imports + 3 validates
python3 -I -B $T/delta.py capture recooked materials/board materials/oblique environment/view envcontrol/view
python3 -I -B $T/delta.py strip materials && python3 -I -B $T/delta.py validate materials
python3 -I -B $T/delta.py strip environment && python3 -I -B $T/delta.py validate environment
python3 -I -B $T/delta.py capture sourcefree materials/board environment/view
python3 -I -B $T/delta.py release                                # captures-complete + lock
python3 -I -B $T/analyse.py && python3 -I -B $T/package.py       # CPU only
cd handoffs/0043-color-mip-roles && sed 's/  (.*//' evidence/screenshots.sha256 | shasum -a 256 -c -   # 11 OK
```

Every step succeeded. **Not run:** Cargo, Rust tests, clippy, the GPU numerical suite
(Astra's), FBX imports, any UI and any performance measurement.

## Open questions

None blocking. Should the editor flag projects whose retained model or texture versions
predate color roles (INCMOD01/02, Color manifest v1/v2)? Their alpha-carrying OPAQUE,
emissive or environment images keep the old darkening until each source is reimported.
