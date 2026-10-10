# 0025: Compound native Inspector and corrected-runtime review — result

**Status: partial native appearance acceptance. The corrected-runtime look-dev is accepted.**

- **Native (Inspector compound states):** accepted from Astra's native pixels at normal (1440×874) and minimum (1000×650) window sizes. The states are mixed parts, exact paths, full readable IDs, End then Tab, and the 64-part bounded list with scroll and focus.
- **Native, still open:** three items:
  - traffic-light alignment, which a CUA overlay hides;
  - selection after reorder, which was not captured natively;
  - an unambiguous primitive-regression frame.
- **Runtime:** the corrected-runtime frames of the 0024 course were reviewed on the SHA-verified binary. They show no new defect.
- **No changes to `editor/ui/src`:** no UI defect was found.
- **Not claimed:** this is not a phase-gate approval or a complete-engine claim.

## Model and transport

- Model: **Claude Opus 5.5** (`claude-opus-5-5`), in Claude Code (Claude Agent SDK). The handoff runner supplies the ACP transport and the director's subscription; I cannot verify the transport from inside the session.
- No other model was substituted.

## Director feedback acknowledged

- **Computer use and screen capture are reauthorized (2026-10-10).** This supersedes the 0024 pause, and screen work has resumed.
- **Native interaction stayed with Astra.** Astra supplied the native interactions and captures through CUA. I reviewed them with the image reader and ran no shell native automation.
- **Validated on the integrated code:** source `64b9efe` (current main plus `eb261ab` and the rotated-child normal fix). This handoff's commits touch only handoff files.
- **Director's layout preserved and seen in native pixels:**
  - neutral connected panels;
  - Hierarchy and Assets as separate workspace tabs;
  - a bottom dock with only Problems, Console and History.
- **No solver inference from spawn height.** I did not alter the solver or runtime.

## Inputs and provenance

Binary, from `artifacts/tools/binary.json`:

| Item | Value |
|---|---|
| Source | `64b9efe5e7ab0e1bdf464539b65c73a570b37e88` |
| `binary_sha256` | `290d85195933866c5cb4793d0374f6354b8a69397d9743bdd7e557511ebc580e` |
| `shasum -a 256 artifacts/tools/incant_headless` | **matches** |

Native captures:

- Astra's native app, per `binary.json`: `native_sha256` `9ce81f7e…bca5980`, project `a945aeee…6bd419`.
- There are 10 captures in `artifacts/0025-native/`. **`manifest.json` was not supplied**, so I inferred the state names from the file names and pixels.
- The captures are **JPEG data with a `.png` extension** (JFIF, 2× Retina): 2880×1748/1746 for 1440-wide windows and 2000×1300 for 1000×650. They are lossy, not raw.
- **They show the director's real ChatGPT account email in the titlebar, so they stay ignored and uncommitted.**
- Committed derivatives are Inspector-column crops only (`sips` crop, re-encoded as PNG with no other edits), and they exclude the account chip.

Truncated SHA-256 of the originals:

| Capture | SHA-256 (truncated) |
|---|---|
| `mixed-default` | `64e9bb37…` |
| `mixed-end-tab` | `2ff08103…` |
| `mixed-min` | `222504c7…` |
| `parts-64-default` | `1e8a2b0c…` |
| `parts-64-min-end` | `02043384…` |
| `parts-64-min-end-tab` | `f25f422e…` |
| `parts-64-wide-end-tab` | `42499d7e…` |
| `parts-64-wide-detail-scrolled` | `2caf55df…` |
| `primitive-min` | `9a29f4e7…` |
| `resize-observation` | `988817d2…` |

## Native review (Astra's pixels)

| State | Size | Verdict |
|---|---|---|
| Dumbbell mixed parts, default | 1440 | **Pass.** Summary `3 parts · 2 sphere, 1 box`. Compact rows: `r 0.3 m` and `½ 0.7 × 0.1 × 0.1 m`. Card `Part 1 of 3` with `/shape/parts/0`. Full ID `000…0110` readable. |
| Dumbbell End then Tab | 1440 | **Pass.** Row 3 is selected (tinted), and the hovered row 1 is visibly different (neutral grey). Caption `/shape/parts/2`. Tab puts focus on the ID: a full-width lavender ring, all 26 characters selected, and nothing clipped. |
| Dumbbell | 1000×650 | **Pass.** The Inspector scrolls inside its own panel. The list, card, Offset and the 2×2 quaternion are legible. |
| 64 parts, default | 1440 | **Pass.** The well is bounded at about 6.5 rows, and row 7 is cut by the well edge as the scroll cue. The summary truncates (`…21 sphere, 21…`); the full text is in `title` and `aria-describedby`. |
| 64 parts End | 1000×650 | **Pass.** Row 64 is focused with its ring fully inside the well. A list scrollbar is visible. Card `Part 64 of 64` with `/shape/parts/63`. |
| 64 parts End then Tab | 1000×650 and 1440 | **Pass.** The row ring hands off to the ID ring, and row 64 stays selected. The full ID `…1063` is readable. `parts-64-min-end-tab-ax.txt` reports the selected text as the ID. |
| 64 parts, detail scrolled | 1440 | **Pass.** Half extents `0.25 / 0.3 / 0.4`, Material, Collision, and the mask bars render like the primitive masks. There is no horizontal overflow. |
| `resize-observation` | 1000×650 | **Pass, with an observation.** After resizing to the minimum, the Inspector half of the right column is about 255 pt tall, so only list rows 1–2 show before the Agent pane. Everything scrolls and nothing clips or overlaps. The shell's equal Inspector/Agent split is director-owned layout, so I left it unchanged; see Open questions. |
| `primitive-min` (Floor) | 1000×650 | **Inconclusive.** The Inspector is scrolled to Friction/Restitution/Collision/masks, which render correctly. The Shape row is off-screen, and without the manifest I cannot tell whether the scroll offset was carried over from the previous entity or set by Astra on purpose. |
| Shell and titlebar | both | **Panels pass, traffic lights not reviewable.** Panels are connected, the toolbar is centred, the dock is correct, and the Read-only chip is present. A purple CUA/control overlay pill covers the traffic-light area in every capture, so I **cannot confirm traffic-light alignment**. |
| Viewport | both | **Out of scope.** The compound entities draw as the default cube meshes, because this native project has no compound render mesh. That is not an Inspector defect, and there is no collider debug draw. |

The browser-fixture pass from the first checkpoint still stands for hostile malformed data: 32 captures, 0 axe violations, 0 clipped inputs and 0 overflow. See `screenshots/browser/`.

## Corrected-runtime look-dev (SHA-verified binary)

```sh
python3 -I handoffs/0024-compound-inspector/tools/compound_lookdev.py artifacts/0025-lookdev-A \
  --run overview:overview:4 --run arch:arch:4 --run props:props:4
python3 -I handoffs/0024-compound-inspector/tools/compound_lookdev.py artifacts/0025-lookdev-B \
  --run overview:overview:4 --run arch:arch:4 --run props:props:4
```

**Authoring path:** the original 0024 helper and models. The scene is built with `init`, `import` and one RPC `command.execute` transaction through the journal, then `project.save`, `validate`, and `play --compiled-script --camera`. These are real GPU frames; none is painted or substituted.

**Budget:** each run captures 76 frames at 960×540, 157.6 MB raw. Wall time per run was 1.13–1.36 s on this host (Apple M5 Pro), an informal timing.

**Repeatability:** runs A and B produced **228/228 byte-identical frames** and byte-identical logs. Only `report.json` timing differs. This is local only.

Numeric evidence from `props.logs.jsonl` (300 samples) matches the 0024 v2 baseline:

| Check | Measured |
|---|---|
| Arch lane | crosses x≈0.55–0.6 at y 0.82, z 0. The contact count stays 1 (floor only), so the **opening is hollow**. y range 0.819–0.83. |
| Rotated ramp (terrace) | peak y **1.321** (0.5 deck + capsule centre). Final (4.26, 0.82, 2.4). z stays **exactly 2.4** for the whole run, so the 8 µm v1 drift is gone. |
| Wall with −35° return | clamps at z **−2.73**, then is pushed out to **−2.328**. |
| Stool (dropped upright from 2 m) | lands by t40. up·Y 1.000 from t24 to t300, at rest at (3.889, −0.01, −1.201). |
| Dumbbell (released tilted 18°) | up·Y 0.951 at t24, then 0.999 at t40. Rests from t60 at y **0.16** (the sphere radius), level. |

**Visual review:**
- `arch-t156` puts the character inside the opening, with the keystone reading as a 45° ridge.
- The overview at t148 shows the climb up the ramp.
- The props frames show the dumbbell tumbling (t24), then both props landed (t40) and at rest (t300).
- Characters move smoothly, with no visible interpenetration.
- The 0024 cosmetic notes still apply: the landed stool is cropped in the arch camera's foreground, the backdrop edge reads as a dark band, and lighting is a single sun with no HDR.

**New runtime defects:** none.

## Changed paths (this checkpoint)

- `handoffs/0025-compound-native-review/result.md` (rewritten).
- `handoffs/0025-compound-native-review/screenshots/native/`: 7 Inspector-column crops with no account content:
  - `mixed-end-tab`
  - `mixed-min`
  - `parts-64-min-end`
  - `parts-64-min-end-tab`
  - `parts-64-wide-end-tab`
  - `parts-64-wide-detail-scrolled`
  - `resize-observation`
- `handoffs/0025-compound-native-review/screenshots/engine/`: four frames copied unchanged from run A, plus two 2×2 pixel-copy sheets made with `contact_sheet.py`:
  - `arch-t156-through-opening.png`
  - `overview-t148-arch-and-terrace.png`
  - `props-t024-falling.png`
  - `props-t040-landed.png`
  - `sheet-arch-t100-t156-t200-overview-t148.png`
  - `sheet-props-t024-t040-t300-overview-t300.png`
- Committed in the earlier checkpoint `caf019d`: `tools/capture-inspector.mjs` and `screenshots/browser/`.

## Tests

Results on this code; no UI source changed after them:

| Command | Result |
|---|---|
| `npx tsc -b --noEmit` (strict) | pass |
| `npx vitest run` | 15 files, 329 tests pass, including `eb261ab`'s reorder test |
| `npm run build` | pass |

## Remaining requests for Astra

1. **Traffic lights:** a titlebar capture at both sizes with the CUA overlay hidden (or moved), so I can check traffic-light alignment against the titlebar row.
2. **Reorder:** select part 3 of the Dumbbell, then reorder its parts through a shared `incant_cmd` transaction. Capture before and after; the same stable ID should stay selected and focused.
3. **Primitive regression:** a primitive collider (Floor) with the Inspector scrolled to the top, so the Shape and dimension rows are visible. Also confirm whether `primitive-min`'s scroll offset was carried over from the previous selection. If it was, that is a behaviour question for the Inspector: should the scroll reset per entity?
4. **Diagnostics:** native diagnostics at exact nested paths still depend on the bridge emitting per-field component pointers (carried over from 0024). The native project had 0 problems, so native error presentation is unreviewed.
5. **Manifest:** please add `manifest.json` with exact steps per capture, and supply raw PNGs if lossless review is required.

## Open questions

- **Minimum-size Inspector/Agent split:** at the minimum window size, should the Inspector/Agent split favour the Inspector when the Agent pane is in its not-ready empty state? This is a director-owned layout choice, so I made no change.

## Limitations

- Native acceptance covers only the states listed. Traffic lights, reorder, the primitive Shape rows and native diagnostics remain open.
- The native images are lossy JPEG, and there was no manifest.
- Determinism is local to one host only.
- The runtime frames are headless engine output, not native editor viewport evidence.
