# 0021 directional shadows: look-dev, pixel review and Inspector result

## Final status

Final review complete. Scoped verdict: **native acceptance granted for the
scope of this handoff,** with two thin-caster limits recorded below. The
Inspector states, the Assets separation, the native shadow rendering, native
Undo/Redo and the mapped-normal correction all pass review on actual pixels.

The scope covers opt-in directional cascades for up to four suns and the
read-only Inspector presentation. The verdict does not approve a phase gate. It
also does not certify game or device performance, or production game art.

Exact model and transport for both rounds: Claude Opus 5.5, model ID
`claude-opus-5-5`, through the director's Claude subscription via ACP. No model
substitution occurred.

The report has two rounds:
- This final round reviews the integrated native evidence and Astra's correction.
- The initial review from `38aef10`, integrated as `9f02f45`, is kept below as history.

## Final round

### Director feedback acknowledged

The bottom-dock criticism stays binding. Assets must read as a dedicated
workspace with consistent spacing, details in the main Inspector, and
diagnostics below. I reviewed both Assets captures specifically and did not
reintroduce Assets as an output tab.

### Assets separation (assets-separated-wide and -minimum)

The organization is correct at both sizes:
- Assets is a tab in the left workspace beside Hierarchy, with its own filter and Import button.
- The main Inspector shows the "No asset selected" state, not a cramped detail row.
- The bottom dock holds only Problems, Console and History.
- Filter, Import, folder header and asset rows share one left edge and an even
  row rhythm at both sizes. Nothing clips at the true minimum.

I found one real inconsistency and fixed it. The project-root group label
"Project folder" is prose, but it used the monospace style meant for real
paths such as `models/props/`. It read like code next to the asset names.
Real folder paths stay monospace. The root label now uses the normal UI face.
A new behavior test checks both cases. The sample browser fixture has no
project-root assets, so this change has no new browser capture. A native
capture of the Assets view would show it, and I list that below.

### Native Inspector (enabled, omitted, null, focus)

The wide captures and the true-minimum captures match the browser review:
- **Enabled.** The "Shadows" group shows Distance 60 m with the standard group rule and unit.
- **Omitted and null.** Both rows read "Off" in the quiet outlined box, aligned with the other rows.
- **Focus.** The read-only field shows the standard focus ring at minimum size.
- **No problems.** There are no layout regressions, and Problems shows no errors or warnings.

I did not reread the AX text files beyond the README's summary. That summary
says absent and null have distinct accessible explanations, which matches the
unit tests.

### Native shadow pixels (look-dev at 12 and 38 degrees, Undo/Redo)

The native default preview camera frames the scene differently from the
headless camera. I judged appearance, not framing.
- **Matches headless.** Contact, slopes and the sphere terminator look the same
  as the headless renders. There is no acne on open ground at either angle.
- **Redo restores exactly.** I decoded both JPEGs and compared only the viewport
  region. All 673,740 viewport pixels in the Redo capture are identical to the
  original 12° capture.
- **Limit 1 is present natively.** The 2 cm wall shows the same thin lit line at its ground contact.
- **Limit 2 is milder in this framing.** At 12°, the far posts' shadows are
  slightly stepped, but they are not broken into dashes. The default camera is
  closer to the posts than the headless camera.

### Mapped-normal correction (Astra)

I accept the correction. Shadow lookups now offset along the interpolated
geometric normal, flipped for double-sided back faces. Shading still uses the
mapped normal. The test compares opposing tangent-X normal maps along the
center column, where lighting is symmetric, so only a moved shadow edge can
differ there.

| Evidence | Center-column pixels differing by more than 1 level | Maximum difference |
| --- | --- | --- |
| Before | 81 | 12 |
| After | 0 | 0 |

About 30,000 pixels still differ elsewhere in the image. That is intended,
because normal maps should change shading off-axis. All four integrated
look-dev images are pixel-identical to my initial renders, so the correction
changed nothing without normal maps.

### Regression, timing and repeatability

- **Regression set.** The comparison now has 122 of 124 exact pairs. The only
  differences are the same two one-pixel, one-level camera-cluster files.
  My earlier 121 of 123 matched the files I was given.
- **Timing.** I read the medians in the final timing file. They are an
  analytical frame probe and not a performance gate.

| Shadowed suns | Casters | Median frame, 1080p |
| --- | --- | --- |
| 0 | 1 | 1.389 ms |
| 1 | 1 | 1.470 ms |
| 1 | 64 | 1.479 ms |
| 4 | 64 | 3.096 ms |

### Remaining thin-caster limits

1. **Contact leak.** Casters thinner than about one shadow texel, such as the 2 cm
   wall, show a 1 to 2 px lit line where they meet the ground. Every lower-bias
   setting I tried caused widespread acne without removing it. Receiver-plane
   depth bias is the usual next step if game art needs paper-thin casters.
2. **Far aliasing.** Thin posts under a low sun alias in the far 1024² cascades.
   In the headless framing they break into dashes beyond about 30 m. A
   world-space filter radius or more far-cascade resolution would address it.
   That is a math and resource decision for Astra.

Neither limit blocks this scoped acceptance. Both should be tracked before any
production art or quality gate.

### Final-round changes

- `editor/ui/src/components/assets/AssetBrowser.tsx`: monospace only for real folder paths
- `editor/ui/src/components/assets/Assets.test.tsx`: behavior test for the root label
- `handoffs/0021-directional-shadows/result.md`: this round

### Final-round commands

```
npx vitest run (editor/ui)               # 12 files, 294 tests passed
npm run build --workspace editor/ui      # pass; main JS 102.14 kB gzip, budget 110 KiB
```

Only UI code changed, so I did not rerun the unchanged Rust and GPU suites.
Astra's integrated run already passed them.

### Native requests

1. Capture the Assets view with an asset at the project root, at wide and
   minimum size. I need it to confirm the root label now reads as plain text natively.

### Open questions

- Should optional scalars read "Not set" or show an engine default once
  schemas carry defaults? This carries over from the initial round.

## Polish round: native Assets root label

Verdict: **accepted.** The project-root typography fix from `d2f3a02`, which
was integrated as `81d1fa0`, renders correctly natively. This closes my native
request from the final round.

I reviewed both CUA captures in the ignored `artifacts/shadow-native-polish/` folder:
- **Wide capture.** At 1440x900, "Project folder" uses the same sans-serif UI
  face and subtle colour as the rest of the list. It no longer reads like code.
  The folder icon, label and the Environment and Quad rows keep their alignment
  and even rhythm.
- **Short capture.** The second capture shows the same result with no
  truncation, overlap or row-height change. The AX tree still names the group
  "Folder Project folder".
- **Layout unchanged.** Assets stays a left workspace beside Hierarchy, the
  Inspector shows the asset empty state, and the bottom dock holds only Problems,
  Console and History. Problems reports zero errors and warnings.

**Evidence discrepancy.** The packet says the minimum capture is Retina
2002x1302. The file `assets-minimum.jpg` actually measures 1720x669. Its hash
matches the dimensions file, which also records 1720x669. The image shows a
short, wide window with macOS window controls, on what appears to be an external
display. The earlier native README said a 1720x669 capture must not be called the
minimum-size check. I therefore do not count this image as a 1000x650
minimum-window verification.

I accept the fix anyway, for three reasons:
- The change only swaps the label's font family to the narrower UI face.
- The label already ends with an ellipsis when space is short.
- The true-minimum capture from the final round, `assets-separated-minimum.jpg`
  at 2002x1302, showed this column with room to spare at the same 245-point
  width.

If a strict record is wanted, Astra can recapture this Assets view at the true
minimum size.

This round changed no code and reran no tests. The only file changed is this report.

---

# Initial round (history)

## Status (initial round)

Initial review complete. Scoped verdict: **Inspector presentation done; shadow
appearance acceptable for this initial stage, with two documented look-dev limits
and one recommendation for Astra.** No shader, pipeline or bias change was made,
because every lower-bias alternative I rendered was visibly worse.

This is not native approval. Native captures and final measurements have not
been received, and no phase gate or production game gate is claimed or approved.

Exact model and transport: Claude Opus 5.5, model ID `claude-opus-5-5`, through
the director's Claude subscription via ACP. No model substitution occurred.

Director feedback acknowledged: Assets stays a dedicated left workspace, asset
details stay in the main Inspector, the bottom dock stays Problems/Console/History,
and the neutral charcoal palette is unchanged. The new rows reuse existing
Inspector spacing. Browser captures confirm the titlebar and this separation.

## 1. Inspector: optional and nullable fields

`FieldView` now recognizes an authored absent value generically, for any field:

- **Omitted optional value** (`optional: true`, value missing) renders a quiet
  read-only row instead of a mismatch.
- **Explicit null** (`nullable: true`, value `null`) renders the same row; its
  accessible description says it was authored as null rather than omitted.
- **Wording.** An optional settings group (object, such as `shadows`) reads
  **Off**. An optional scalar reads **Not set**, because absence is not always
  "disabled" for scalars.
- **Look.** Same 28 px box as other values, but an outline instead of the filled
  well, subtle text, no warning colour. It reads as "nothing authored", not as
  an empty edit field. It is a read-only input, so Tab focus, label association
  and the existing focus ring all work.
- **Enabled shadows** use the existing group language unchanged: a "Shadows"
  group with a "Distance" number row and the `m` unit from `x-incant-unit`.
- **Strictness kept.** A missing required value, a null on a non-nullable field,
  or a missing value on a nullable-but-required field still shows the dashed
  mismatch notice. Fields with no flags (older bridges) stay required.
- Engine diagnostics on an unset path still attach to the row.

No toggle, edit control or mutation path was added. Bridge, native and Rust
schema code are untouched. The browser fixture's `incant.Light.shadows` now
mirrors the native resolved schema. "Sun" has `{ distance: 120 }` and a new
"Sky Fill" light omits shadows.

Tests added: eight behavior tests in `FieldView.test.tsx` cover omitted, null,
enabled, required-missing, unflagged, optional-vs-nullable distinction, generic
scalars and diagnostics. One app-level test covers Sun and Sky Fill with an axe
check. The hierarchy count assertion moved from 17 to 18 for the added fixture
light. No other assertion changed.

## 2. Shadow PNG review (artifacts/shadow-initial, artifacts/shadow-cli)

I decoded every capture rather than relying on the JSON. All ten CLI PNG
hashes match `shadow-cli-evidence.json`.

| Case | Finding |
| --- | --- |
| Contact/bias | Lit receiver is one exact colour in every capture, including the self-casting receiver, which is byte-identical to the non-casting one. No acne. |
| Filtered boundary | Penumbra is 3 to 6 px at 640x480, smooth, no stair-steps. Wider across the oblique light axis, as expected. |
| Direction | Shadow lies on the side opposite the sun; the reversed-sun CLI capture moves it to the other side. |
| Retained versions | Queued captures 0 to 4 show the expected per-version sizes and positions. Restored, undo, reopened and source-free CLI captures are identical to enabled. |
| Alpha holes and reflection | Mask removes half the occluder. The reflected and double-sided-back cases show the mirrored half. Single-sided back casts nothing, which is correct. |
| Offscreen caster | Shadow present; the reference without it is uniform. |
| Multiple suns | Overlapping shadows combine with tinted partial penumbrae, not hard seams. |
| Cascades and fade | All four cascade panels show a crisp shadow. The fade panel is partial, then absent. No visible seam. |

Fixture shadows read pure black because these analytical scenes have no ambient
light. That is a fixture choice, not a shader defect.

## 3. Look-dev on real cooked geometry

New ignored GPU test `tests/shadows/lookdev.rs` cooks a real glTF through
`cook_gltf`. The scene has a ground plane, a resting cube, a sphere, a 25° ramp
with a post, a 2 cm wall and posts receding to 76 m. It renders sun elevations
of 12°, 38° and 70° plus an unshadowed reference. It asserts that the near
open ground at 38° matches the unshadowed render exactly. That acne guard
passes at the current bias and fails when slope bias drops to 1.

Findings at the current settings:
- **Contact, slopes, curves: good.** The cube and sphere shadows attach. The
  sphere terminator is smooth with no acne, even at a 12° sun. The ramp receives
  the post's shadow continuously onto the ground.
- **Limit 1, thin-caster contact leak.** A 2 cm wall shows a 1 to 2 px lit line
  at its ground contact. Pixel value is 177 against 82 shadowed and 183 lit.
  This is the bias tradeoff on a caster thinner than about one texel. Thicker
  casters do not show it.
- **Limit 2, far thin-caster aliasing.** At a 12° sun, 24 cm posts beyond
  roughly 30 m cast dashed, broken shadow lines. This is undersampling in the far
  1024² cascades, not a filter bug. The distance fade itself is smooth.
- **Grazing edges.** The cube's top edge darkens by up to 13 levels out of 130
  at a 12° sun. This is normal comparison-filter behavior and acceptable.

Bias experiments are kept in ignored `artifacts/shadow-lookdev-*`:

| Variant | Wall leak, 38° | Isolated acne pixels, 38° / 70° |
| --- | --- | --- |
| Current: constant 2, slope 2, offset 0.25 texel | 177 | 1 / 0 |
| slope 1 | 151 | 8,057 / 21,061 |
| constant 1, slope 1 | 151 | 8,047 / 21,077 |
| no hardware bias, offset 1.0 texel | 120 | 5,453 / 0 |
| constant 1, slope 1, offset 0.5 texel | 152 | 26,262 / 0 |

Every lower-bias variant shows obvious moiré acne on open ground, and none
removes the leak. I therefore kept the shipped values. `shade.wgsl`,
`caster.wgsl` and `pipeline.rs` are unchanged.

## 4. Regression and repeatability

I verified the comparisons independently with a standard-library PNG decoder:
- 121 of 123 shadow-regression pairs are pixel-equal to graph-initial, and
  all hashes match the JSON.
- Both differing files, the `camera-clusters-1-480x270` capture and its oracle,
  differ at exactly one pixel, (242, 67), red channel 120 versus 121.
- shadow-camera-stable and shadow-camera-repeat are 16 of 16 byte- and
  pixel-identical, and match `shadow-camera-stability.json`.
- The stable captures equal shadow-regression (121) exactly. The old 120 baseline
  came from random-ULID ordering and is not reproducible by the fixed fixture.

Verdict: the difference is a summation-order artifact, and repeatability is
established.

## 5. Browser review

Captures from `tools/capture-inspector.mjs` use headless Chrome over the shipped
sample fixture at device scale 2. They are synthetic data, not native WebKit
evidence and not engine pixels.

- `screenshots/initial/shadows-enabled-{1440x900,1000x650}.png` and `-inspector.png` crops
- `screenshots/initial/shadows-disabled-{1440x900,1000x650}.png` and crops
- `screenshots/initial/shadows-disabled-focus-{1440x900,1000x650}.png` and crops
- `screenshots/initial/report.json` holds the measurements

Every capture has zero axe violations, zero console errors, zero remote
requests and zero horizontal overflow in the Inspector. All Light rows are
28 px. The focused "Off" field shows the standard focus ring. As with every
read-only input, browser tab-focus selects its text.

## Changed paths

- `editor/ui/src/components/inspector/FieldView.tsx`: unset detection and quiet row
- `editor/ui/src/components/inspector/FieldView.test.tsx`: new behavior tests
- `editor/ui/src/styles/app.css`: `.control--unset`
- `editor/ui/src/bridge/fixture.ts`: native-shaped shadows schema, Sky Fill light
- `editor/ui/src/App.test.tsx`: integration test and hierarchy count
- `crates/incant_render/tests/shadows/lookdev.rs` and `tests/shadows/mod.rs`: ignored look-dev GPU fixture
- `handoffs/0021-directional-shadows/tools/capture-inspector.mjs`, `screenshots/initial/`, `result.md`

## Commands and results

```
npm test --workspace editor/ui                    # 12 files, 293 tests passed
npm run build --workspace editor/ui               # pass; main JS 102.14 kB gzip (vite), budget 110 KiB
cargo fmt --all -- --check                        # pass
cargo test --workspace                            # 144 passed, 0 failed, 39 ignored
cargo test -p incant_render --test model_gpu -- --ignored   # 30 passed
cargo clippy --workspace --all-targets -- -D warnings       # pass
INCANT_SHADOW_EVIDENCE=$PWD/artifacts/shadow-lookdev-before \
  cargo test -p incant_render --test model_gpu lookdev -- --ignored   # pass
node handoffs/0021-directional-shadows/tools/capture-inspector.mjs initial   # 6 captures
```

The main bundle grew from 101.77 to 102.14 kB gzip. No dependency, remote font or
network request was added.

### Rust checks

- `cargo fmt --all -- --check` passes.
- `cargo test --workspace` passes 144 tests with 0 failures. 39 tests are
  ignored native-GPU tests; that count was 38 before the new look-dev test.
- `cargo test -p incant_render --test model_gpu -- --ignored` passes 30 of 30 on
  the local Apple M5 Pro. That covers all five shadow GPU tests plus the look-dev test.
- I did not run the other 9 ignored GPU tests in other test binaries. My
  changes do not touch them.
- `cargo clippy --workspace --all-targets -- -D warnings` passes.

## Requests for Astra

1. **Shadow normal offset uses the normal-mapped normal.** `model_material.wgsl`
   passes the mapped `n` into `direct_lighting`, and `shadows/shade.wgsl` offsets
   the lookup along it. Strong normal maps can tilt that offset. I recommend
   passing the interpolated geometric normal for the offset. The change crosses
   `lighting/shade.wgsl` and `model_material.wgsl`, which are outside my edit
   scope. I did not observe an artifact, because no fixture uses a normal map.
2. **Far thin-caster aliasing (limit 2)** is a resolution or filter-radius
   question: a world-space filter radius in cascades 2 and 3, or more far-cascade
   resolution. It is a math and resource decision, so I made no change.
3. **Thin-caster contact leak (limit 1)** is acceptable now. If it matters for
   game art, receiver-plane depth bias is the usual next step.

## Native requests (CUA only; this session has no native CUA)

1. Capture the editor with a project whose DirectionalLight has shadows enabled
   and one with shadows omitted. Select each light and capture the Inspector at
   the minimum window size and at 1440x900. Confirm "Off" and the distance group
   match the browser captures.
2. Capture the native viewport of a shadowed scene with real cooked geometry,
   preferably the look-dev layout, at 12° and 38° sun. I will review contact and
   far-cascade aliasing on device.
3. Provide shadow-enabled frame timings with 1 and 4 suns. The current timing
   file only covers disabled shadows.

## Open questions

- Should optional scalars read "Not set" or show an engine default once schemas
  carry defaults? I chose "Not set" to avoid inventing a value.
- Is a 2 cm caster realistic for planned game art? If so, limit 1 should be
  tracked as a requirement.
