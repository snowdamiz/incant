# 0024 — Compound collider Inspector and real-engine look-dev: result

**Status: reviewable checkpoint. Inspector work complete and verified in the browser fixture; real-engine look-dev rendered with the original binary and numerically replayed with v2. Final native appearance acceptance and any new visual review are paused by the director (see "Screen work pause").** Not a phase-gate approval.

## Model and transport

- Model: **Claude Opus 5.5** (`claude-opus-5-5`), running in Claude Code (Claude Agent SDK). The handoff runner supplies the ACP transport and the director's subscription; I could not independently verify the transport from inside the session.
- No other model was substituted.

## Director feedback acknowledged

- **Screen-work pause (latest priority revision):** applied immediately. After it arrived I took no browser, native or headless captures, ran no capture helper, operated no app and ran `play` only without `--output`. All images below were produced **before** the pause, and the engine frames came from the **original** binary.
- The neutral integrated editor is preserved. Assets keeps its own left workspace and asset details stay in the main Inspector. The bottom dock is still only Problems, Console and History. Nothing was added to the dock and no floating panels were introduced.

## What changed

### Inspector (commit 29f50d1, already integrated by Astra)

- **`ObjectList.tsx` (new):** a generic, read-only master/detail view for schema arrays whose items are records. `FieldView` now sends these arrays to it instead of the numeric-vector path. Numeric arrays keep the vector presentation, and other item types show an explicit "Unsupported" notice.
  - **List:** a bounded well of one-line rows, about 6.5 visible before it scrolls. Each row shows the index, the primitive tag and compact values (`½ 1.2 × 0.05 × 0.5 m`, `r 0.18 m`, `½h 0.32 · r 0.04 m`), plus an error/warning marker. The header summarises the parts (`4 parts · 1 box, 2 sphere, 1 capsule`).
  - **Keyboard:** the list is a `listbox` with one Tab stop. Arrow keys, Page Up/Down and Home/End move through it, and selection follows focus. A 64-part collider costs one Tab stop and about 177 px of height.
  - **Detail card:** only the selected part's fields are rendered: ID, Offset, Rotation, then Primitive with its dimensions. The card caption shows `Part 4 of 6` and the exact pointer `/shape/parts/3`.
  - **Initial selection:** the first part with an error, otherwise the first with a warning, otherwise part 1. It resets per entity and is scrolled into view inside the list only.
- **Diagnostics stay at exact paths:**
  - `/shape/parts` itself, or an index past the end of the list, appears on the list row.
  - `/shape/parts/3` exactly appears under the card caption.
  - Field and axis paths (`/shape/parts/0/shape/half_extents/2`) appear on their rows and mark the axis input `aria-invalid`.
  - Row names include error and warning counts.
  - Inspector Problems links now follow through: focusing an unselected part's row selects it, then focus moves on to the exact field.
- **Malformed data is shown, never repaired:** non-object parts, missing or unknown primitive tags (including a nested `"compound"`), missing required fields and fields outside the part schema each get an explicit notice with the raw JSON. A non-list `parts` value gets a "Cannot show as a list" notice.
- **Labels (in `presentation.ts`, profile only; the schema wins):**
  - The part's `shape` is labelled **Primitive**, so it does not share the accessible name "Shape" with the compound's own row.
  - `translation` is labelled **Offset** and `id` is labelled **ID**.
  - Profile keys accept `*` index segments, and the profile sets the authored part order (`id, translation, rotation, shape`), since the schema's object properties are alphabetical.
- **Accessibility:** each row's name comes from its content: one visually-hidden sentence such as "Part 3: capsule, half height 0.32, radius 0.04 m, 2 errors, 1 warning", with the compact visual cells `aria-hidden`. This avoids axe's `label-content-name-mismatch`, which `aria-label` triggered in the first capture round. Chrome's accessibility tree confirms these names (`listAria` in the report).
- **Layout:**
  - Because a compound's only field is its parts list, that variant drops its indent rule, so the list and card get full width.
  - The ID spans the card with its label inside the box, so all 26 characters show at 320 px.
  - In narrow cards, vector rows stack under their label (container query below 360 px). Below 320 px, quaternions wrap to X Y / Z W instead of truncating.
  - Deep entity problem paths now wrap instead of widening the Inspector, which removes 21–32 px of horizontal overflow.
- **Fixture:** the compound variant mirrors the native bridge conversion; the existing drift test checks it. New sample entities:
  - **Stone Arch**: includes a 45° keystone.
  - **Handcart**: mixed parts with masks.
  - **Broken Railing**: one malformed part of each kind, with engine-worded diagnostics at exact paths.
  - **Rubble Pile**: 64 parts.
- **Tests:** `CompoundParts.test.tsx` adds 12 tests against the real `schemas/Collider.schema.json` through `snapshotFromEngine`. They cover:
  - valid mixed parts and authored order;
  - keyboard behaviour (one Tab stop, arrows, Page, Home/End, click);
  - missing and unknown nested tags;
  - non-object parts, missing fields and extra fields;
  - a non-list value;
  - list, part, field and axis diagnostics, including an index past the end;
  - 64 parts with axe;
  - per-entity selection reset;
  - axe on mixed parts;
  - fixture entities in the App;
  - a Problems link landing on the exact field of an unselected part.

  One App test's tree count was updated from 19 to 23 for the new fixture entities.

Astra's later `eb261ab` keeps selection and focus stable when parts are reordered, without styling changes, and adds a behaviour test. That commit is in the integrated parent and not on this branch's older HEAD. I deliberately did not touch `ObjectList.tsx` again, to avoid a conflict.

### This checkpoint commit (handoff files only)

- `tools/compound_lookdev.py`: builds the real-engine review scene. It refuses an existing output directory and enforces the ≤128-frame and ≤256 MiB raw budgets.
- `tools/compound_course.ts` and `tools/tsconfig.json`: the character behaviour script, checked with strict TypeScript.
- `tools/contact_sheet.py`: a stdlib tiler that copies pixels unchanged and refuses an existing output file.
- `tools/capture-inspector.mjs`: committed in 29f50d1. It refuses an existing output directory, scrolls only the Inspector's own scroller (fixing the titlebar crop seen in the 0022 captures), measures clipped inputs with canvas text metrics, and records Chrome's ARIA snapshot.
- `screenshots/` and this `result.md`.

## Commands run

```sh
npm ci                                   # repo root workspace (editor/ui)
cd editor/ui && npx tsc -b --noEmit      # strict UI typecheck: pass
npx vitest run                           # 15 files, 328 tests: pass (includes ../bridge tests)
npm run build                            # pass
npx tsc -p handoffs/0024-compound-inspector/tools/tsconfig.json   # strict course script: pass
shasum -a 256 artifacts/tools/incant_headless      # 37b78728…df4fe79 = binary.json (source 8b7f211 = HEAD at start)
shasum -a 256 artifacts/tools/incant_headless-v2   # 8235a36a…5661de99 = binary-v2.json (source aa2a4d5, fix 68993c8)

# Before the pause (browser fixture, headless Chrome 155.0.8059.40):
node handoffs/0024-compound-inspector/tools/capture-inspector.mjs artifacts/0024-ui-r4
# Before the pause (original binary, real GPU frames):
python3 -I handoffs/0024-compound-inspector/tools/compound_lookdev.py artifacts/0024-lookdev-{A,B,C} \
  --run overview:overview:4 --run arch:arch:4 --run props:props:4
python3 -I handoffs/0024-compound-inspector/tools/compound_lookdev.py artifacts/0024-lookdev-topple \
  --stool 9:0.05 --run props:props:4
# After the pause (numeric only, no --output):
incant_headless{,-v2} play artifacts/0024-lookdev-C/compound.incant.json --seconds 5 \
  --compiled-script artifacts/0024-lookdev-C/compound_course.js --log-output artifacts/0024-v2-numeric/<run>.logs.jsonl
```

Raw runs, logs and projects are kept in the ignored `artifacts/0024-*`.

## Inspector evidence (browser fixture, before the pause)

These are **browser-fixture** captures: the built UI in headless Chrome at 2× DPR over the read-only `?fixture=sample` data. They are not native WebKit evidence. The account labels in the shell capture are synthetic fixture values (`ada@example.com` and similar), not real accounts.

Results across 14 states × 2 sizes (1440×900 and 1000×650), from `screenshots/inspector/report.json`:

| Check | Result |
|---|---|
| axe violations | 0 |
| Page errors | 0 |
| Remote requests | 0 |
| Clipped inputs (canvas-measured) | 0 |
| Inspector horizontal overflow | 0 px |
| Tab from the Shape row | lands on the list in one stop |
| Tabbable options per list | exactly 1 (64-part list included) |

Retained screenshots (`screenshots/inspector/`):

| File | Shows |
|---|---|
| `arch-valid-1440x900.png` | Full window: valid compound and the list/card at default width |
| `handcart-mixed-1000x650-inspector.png` | Mixed box/sphere/capsule, part 4 selected by keyboard (End), stacked vectors |
| `railing-axis-error-1440x900-inspector.png` | First problem preselected, Z axis invalid with exact nested path |
| `railing-rotation-error-1000x650-inspector.png` | Non-unit rotation diagnostic on the Rotation row |
| `railing-missing-tag-1440x900-inspector.png` | "No “type” is set" notice with raw value |
| `railing-unknown-tag-1000x650-inspector.png` | Unknown `"cylinder"` primitive, duplicate-ID diagnostic |
| `railing-not-object-1440x900-inspector.png` | Non-object part shown raw |
| `rubble-64-start-1440x900-inspector.png` | 64 parts, bounded list, summary |
| `rubble-64-scrolled-1000x650.png` | Full window at 1000×650 after Page Down, focused row, 2×2 quaternion |
| `crate-primitive-1440x900-inspector.png` | Primitive Inspector unchanged (regression) |
| `post-capsule-masks-1000x650-inspector.png` | Mask presentation unchanged (regression) |
| `saved-accounts-shell-1440x900.png` | Saved-account shell with a compound selected (synthetic fixture accounts) |

Native CUA checks by Astra before the pause verified:

- the real three-part list;
- End selects part 3, and Tab moves to its full stable ID;
- exact nested paths;
- a healthy engine and saved-account restoration.

**No final native appearance acceptance is claimed.**

## Real-engine look-dev (original binary, before the pause)

**Scene.** All geometry is imported glTF whose vertices are the exact union of each compound's parts, baked at the parts' authored offsets and rotations. Every physics root has unit scale. Building the scene takes:

- 12 `import`-origin journal transactions;
- one 17-command `apply` transaction ("Compound collider look-dev scene", origin `user`, actor `editor`, with before/after snapshots);
- `project.save`, then `validate`.

Compound bodies in the scene:

| Body | Type | Parts |
|---|---|---|
| Arch | fixed | two piers, a lintel and a keystone turned 45° about X. The 1.0 × 1.8 m opening is genuine. |
| Terrace | fixed | a 17° ramp (a rotated local part), a 0.5 m deck and a 0.25 m step |
| L wall | fixed | a long run plus a return turned −35° about Y |
| Falling stool | dynamic | seat plus three capsule legs, released upright from 2 m |
| Falling dumbbell | dynamic | a capsule bar turned 90° about Z plus two spheres, released tilted 18° |

Three kinematic capsule characters walk the lanes using `computeCharacterMotion` with behaviour-owned gravity. The props are never scripted. Cameras: overview, arch (low, looking back through the opening) and props. Each run is 300 ticks with capture every 4 ticks: 76 frames at 960×540, 157.6 MB raw, within budget.

**Numeric reading of the logs** (each matches its frames):

- **Arch lane:** crosses x≈0.6 at z=0 with no extra contact, so the opening is hollow.
- **Terrace lane:** climbs the rotated ramp part to y=1.32 (0.5 deck + 0.82 capsule centre), steps down to 1.07 and returns to the floor at 0.82.
- **Wall lane:** clamps at z=−2.73 (face at −3.05 + radius 0.3 + offset 0.02). The rotated return part then pushes it out to z=−2.33.
- **Dumbbell:** lands by t≈50 and rests at y=0.16 (the sphere radius), level, with only yaw left.
- **Stool:** lands upright on its three legs and stays upright.

**Retained frames** (`screenshots/engine/`, copied unchanged from run A):

- `overview-t000-start.png`, `overview-t148-arch-and-terrace.png`, `overview-t300-end.png`
- `arch-t100-approach.png`, `arch-t156-through-opening.png` (the hero frame: the character inside the opening), `arch-t200-past-arch.png`
- `props-t024-falling.png`, `props-t040-landed.png`, `props-t300-rest.png`
- `defect-stool-topple-9deg-5cm-t000-t040-t080-t300.png`: a 2×2 pixel-copy sheet from the defect variant run (see Defects)

**Visual notes:**

- The landed stool sits cropped in the arch camera's right foreground. It is honest but slightly distracting.
- The edge of the backdrop slab reads as a dark band at the top of the overview.
- Lighting is the single sun with shadows. `EnvironmentLight` needs an HDR texture asset, which this scene does not provide.

**Repeat runs on the same host** (Apple M5 Pro, original binary):

- Runs A, B and C: **228/228 frames byte-identical** and all logs byte-identical. Run C used the strict-TS-fixed script (a non-mutating `[...query].sort`), which had no behavioural change.
- Final state is equal once the IDs that `init` and the journal mint (project, scene, asset and transaction ULIDs) are normalised. Only `wall_ms` differs.
- The same binary with and without `--output` gives identical logs and state, so capturing does not perturb the simulation.
- This is **local equality only. Cross-host determinism was not tested.**

**Performance:**

| Run type | Wall time |
|---|---|
| 300 ticks + 76 captures at 960×540 (`play` `wall_ms`, 9 runs) | 821–971 ms per run |
| Numeric replay, no captures (`/usr/bin/time`, warm) | 0.14 s |

These are informal single-host timings, not a platform budget measurement.

## v2 numeric replay (after the pause; no images)

I replayed the same saved course and compiled script with `incant_headless-v2`, without `--output`.

- v2 is repeatable: two runs gave byte-identical logs and equal state.
- v2 differs from v1 **only for characters touching rotated parts**:
  - the terrace lane on the rotated ramp, first at t146 (y 1.266 → 1.265);
  - the wall lane on the −35° return, first at t265.
- The largest logged position delta is 1 mm, at the logs' 3-decimal rounding.
- In full precision the final positions differ by at most **0.18 mm**. v1 had drifted the terrace character 8 µm off its lane (z = 2.400008); v2 keeps z = 2.4 exactly.
- Event logs (contacts, landings) are identical. Props and the arch lane are bit-identical.
- This course did **not** reach the 32% forward-loss regime fixed in 68993c8. The minimum per-tick horizontal steps are identical under v1 and v2: terrace 0.023 at ramp entry (t96) and wall 0.0108 sliding into the corner (t236), both explained by geometry.
- **Conclusion:** the rendered images from the original binary remain representative of v2 to within 0.18 mm. They still apply strictly to their recorded original binary.

## Defects and requests for Astra

1. **Investigated stool toppling: the original stability/energy objection below was not supported by numerical replay. Solver untouched.**
   - **Repro (numeric, no images needed):**
     ```sh
     python3 handoffs/0024-compound-inspector/tools/compound_lookdev.py OUT \
       --stool 9:0.05 --no-captures --binary target/release/incant_headless
     python3 tools/probes/compound-stool-energy.py OUT ENERGY_OUT
     ```
     This tilts the stool 9° about its local X axis and releases it 5 cm above the floor.
   - **Initial expectation (corrected below):** the COM (≈0.427 m up) leans 0.067 m toward the +Z leg, well inside the tripod (0.22 m to that leg, inradius 0.11 m). The stool should settle upright.
   - **Observed:**
     - At t13 it rests upright on all three legs (up·Y = 1.0).
     - It then slides about 0.16 m toward −Z while the +Z leg steadily lifts.
     - It rolls through 180° (t57–t97) and rests inverted at (3.49, 0.51, −2.08).
   - **Initial hypothesis (withdrawn below):** a 3 cm fall cannot supply the energy to lift the COM over an edge, suggesting energy injection or wrong mass/contact properties.
   - **Controls:** 0° from 0.05 m and from 2 m, and 6° from 0.05 m, all settle upright. 6° from 0.3 m and 12° from 0.3 m also topple.
   - The trace is **bit-identical under v1 and v2**.
   - The default look-dev scene drops the stool upright. The historical topple sheet is retained.
   - **Astra numerical investigation, after the capture pause:** independent additive primitive mass/inertia calculation gives 17.3657 kg and local COM y=0.427306 m. A point-foot tipping estimate needs a COM rise of only 0.014254 m (2.4283 J); the release has 9.3252 J above the upright equilibrium. Rounded-foot contact changes the exact barrier, but the claim of insufficient release energy is unsupported. At t13 the body is near upright, **not at rest**: velocity is (−0.2444, −0.2847, −0.5241) m/s and angular velocity (−1.2390, approximately 0, 0.5778) rad/s. At t97 it is near sideways, not inverted yet. Across all 301 state samples, total energy never exceeds its 80.4162 J release value and finishes at 14.0716 J. Energy is not strictly monotonic: the largest local increase is 0.8868 J at a later contact. This fixture does not establish an engine defect or prove universal energy conservation. A fresh public-command-built project reproduces the complete numeric trace exactly; no images were generated. See `docs/spikes/evidence/compound-stool-energy-2026-10-09.json`.
2. **Data-binding gap: no per-field component diagnostics from the native bridge.** `snapshotFromEngine` only emits viewport and asset-source diagnostics. The engine's compound validation returns a single rejection string such as `compound part 3 rotation must be a unit quaternion`, with no JSON pointer. The Inspector consumes `Diagnostic.path` as contracted and was verified with fixture diagnostics using the engine's wording. **Request:** document-level component diagnostics with pointers such as `/shape/parts/3/rotation` and `/shape/parts/0/shape/half_extents/2`, so the native editor can place them at the exact field.
3. **Provenance labelling (observation):** RPC `command.execute` transactions record origin `user` with actor `editor`, even when a tool script sends them. Consider an explicit script/tool origin for public RPC callers.

## Screen work pause: unresolved native and visual review

Native CUA states to request from Astra once screen work resumes:

1. **Broken Railing at 1000×650, initial view:** part 1 preselected, Z half-extent axis invalid and its exact path visible, list summary not clipped.
2. **Rubble Pile:** End, then Page Up three times. Check the focused row stays visible inside the bounded list, the 2×2 quaternion in the card, and that VoiceOver reads the row sentence (for example "Part 49: …").
3. **Railing:** an Inspector Problems link for `/shape/parts/3/id` while part 1 is selected. Focus should land on part 4's full ID.
4. **Primitive Crate 01 and Mooring Post:** confirm no regression in native WebKit.
5. **Handcart:** with a widened Inspector (≥ 360 px card), confirm the vectors return to two-column rows.

Also pending:

- any new rendered v2 frames;
- native appearance acceptance of the list well, card border and focus ring.

## Limitations

- The browser fixture is not native WebKit evidence, and the engine frames are not native editor evidence.
- Live layout was checked only at 1440×900 and 1000×650, in the browser.
- Determinism is local only. GPU frame equality was established for the original binary, not for v2.
- There is no environment or HDR lighting in the look-dev scene.
- Row values truncate with an ellipsis in very narrow Inspectors. The full text is in the row's accessible name and tooltip, and in the detail card.
- The compound Inspector is read-only by design. There is no authoring UI or new mutation path, and the existing Read-only chip stays.
- The stool toppling report was numerically investigated by Astra; its original energy objection was withdrawn. No solver change or universal solver-correctness claim follows from this fixture.
