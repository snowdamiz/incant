# 0073 R4 bounded rendered execution and pixel review

**Status:** the run was accepted and I reviewed the pixels, but **overall R4 appearance is REJECTED.**

- **Execution:** the one granted run completed all five groups. All 15 PNGs were produced and inspected in full. The GPU lease was released before I opened any image.
- **Stationary OWN:** close to FULL. This is a clear improvement over every prior run.
- **Moving content:** both UP and OWN fall back to an aliased current-only result that is worse than prior expectations for textures.
- **Ordinary UP:** remains poor even when stationary.

No gate is closed by this run.

**Model and settings:**
- Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max. Per the packet, root verified both through ACP before prompting.
- Auto-memory was disabled (`CLAUDE_CODE_DISABLE_AUTO_MEMORY=1`), and no memory or preferences were read or written.
- The bound was 15 minutes. I started at 22:07:35 local (monotonic 123891.57), and the run took 22:07:43–22:07:53.

## Execution evidence

**Identities:** each was verified by sha256 before running:

| Item | sha256 |
|---|---|
| Runner | `2a991b3a…af6c` |
| Plan | `plan-ready.json` `9792295b…c3ba` (`FROZEN_READY_FOR_ROOT_GRANT`) |
| Binary | `9c3a458c…fa59` (mode 0555) |
| Grant | `397f5a3a…c9c4` |
| Fixture | `650316a2…997` |
| Lifecycle | `b9e12895…ff34` |

**Predecessor release:** `reconstruction-r4-numeric-002/release.json`, sha256 `114f9cf5…91eb`.

**Commands:**
- `runner.py check plan-ready.json` once.
- The single granted `runner.py run plan-ready.json`, which exited 0.
  - stdout: `0073-r4-quality-supervision-001/runner.stdout`
  - stderr: `runner.stderr`, 0 bytes

There were no other GPU commands, builds, probes or retries.

**Children and adapter:**

| Group | PID | Accepted |
|---|---|---|
| stationary | 54054 | yes |
| motion | 54064 | yes |
| transitions | 54067 | yes |
| c7_disocclusion | 54071 | yes |
| p1_crossing | 54073 | yes |

- No child timed out.
- The adapter for every child was Apple M5 Pro (Metal, integrated GPU, max 2D texture 8192), on macOS (Darwin 27.0.0, arm64).

**Budget:** 5 processes, 380 scene + 18 UI + 380 readback submissions (778 total), 380 maps, 15 PNGs at 960×540, 0 builds, 0 retries. The runner bound these exactly.
- Ledger: `92683b61…c9e5`
- Debits: `1de2dc0b…3f3d`

**Release:** `released: true` at 2026-10-14T02:07:53Z (`release.json`), with no owned processes and no process groups still alive. The lease ended before image review.

**PNG identities** (all 960×540; full paths in `execution-result.json`):

| Group | File | sha256 |
|---|---|---|
| stationary | `stationary-full.png` | `992c5543…ceb9` |
| stationary | `stationary-drs50.png` | `724fa644…40e0` |
| stationary | `stationary-up50.png` | `e1156008…c503` |
| stationary | `stationary-own50.png` | `57b53ca3…7d8` |
| stationary | `stationary-phases.png` | `bdfeb4db…f245` |
| stationary | `stationary-detail.png` | `37d599a0…980d` |
| motion | `motion-full.png` | `9eeb1222…5b52` |
| motion | `motion-up50.png` | `af438cfc…a5eb` |
| motion | `motion-own50.png` | `8c035e2c…077d` |
| motion | `motion-detail.png` | `ff40c93f…82fc` |
| transitions | `transitions-full.png` | `1be9b930…f88e` |
| transitions | `transitions-strip.png` | `20ac066c…13d8` |
| c7_disocclusion | `c7-detail.png` | `57a437a0…8882` |
| c7_disocclusion | `c7-own-moving.png` | `a0251bb9…7e2a` |
| p1_crossing | `p1-detail.png` | `5f9a22f3…36e1` |

## Pixel review: all 15 images inspected in full

### Stationary: OWN much improved and near FULL; UP rejected

**`stationary-own50.png` vs `stationary-full.png`:**
- **Wires:** all three horizontal wires and the diagonal are present and continuous. The lowest wire is slightly fainter.
- **Pole and texture card:** the pole is straight. The fine stripes on the card (top-left, around x 620–690, y 180–245), the checker and the ring targets resolve almost like FULL.
- **Sphere and shadow:** the sphere silhouette is smooth and the shadow matches.
- **Detail crop (`stationary-detail.png`, column 4):** this confirms the wires and the card stripes and checker. The OWN wires have a faint beaded texture, and the card-edge hatch is slightly softer than FULL.

This is far better than DRS 8/16 and better than every prior run, where accumulated texture detail was worse than nearest-neighbour DRS.

**Phase stability (`stationary-phases.png`, rows 3–4, OWN frames 17–24):** all eight phases show the same complete wire set. The phase-dependent thin geometry is resolved for stationary OWN in this fixture.

**`stationary-up50.png` (ordinary UP): rejected.**
- **Wires:** only the top horizontal wire survives, and the diagonal is dotted.
- **Pole and silhouettes:** the pole and the fence posts are strongly stair-stepped, and the sphere's lower-right silhouette is jagged.
- **Texture card:** the fine stripes alias into a coarse diagonal moiré, a false pattern that FULL does not have and DRS does not have either (DRS flattens them to grey). See detail row 3, column 3.
- **Phases:** `stationary-phases.png` rows 1–2 show the UP wires jumping position and dropping out from phase to phase. The phase-dependent thin-geometry failure remains in UP.

**`stationary-drs50.png`:** loses the horizontal wires, flattens the stripes, and has blocky edges.

**Verdict:** stationary OWN is *usable reduced-resolution quality, close to FULL* (improved). Stationary UP *fails*: it aliases worse than DRS on texture.

### Motion and disocclusion: REJECTED for both UP and OWN

**`motion-up50.png` and `motion-own50.png`:** these are nearly identical. The moving OWN falls back to the same current-only reconstruction as UP:
- one horizontal wire, plus a dotted diagonal;
- a jagged pole;
- strong moiré on the card stripes (x 600–670, y 180–245) and a noisy checker.

**`motion-detail.png`:**
- **Row 1 (UP mover frames 9, 11 and 14):** a heavily aliased texture surrounds the red cube, while FULL frame 14 is crisp.
- **Row 2 (OWN):** the same as UP.
- **Row 3:** the fence posts are broken and jagged in both.
- **Row 4:** the |UP−FULL| and |OWN−FULL| ×4 tiles show errors along every post and crossbar.
- **Leading and trailing edges:** neither shows a gross ghost trail.

**`c7-detail.png` and `c7-own-moving.png`:**
- **Trailing edge:** the vacated region shows no ghost trail. The ×4 differences behind it show only a thin band of speckle at the cell edges (rows 3–4, near x 40–65, 280–330 and 520–590).
- **Background:** the UP/OWN moving and reference checkers are soft (blurred cell edges) compared with FULL.

**Rendered C7 observation:** worst column is UP 0.0268 and OWN 0.0206, against the quoted 0.06. This is 8-bit sRGB against the same-chronology reference. It is an observation only; the historical linear-model C7 failure (0.0918) remains open.

**Verdict:**
- Disocclusion is clean, with no ghosting. This improves on prior failures.
- Moving texture and thin-feature quality *fails*. With the motion-coherence fallback, the negative LOD bias turns current-only frames into moiré and stair-stepping, and that is worse than DRS's blur for textures.

### DRS and transitions: REJECTED, with partial improvement

**`transitions-strip.png`:**
- **Rows 1–2 (UP and OWN):** frame 14, the first frame at 4/16, shows a large blocky dark diagonal band and noise on the stripes. That is a severe one-frame transition artefact. The third and fourth 16/16 frames (frames 19–20) recover fine stripes in both, so recovery is fast.
- **Row 3, columns 3–4 (first frame after the cut):** smooth diagonal moiré bands replace the stripes.
- **Row 4, frame 10:** OWN (column 4) keeps cross-hatched stripes near FULL. UP (column 3) shows low-frequency moiré bands. DRS (column 2) is flat grey.

**Shadows:** in `transitions-full.png` and the stationary frames, no shadow pop or cascade jump is visible. However, the fixture's views offer only limited shadow comparison, so the DRS-stable guard cannot be confirmed visually.

**Rendered C9 observations (sRGB, finite matched-cycle reference):**
- UP: first_4 not met, fourth_16 not met, third_16 met.
- OWN: none of the three met.

These are observations only. The historical C9 model failures remain open.

**Verdict:** the DRS wipe is *not resolved*. The 4/16 entry frame and the post-cut frame show strong artefacts, although recovery at 16/16 is fast.

### Overlays: PASS, preserved

In every full frame, the following are crisp and identical across FULL, UP and OWN:
- the sprites (y 415–475);
- the world text `ffi سلام 中文` and `Ag`;
- the BLEND card (the orange translucent box);
- the physical UI panel `Upscale 50%`;
- the `physical UI 960x540` label.

The sprite crop (`stationary-detail.png` row 4) matches FULL in UP and OWN, while DRS shows blockier corners. The earlier accepted output-space sprite, text and BLEND route therefore has no regression.

### P1 crossing: unresolved, observations only

**Image (`p1-detail.png`):** the panel checker is soft in UP and OWN compared with the FULL column. The ×4 differences are mostly black, apart from bar markers and faint rectangle outlines at the panel edges in the OWN row (columns 2–3).

**Rendered residual at the stop:** the interior worst is 0 for both UP and OWN. The 6-px margin worst is 0.20–0.55 for OWN (step 19: 0.549) and 0.0013–0.20 for UP. This covers grey 8/16 at one stop only. P1 remains unresolved.

### Overall: REJECTED

**Improved over prior failures:**
- Stationary OWN detail, wires and phase stability are close to FULL.
- Disocclusion has no ghost trails.
- Overlays have no regression.

**Remaining defects:**
- Ordinary UP aliases (moiré) and loses wires even when stationary.
- Moving UP and OWN are current-only and aliased: texture moiré, broken posts and wires, jagged silhouettes.
- The 4/16 DRS entry and post-cut frames show strong artefacts.
- Moving content is soft or aliased relative to FULL.
- C7 and C9 (historical definitions) and P1 remain open.

**Likely contributors** (stated as hypotheses only; no repair was made):
- the negative LOD bias when history is not reused;
- the strict 1/256-pixel motion-coherence admission;
- the ordinary path's single-surface requirement.

**Historical items:**
- GPU001 case 3 is a retained historical failed attempt. Corrected numeric-002 later passed, so it is not a current kernel failure.
- Overall appearance remains open.
- The 8-phase / 24-frame schedule and the original numeric thresholds are unchanged.

**Not covered:** no performance, phone, memory or timing claim is made. Cooked-temporal and native MetalFX were not covered.

**Screenshots:** the 15 runner PNGs listed above are under `/Users/andreyyurlov/.codex/incant-evidence/0073-r4-quality-gpu-001/*/artifacts/`. No extra images, crops or video were made.
