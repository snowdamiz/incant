# 0025 — Compound native Inspector and corrected-runtime review: result

**Status: reviewable checkpoint, waiting on Astra's native inputs.** The browser-fixture re-check on the integrated code passes, and I found no UI defect, so no `editor/ui/src` change was made. **Native appearance acceptance is NOT claimed:** `artifacts/0025-native/manifest.json` and its native screenshots were not in this worktree. **No corrected-runtime frames were reviewed:** `artifacts/tools/incant_headless` and `binary.json` were not supplied either (the worktree has no `artifacts/` directory at all). This is not a phase-gate approval or a complete-engine claim.

## Model and transport

- Model: **Claude Opus 5.5** (`claude-opus-5-5`) in Claude Code (Claude Agent SDK). The handoff runner provides the ACP transport and the director's subscription; I cannot verify that transport from inside the session.
- No other model was substituted.

## Director feedback acknowledged

- **Computer use and screen capture are reauthorized (2026-10-10).** This lifts the 0024 pause. I used screen capture only for headless browser-fixture captures. Native desktop interaction and capture stay with Astra through CUA, as the packet requires, so I ran no shell native automation.
- **Validated against the current integrated code** (HEAD `64b9efe`: current main plus Astra's `eb261ab` stable-selection and rotated-child normal corrections), not the old 0024 HEAD.
- **Layout preserved:** clean neutral connected panels, the separate Assets workspace, and a bottom dock with only Problems, Console and History. All 32 captures report exactly those three dock tabs. Titlebar and traffic-light alignment can only be judged from native pixels; that check is pending.
- **Stool:** I make no solver inference, in line with `docs/spikes/compound-colliders.md`.

## Changed paths

- `handoffs/0025-compound-native-review/tools/capture-inspector.mjs`: copied from 0024. Changes: port 4195, plus two new states, `handcart-end-tab-id` (End, then Tab) and `rubble-64-end` (End on 64 parts). It still refuses an existing output directory.
- `handoffs/0025-compound-native-review/screenshots/browser/`: 7 PNGs and `report.json`, copied unchanged from `artifacts/0025-ui-r1`.
- `handoffs/0025-compound-native-review/result.md`.

## Commands and results

```sh
npm ci                                     # pass, 0 vulnerabilities
cd editor/ui && npx tsc -b --noEmit        # strict typecheck: pass, no output
npx vitest run                             # 15 files, 329 tests: pass (includes eb261ab's reorder test)
npm run build                              # pass
node handoffs/0025-compound-native-review/tools/capture-inspector.mjs artifacts/0025-ui-r1
                                           # 32 captures (16 states × 1440×900 and 1000×650), headless Chrome 155.0.8059.40
```

Not run, because the inputs are absent: native review, `shasum` of `incant_headless` against `binary.json`, and engine look-dev frames. I did not use another worktree's Cargo target or binary.

## Browser-fixture evidence (not native WebKit)

These are the built UI in headless Chrome at 2× DPR over the read-only `?fixture=sample` data.

**Across all 32 captures** (`screenshots/browser/report.json`):

| Check | Result |
|---|---|
| axe violations | 0 |
| Page errors | 0 |
| Remote requests | 0 |
| Canvas-measured clipped inputs | 0 |
| Inspector horizontal overflow | 0 px |
| Tabbable options per list | exactly 1, including 64 parts |
| Dock tabs | Problems, Console, History |

**Per-state findings:**

- **Keyboard End, then Tab** (`handcart-end-tab-id`, both sizes): End selects part 4 and the caption reads `Part 4 of 4, path /shape/parts/3`. Tab focuses `…parts_3_id`. The full 26-character ID `01J9ZF1XTR0000000000000X9R` is unclipped at 1000×650, inside the card-width focus-within ring. (The input's own `outline: none` is intended; the ring is on the box.)
- **64 parts** (`rubble-64-*`):
  - The list is bounded at 177 px.
  - End focuses part 64, and the focused row is visible inside the list.
  - After 7 Page Down and 2 Arrow Down presses, part 38 is focused and visible.
  - The list uses one Tab stop.
  - At 1000×650 the shared Inspector/Agent column is short. The list and the start of the card fit, and the rest of the card scrolls in the Inspector's own scroller. That is acceptable, not a defect.
- **Exact paths and diagnostics** (`railing-axis-error`):
  - Part 1 is preselected as the first part with an error.
  - The Z half-extent axis is invalid and shown at its nested path.
  - Unreadable parts show their explicit tags: `no type`, `"cylinder"` and `not an object "post-04"`.
- **Rotated keystone** (`arch-rotated-keystone`): the quaternion card is unclipped at both sizes.
- **Primitive regression** (`crate-primitive`, `post-capsule-masks`): no compound UI appears, and the 0024 presentation is unchanged.
- **Selection after reorder:** the browser fixture is static and cannot reorder parts. This is covered only by the `eb261ab` behaviour test, which keeps the stable part selected and focused through a reordered snapshot. **Native visual confirmation is pending.**
- **Minor observation, no change made:** at 1000×650 the list summary truncates (`64 parts · 39 box, 13 sphere, …`). The full text is in its `title` and in the listbox's `aria-describedby`. It was the same in 0024.

**Retained screenshots** (`screenshots/browser/`):

- `handcart-end-tab-id-1000x650-inspector.png`
- `rubble-64-end-1000x650.png` (full window)
- `rubble-64-scrolled-1440x900-inspector.png`
- `railing-axis-error-1440x900-inspector.png`
- `arch-rotated-keystone-1000x650-inspector.png`
- `crate-primitive-1000x650-inspector.png`
- `post-capsule-masks-1440x900-inspector.png`

All other captures are in the ignored `artifacts/0025-ui-r1`. None of them uses a real account.

## Requests for Astra (precise native states)

Please supply `artifacts/0025-native/manifest.json`. For each capture, record the source SHA, app build, window size and the exact steps. Capture each state at **~1440×900 and at the minimum ~1000×650**, as raw unscaled Retina PNGs:

1. **Shell:** the whole window, to check titlebar and traffic-light alignment, connected panels, Hierarchy/Assets as separate workspaces, and a dock with only Problems/Console/History. Use a signed-out or synthetic account; keep any account-bearing frames ignored.
2. **Handcart:** click the Shape row, Tab, then End. Capture the list and card. Then press Tab and capture again, focused on part 4's full ID with the focus ring visible.
3. **Broken Railing, initial view:** part 1 preselected, the Z half-extent axis invalid with its exact path, the summary row, and the unreadable rows 3, 4 and 6.
4. **Rubble Pile (64 parts):**
   - list focused at rest;
   - after End;
   - after End then 3× Page Up (focused row inside the list, focus ring not clipped by the well);
   - the Inspector scrolled to the 2×2 quaternion;
   - if available, the VoiceOver caption for one row.
5. **Reorder:** using the real 3-part compound, select part 3, then apply a reorder through the normal command path (a shared `incant_cmd` transaction, not a UI-only edit). Capture before and after, showing the same stable ID still selected and focused.
6. **Inspector Problems link:** follow the link for `/shape/parts/3/id` while part 1 is selected. Focus should land on part 4's ID.
7. **Primitive regression:** Crate 01 and Mooring Post.
8. **Stone Arch:** the keystone part, showing the 45° quaternion.

For runtime: supply `artifacts/tools/incant_headless` plus `binary.json` (the SHA-256 and the exact source commit). I will then re-run `handoffs/0024-compound-inspector/tools/compound_lookdev.py` into new `artifacts/0025-lookdev-*` directories (≤128 frames and ≤256 MiB raw each) using:

- `--run overview:overview:4`
- `--run arch:arch:4`
- `--run props:props:4`

I will review the arch opening, the rotated ramp, the falling props and the character motion against the 0024 numeric baseline.

## Defects

**New:** none found in this pass.

**Carried from 0024 and still open for Astra:**
- **Bridge diagnostics:** the native bridge does not emit per-field component diagnostics with JSON pointers, so in the native app exact-path placement is only proven with fixture diagnostics.
- **Provenance:** RPC `command.execute` transactions record origin `user` and actor `editor` even when a tool script sends them.

## Limitations

- **No native pixels were reviewed.** Native appearance acceptance for normal and minimum sizes is open.
- **No corrected-runtime frames were reviewed.**
- Browser fixture results are not native WebKit evidence.
- Selection after reorder is verified only by a jsdom behaviour test.
- Timings and determinism were not re-measured in this pass.
