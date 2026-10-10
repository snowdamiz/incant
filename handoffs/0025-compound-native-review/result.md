# 0025 — Compound native Inspector and corrected-runtime review: result

**Status: SCOPED NATIVE APPEARANCE ACCEPTED. PR #25 may leave draft for this scope.** The final native captures (source `5b97ecd`, app SHA `f23c403f…5224`) confirm the compact unavailable-Agent layout and every required valid-data Inspector state at 1440×874 and 1000×650. No remaining UI defect was found.

- **Native Inspector:** accepted for the real states Astra captured at normal (1440×874) and minimum (1000×650) window sizes:
  - a valid mixed compound;
  - the valid 64-part list (bounded, scrolling, focus);
  - End then Tab to the full ID;
  - exact paths;
  - scrolled fields;
  - primitive regression.
- **Corrected-runtime frames and numbers:** accepted. That review is complete and was not repeated.
- **Layout fix:** the crowded minimum-size Inspector is fixed. When the agent cannot run, the Agent pane takes a compact default height. This is verified in the browser fixture and in the final native pixels.
- **Not claimed:** this is not a phase-gate approval or a complete-engine claim.

## Final verdict (2026-10-10)

**Accepted for the scoped native Inspector review**, at normal 1440×874 and minimum 1000×650. The accepted states are:

- the mixed sphere/box compound;
- exact part paths;
- readable full IDs;
- keyboard End, then Tab;
- the valid 64-part list (bounded, scrolling, focus);
- primitive regression;
- the compact unavailable-Agent allocation.

This verdict rests on Astra's real CUA captures plus the earlier browser-fixture hostile-data results and behaviour tests.

**Not claimed:**

- Native reorder and native invalid-data display. These are outside the read-only/rejection boundary and are covered by tests instead.
- Traffic-light alignment, because the OS pill hides it.
- A phase gate or complete-engine status.

## Model and transport

- Model: **Claude Opus 5.5** (`claude-opus-5-5`) in Claude Code (Claude Agent SDK). The handoff runner provides the ACP transport and the director's subscription; I cannot verify the transport from inside the session.
- No other model was substituted.

## Priority revisions and feedback acknowledged

- **Final native review supplied (latest revision):**
  - I read `artifacts/0025-native-final/manifest.json` and `notes.md`.
  - All nine JPEG SHA-256 values match the manifest.
  - The rebuilt app is source `5b97ecd`, SHA `f23c403fceb9bca3418bd0dc317e9e316c808fe665eb9fc8fb99748afabf5224`, project `a945aeee…6bd419`.
  - `5b97ecd` contains this change as `19c3be9`, and its `editor/ui/src` is byte-identical to this branch's `6a9c832` (`git diff --quiet`).
  - I did not repeat playback or Cargo.
  - On resuming, a cherry-pick of Astra's `9c84af0` (packet update) stood in conflict on `brief.md`. It was completed as `3b96ee8` with Astra's version, which is the current packet. I made no other change to it.
- **Follow-up (Astra, under the director's standing request to keep improving small UI issues):** the minimum-size Inspector was crowded while the Agent is unavailable. I applied a layout fix using my own design judgment (below), keeping resizable panels and the existing layout. The agent runtime, the project mutation boundary and the titlebar are untouched.
- **Manifest and notes read.** I used `artifacts/0025-native/manifest.json` and `notes.md`, with the `.jpg` names. The earlier `.png` files are identical, mislabelled copies, and they stay ignored.
- **Read-only boundaries recorded, not requested again:**
  - **Reorder:** the read-only Inspector has no reorder control. Stable selection across reorders is covered by the `eb261ab` behaviour test.
  - **Invalid data:** native code rejects invalid projects, so it never shows a field-level invalid snapshot. Malformed nested data is covered by the browser fixture (32 captures) and by schema and command rejection tests.
- **Traffic lights:** the macOS screen-sharing indicator covers the traffic-light pixels during CUA capture. This is an OS limitation, not an app defect, and titlebar styling is unchanged in this increment.
- **Earlier feedback:** computer use and capture are reauthorized. Native interaction is by Astra through CUA. Validation is on `64b9efe`. Connected panels, the separate Assets workspace and the Problems/Console/History dock are preserved.

## UI improvement: compact Agent pane while the agent cannot run

**Defect.** At the minimum window size the side column gave the Agent its normal default of `clamp(0.33·h, 260, 340)`, which is 260 px at 650 tall, even when the pane could only say why it is unavailable. The Inspector kept 321 px, of which about 285 px was scrollable. In the native 64-part minimum capture, the part card's ID row was cut off at the Agent boundary.

**Change** (`editor/ui/src/shell/layout.ts`, `editor/ui/src/App.tsx`):

- **New `idleAgentHeight(h)`:** returns `clamp(0.24·h, 200, 240)`. It is bounded like `clampLayout`, so it is never taller than the regular agent default.
- **When it applies:** while `snapshot.agent.status` is anything other than `idle` or `running` (signed out, checking, error, not ready, or no engine), the side column uses `min(layout.agent, idleAgentHeight)`.
- **When it stops applying:**
  - The first time someone moves the Agent separator (pointer or keyboard), their size wins, even while the agent is unavailable.
  - A ready agent (`idle` or `running`) gets the normal default size again.
- **Why 200 px:** that is the measured height that fits the compact empty state (title plus reason, with no mark below 160 px of transcript) and the one-line disabled composer, including the "Continue with ChatGPT" sign-in button.
- **Why there was no CSS change:** the compact empty state from `6cb489a` already handles this height.

**Measured in the browser fixture** (headless Chrome 155, `tools/measure-side-column.mjs`, Rubble Pile selected; 16 captures):

| Window | Agent before → after | Inspector before → after | Transcript overflow |
|---|---|---|---|
| 1000×650 | 260 → **200** | 321 → **381** (+19%) | none, in all 4 provider states |
| 1440×874 | 288 → **210** | n/a → **595** | none |
| 1440×900 | 297 → **216** | 534 → **615** (+15%) | none |
| 1920×1080 | 340 → **240** | n/a → **771** | none |

- **Native reason text:** I also checked it, substituting the exact native wording ("Use the headless agent command for the Phase 0 provider spike. …"). It fits without scrolling: 82 of 82 px at 650 tall, and 92 of 92 px at 874 tall. The transcript stays non-tabbable because nothing overflows.
- **Inspector regression re-run** (`tools/capture-inspector.mjs` → `artifacts/0025-ui-r2`, 32 captures):
  - 0 axe violations, 0 page errors, 0 remote requests, 0 clipped inputs, 0 px horizontal overflow.
  - Exactly one tabbable option per list.
  - Focus stays visible for End and End then Tab at both sizes.
- **Result at 1000×650:** with part 64 selected, the card's ID and Offset now show above the Agent pane (`screenshots/agent-idle/before-…` vs `after-…`).

**Tests added:**

- `layout.test.ts`: idle heights at 650, 874, 1080 and 1440 px tall, each at least 200 and below the regular default.
- `App.test.tsx`:
  - an unavailable agent starts compact;
  - a keyboard separator move keeps the user's size (+16);
  - a ready agent with `agent.send` gets the full default.

**Native confirmation (final captures, `artifacts/0025-native-final`):**

These are CUA JPEG bytes at 2× Retina scale. Committed crops of the Inspector and Agent column are in `screenshots/native-final/*-side.png`. They are cropped below the titlebar with `sips` and re-encoded as PNG with no other edits, and they contain no account identity.

| Capture | Size | Verdict |
|---|---|---|
| `parts-64-min-end` | 1000×650 | **Pass.** Row 64 is focused and fully inside the visible Inspector, with a list scrollbar. The Agent pane measures about 200 pt from the pixels (manifest: 200 px). The Inspector is about 381 pt, up from about 321 pt before the change. |
| `parts-64-min-end-tab` | 1000×650 | **Pass. This is the defect state that is now fixed.** The card shows `Part 64 of 64` and `/shape/parts/63`. The full ID `…1063` sits in its focus ring, and Offset plus the 2×2 Rotation are all visible above the Agent pane. Before the change, the card was cut off at the ID row. |
| `mixed-min-end-tab` | 1000×650 | **Pass.** `3 parts · 2 sphere, 1 box`. Row 3 is selected and distinct from the hovered row 1. Caption `/shape/parts/2`. The full ID `…112` is visible and selected in its ring at the boundary. |
| `primitive-min-top` | 1000×650 | **Pass.** Floor shows `box`, half extents 10 / 0.5 / 10, Material, then Collision. Primitive presentation is unchanged. |
| `parts-64-wide-end`, `parts-64-wide-end-tab` | 1440×874 | **Pass.** About 6.5 rows in a bounded well. The row ring hands off to the ID ring. `/shape/parts/63`. ID, Offset and the first Rotation row show. Agent pane about 210 pt (manifest: 210 px). |
| `mixed-wide-end-tab` | 1440×874 | **Pass.** The card shows down to Primitive and Half extents. |
| `primitive-wide-top` | 1440×874 | **Pass.** The whole Collider (Material, Collision and the mask bars) plus the start of Transform now fit above the Agent pane. |
| Agent pane | both | **Pass.** The native three-line reason ("Use the headless agent command for the Phase 0 provider spike…") is fully legible in the compact layout (no mark), with the disabled composer below. Nothing clips or overlaps. |
| Shell | both | **Pass.** Connected neutral panels, the Hierarchy/Assets tabs, the Problems/Console/History dock and the Read-only chip are unchanged. The titlebar is unchanged; traffic lights are hidden by the OS screen-share pill. |

`resize-observation.jpg` has the same bytes as `primitive-min-top.jpg` (SHA `89e620e5…`).

## Native review (Astra's pixels, from source `64b9efe`, before this change)

Provenance: native app `9ce81f7e…bca5980`, project `a945aeee…6bd419`. These are CUA JPEG bytes at 2× Retina scale, unchanged; the SHA-256 of every file is in the manifest.

**Limitations of the JPEGs:**

- They are lossy, so there is no pixel-exact or colour-exact claim. Fine anti-aliasing and the 1 px separators are judged only approximately.
- They show the director's real account email, so the originals stay ignored. The committed `screenshots/native/*-inspector.png` files are Inspector-column crops (sips crop to PNG, with no other edits) that exclude the account chip.
- `primitive-min-top.jpg` is 1000×873 logical: minimum width, not minimum height.

| Capture | Verdict |
|---|---|
| `mixed-default`, `mixed-end-tab` (1440×874) | **Pass.** `3 parts · 2 sphere, 1 box`. Card shows `/shape/parts/0` and then `/shape/parts/2`. End selects row 3, which is visibly different from the hover tint. Tab gives a full-width ring around the full ID ending `112`. |
| `mixed-min` (1000×650) | **Pass.** The Inspector scrolls inside its own panel. The card and the 2×2 quaternion are legible. |
| `parts-64-default` (1440×874) | **Pass.** The list is bounded at about 6.5 rows, with the cut-off row as a scroll cue. The summary truncates; the full text is in its tooltip and accessible description. |
| `parts-64-min-end`, `parts-64-min-end-tab` (1000×650) | **Pass.** Row 64 is focused with its ring inside the well, and a scrollbar is visible. `/shape/parts/63`. The ID ending `1063` is fully readable, and `-ax.txt` confirms it is the selected text. This was the crowding that motivated the change above. |
| `parts-64-wide-end-tab`, `-detail-scrolled` (1440×873) | **Pass.** Half extents, Material, Collision and the mask bars render, with no overflow. |
| `primitive-wide-top` (1440×873), `primitive-min-top` (1000×873) | **Pass (primitive regression).** Floor shows `box` with half extents 10 / 0.5 / 10 and the Material/Collision groups, unchanged from earlier primitive presentation. "Half extents m" wraps to two lines at the default width, which is acceptable and pre-existing. |
| `primitive-min`, `resize-observation` (1000×650) | **Pass.** These are scrolled views; the scroll was Astra's action (see the manifest). |
| Shell | **Pass.** Connected neutral panels, the Hierarchy/Assets tabs, and a dock with only Problems/Console/History. Traffic lights are not reviewable because of the OS pill. |
| Viewport | **Out of scope.** The native fixture's entities draw as default cubes, because there is no compound render mesh. |

The browser-fixture hostile-data evidence from the first checkpoint still stands: 32 captures with 0 axe violations, 0 clipped inputs and 0 overflow (`screenshots/browser/`).

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

## Changed paths (this increment)

- `editor/ui/src/shell/layout.ts`: adds `IDLE_AGENT_MIN` and `idleAgentHeight`.
- `editor/ui/src/App.tsx`: the side column uses the idle Agent height until the separator is moved or the agent is ready; adds `windowHeight` state.
- `editor/ui/src/shell/layout.test.ts` and `editor/ui/src/App.test.tsx`: new tests.
- `handoffs/0025-compound-native-review/tools/measure-side-column.mjs` (new): browser measurement of the side column. It refuses an existing output directory.
- `handoffs/0025-compound-native-review/screenshots/agent-idle/`:
  - before and after full-window views at 1000×650;
  - four side-column crops;
  - `side-report.json` and `inspector-report-r2.json`.
- `handoffs/0025-compound-native-review/screenshots/native/primitive-{wide,min}-top-inspector.png`: account-free crops.
- `handoffs/0025-compound-native-review/result.md`.
- `handoffs/0025-compound-native-review/screenshots/native-final/` (final review, this commit): 8 account-free crops of the Inspector+Agent column:
  - `mixed-min-end-tab`
  - `parts-64-min-end`
  - `parts-64-min-end-tab`
  - `primitive-min-top`
  - `mixed-wide-end-tab`
  - `parts-64-wide-end`
  - `parts-64-wide-end-tab`
  - `primitive-wide-top`
- Earlier commits on this branch hold the browser and engine evidence and the other native crops: `caf019d` and `df6a584`. The UI change is `6a9c832`.

## Commands and results

```sh
cd editor/ui && npx tsc -b --noEmit     # strict: pass
npx vitest run                          # 15 files, 334 tests: pass (329 + 5 new)
npm run build                           # pass
node handoffs/0025-compound-native-review/tools/capture-inspector.mjs artifacts/0025-ui-r2       # 32 captures, all checks clean
node handoffs/0025-compound-native-review/tools/measure-side-column.mjs artifacts/0025-side-r2   # 16 captures, no transcript overflow
```

For the final review I re-ran `tsc -b --noEmit` (pass) and `vitest run` (15 files, 334 tests pass) on HEAD. I ran no native automation, no Cargo and no playback, and I did not repeat the runtime look-dev runs.

## Requests for Astra

1. **No further native captures are needed for this scope.**
2. **Nonvisual correction:** none required.

## Limitations

- The native evidence is lossy CUA JPEG (2× Retina), so there is no pixel-exact or colour-exact claim; 1 px separators and anti-aliasing are judged only approximately.
- The macOS screen-sharing pill hides the traffic lights in every native capture, so their alignment is not reviewed. This is an OS capture limitation, and titlebar styling is unchanged.
- The full native images contain account identity. They stay ignored, and only account-free crops are committed.
- Reorder and invalid-data presentation are covered by behaviour, fixture and command tests, not native captures, because of the read-only and rejection boundaries.
- The idle state is per session and is not persisted. Moving the separator once opts out until reload.
- Determinism of the runtime frames is local to one host only.
