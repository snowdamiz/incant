# 0021 directional shadows: initial look-dev, pixel review and Inspector result

## Status

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
