# 0073 R5 numeric + quality execution and pixel review

**Status:** R5 numeric checks PASS. R5 appearance is **REJECTED**, with no practical visual change from R4 in any failing area.

- **Execution:** the one granted run completed all nine children and was accepted. The four strict numeric GPU tests passed before any quality child launched.
- **Images:** all 15 current PNGs were inspected in full.
- **Outcome:** the three R5 changes (displacement-based reaction, r/16 motion slack, −1.5 LOD floor) produced no visible improvement in the failures they targeted. Stationary OWN quality and the overlays are preserved.

**Model and settings:**
- Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max. Per the packet, root verified both through ACP.
- Auto-memory was disabled by the supervisor. No memory, preferences or credentials were read or written.
- Bound: 15 minutes, starting at 23:10:31 local (monotonic 127667.41).

## Execution

**Identities:** verified by sha256 before running:
- runner `f2ba4c96…d72f`;
- `plan-ready.json` `47133884…5a2f`;
- grant `e981286a…98f8` (the evidence directory held only `grant.json`).

**One process note:**
- My first shell command failed in the shell itself, on an invalid `/dev/fd/3` redirect, before the runner could start.
- The evidence directory still held only `grant.json`, and no runner process existed. That attempt never reached the runner, debited nothing and touched no GPU.
- The single real execution of the granted command started at 23:10:48. It was accepted, released at 2026-10-14T03:11:00.930963Z, and exited 0.

**Children:** all ran on Apple M5 Pro (Metal, integrated). None timed out, and no owned processes or process groups remained alive after release.

| Order | Child | Kind | PID | Result |
|---|---|---|---|---|
| 1 | ordinary-default | numeric | 80611 | `test result: ok. 1 passed; 0 failed` |
| 2 | owner-default | numeric | 80616 | `ok. 1 passed; 0 failed` |
| 3 | ordinary-cooked | numeric | 80619 | `ok. 1 passed; 0 failed` |
| 4 | owner-cooked | numeric | 80628 | `ok. 1 passed; 0 failed` |
| 5 | stationary | quality | 80631 | accepted |
| 6 | motion | quality | 80637 | accepted |
| 7 | transitions | quality | 80639 | accepted |
| 8 | c7_disocclusion | quality | 80643 | accepted |
| 9 | p1_crossing | quality | 80645 | accepted |

**Budget, as debited:**
- 9 processes;
- 840 queue submissions, of which 62 numeric, 380 scene, 18 UI and 380 readback;
- 442 maps;
- 15 PNGs at 960×540;
- 0 builds and 0 retries.

**Hashes:**

| Record | sha256 |
|---|---|
| `release.json` (path below) | `2a3abbff5d6d54a2366448ccea0fd89d4eca617d869926c31a134364116e26a3` |
| Ledger | `fe1c4237…8797` |
| Debits | `2e217855…338a` |
| Predecessor release (R4) | `7d642244…149f` |

Release path: `/Users/andreyyurlov/.codex/incant-evidence/0073-r5-numeric-quality-gpu-001/release.json`.

## Images read

All 15 current files were read in full. The first six below are byte-identical to the R4 files I reviewed earlier, as expected since their code paths are unchanged in R5. I re-read them anyway.

**Byte-identical to R4:**
- `stationary-full` (`992c5543…`)
- `stationary-drs50` (`724fa644…`)
- `stationary-own50` (`57b53ca3…`)
- `stationary-phases` (`bdfeb4db…`)
- `motion-full` (`9eeb1222…`)
- `transitions-full` (`1be9b930…`)

**Changed bytes:**

| File | R5 sha256 | R4 sha256 |
|---|---|---|
| `stationary-up50` | `cb0845ba…` | `e1156008…` |
| `stationary-detail` | `7faa5c49…` | — |
| `motion-up50` | `026ca0d3…` | — |
| `motion-own50` | `81170869…` | — |
| `motion-detail` | `76fc6eed…` | — |
| `transitions-strip` | `ed6c4cde…` | — |
| `c7-detail` | `d4f33691…` | — |
| `c7-own-moving` | `2c63710c…` | — |
| `p1-detail` | `12b3313c…` | — |

## Visual verdicts, R5 against retained R4

### Stationary OWN: preserved

`stationary-own50.png` is identical to R4. All four wires are continuous, the card stripes, checker and rings resolve close to FULL, and the silhouettes are smooth. `stationary-phases.png` rows 3–4 are stable across all eight phases.

### Ordinary UP stationary: still FAIL, no visible change

**`stationary-up50.png`:**
- Only the top horizontal wire survives, and the diagonal is dotted.
- The pole and fence posts are stair-stepped.
- The card stripes still show the same coarse diagonal moiré (around x 620–690, y 180–245), and the checker is noisy.

**`stationary-detail.png`, column 3:** the wires and texture rows look the same as R4.

**`stationary-phases.png`, rows 1–2:** byte-identical to R4. The wires still jump and drop out by phase.

The bytes changed slightly, but the appearance did not.

### Moving UP and OWN: still FAIL, no visible change

**`motion-up50.png` and `motion-own50.png`:** still nearly identical to each other and to R4. They show card-stripe moiré, a noisy checker, a single horizontal wire, a dotted diagonal and a jagged pole.

**`motion-detail.png`:**
- Rows 1–2: the aliasing around the moving cube is unchanged.
- Row 3: the posts are still broken.
- Row 4: the |UP−FULL| and |OWN−FULL| error maps show the same post and crossbar errors.

There is no evidence that the r/16 slack admitted materially more moving history in these views.

### Disocclusion (C7 scene): unchanged

`c7-detail.png` and `c7-own-moving.png` look like R4:
- no ghost trail;
- thin speckle at cell edges behind the trailing edge;
- a softer checker than FULL.

### DRS and transitions: still FAIL, slight reduction only

**`transitions-strip.png`, rows 1–2, column 2 (frame 14, the first 4/16 frame):** the blocky dark diagonal band is still clearly visible in both UP and OWN. It is at most marginally less pronounced than in R4. That is consistent with the −1.5 floor touching only the LOD part, but it is not a practical fix.

**Rest of the strip:**
- The third and fourth 16/16 frames still recover well.
- Row 3, columns 3–4: the first frame after the cut still shows smooth moiré bands.
- Row 4, frame 10: OWN stays near FULL, UP shows moiré, and DRS is flat grey.

The DRS wipe and the post-cut artefact remain open.

### Overlays: PASS, preserved

Sprites, the world text `ffi سلام 中文` and `Ag`, the BLEND card, the physical UI panel and the label are crisp and unchanged in every full frame. The sprite crop in `stationary-detail.png` row 4 matches FULL.

### P1 crossing: unchanged, unresolved

`p1-detail.png` looks like R4: a soft panel checker, a mostly black ×4 residual, and faint panel-edge outlines in the OWN row. The coverage is still grey 8/16 at one stop only.

### Overall: REJECTED

R5 kept everything R4 had gained: stationary OWN, clean disocclusion and the overlays. It produced no practical improvement in:
- ordinary-UP stationary moiré and thin lines;
- moving UP and OWN aliasing;
- the 4/16 entry and post-cut artefacts.

So the three changes did not address the binding causes, and the source-derived hypotheses behind R5 are not supported by these pixels.

**Gates and failures that stay open:**
- C7 and C9 under their historical linear-model and convergence definitions (this run's rendered sRGB and finite-reference values are observations only);
- P1, with its grey 8/16 one-stop qualification;
- the DRS wipe, thin geometry and moving detail.

GPU001 case 3 is retained as a historical failed attempt, later passed by numeric-002. The four numeric passes in this run establish numeric conformance only, not appearance.

## Limitations

- **Not measured:** no performance, timing, phone or memory claims.
- **Not repaired:** no repair, rerun, build or extra image was made in this turn.
- **Scope of judgment:** the visual verdicts come from inspecting the 15 fixture views only.
