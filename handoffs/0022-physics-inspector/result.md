# 0022 physics Inspector: presentation and real-engine look-dev result

## Status

Scoped verdict: **browser-fixture presentation and headless engine look-dev are
complete for this handoff. Native acceptance is pending.** It waits on the CUA
requests listed below. This verdict does not approve a phase gate. It does not
certify the physics runtime, which Astra owns, or any device performance.

Exact model and transport: Claude Opus 5.5, model ID `claude-opus-5-5`, through
the director's Claude subscription via ACP. No model substitution occurred.

### Director feedback acknowledged

The bottom Assets layout stays rejected. All captures show Assets as a left
workspace tab, details in the main Inspector, and only Problems, Console and
History in the bottom dock. The report records `outputTabs` for every capture.
The neutral charcoal palette, continuous panels and titlebar alignment are
unchanged. The work adds no new tokens, colours or surfaces.

This was a fresh attempt. The worktree was clean at `a9367c1`, with no files left
from an earlier attempt.

## What changed

### 1. Tagged-union FieldView (read-only)

- **One row per union.** It is labelled with the field name ("Shape") and shows
  the selected variant tag (`box`, `sphere`, `capsule`) as its read-only value.
  The selected variant's fields hang below it in one indented group. There is no
  second legend and no repeated "Type" row. The group is labelled by the Shape row.
- **Variant selection is strict.** A variant is selected only when the value is a
  record whose tag is a string naming a known variant (`Object.hasOwn`, so
  `"constructor"` is not a variant). The other cases each show their own explicit
  mismatch, keep the raw value, show no variant fields and pick no default:
  - missing tag ("No “type” is set …")
  - unknown or non-string tag ("Unknown shape type "cylinder". Expected box, sphere or capsule")
  - non-record value
- **Extra fields are not dropped.** A field that the selected variant does not
  define gets an explicit notice.
- **The capsule note is visible.** The engine's own variant description ("Capsule
  along local Y; half_height excludes the hemispherical ends.") appears as a quiet
  note in the variant group. It is also part of the Shape row's accessible
  description.
- **Diagnostic paths stay whole.**
  - A leaf row now claims diagnostics at its own path and anything beneath it.
  - A vector shows the error on the exact axis. Only that axis is `aria-invalid`,
    and each axis input has its own DOM id.
  - The union row claims `/shape`, `/shape/type` and any path below `/shape` that no child row presents.
  - When the diagnostic path is deeper than the row, the full pointer (for example
    `/shape/half_extents/1`) is printed in mono on its own line under the message.
  - Links in the Problems list and the entity's problem list fall back to the
    nearest rendered ancestor. A link to `/shape/type` on an unknown shape
    therefore focuses the Shape mismatch.
- **Keyboard.** A mismatch notice that stands in for a field is now focusable
  (`tabIndex=0`) and has the standard focus ring. Keyboard users and Problems links can reach it.

### 2. Physics presentation

The engine schemas carry no titles, units or order for these components. Without
help, native would sort Collider alphabetically: density, filter, friction, …
A small presentation profile (`presentation.ts`) supplies display hints only.
Any `title`, `description`, `x-incant-unit`, `x-incant-widget` or `order` in the
schema overrides it. Keys the profile does not name still render, in a final
"Other" section. Nothing is validated, converted or hidden.

| Component | Layout |
| --- | --- |
| RigidBody | Motion · **Gravity and damping**: Gravity scale, Linear damping, Angular damping (dimensionless, no unit) · **Solver**: Can sleep, CCD (tooltip: continuous collision detection) |
| Collider | Shape (union) · **Material**: Density `kg/m³`, Friction, Restitution · **Collision**: Sensor, Memberships, Filter |
| AngularVelocity | Angular, `rad/s` in the label cell |

- **Section captions.** Captions are subtle text with extra top space. They have
  no rule and add no indent, so nesting stays at most one level (the variant group).
- **Units.** Scalars show the unit inside the value box, as existing fields do.
  Shape dimensions use `m`. For vectors the unit sits in the label cell
  ("Half extents m"). At 320 px, three axis boxes cannot hold "rad/s" plus values.
- **Collision masks.**
  - The raw u32 is always the focusable value.
  - A short summary sits where a unit would: "All groups" (4294967295), "No groups"
    (0), "Group 3" or "2 groups".
  - The spoken description lists the groups, collapsing runs: "Collision groups 1,
    5 to 8 of 32." Groups are 1-based, and group 1 is bit 0.
  - A 32-cell strip in four bytes sits under the box with `aria-hidden`.
  - A value that is not an integer in 0..2³²−1 shows an explicit mismatch.
- **Read-only.** No authoring control or mutation path was added. A test checks
  that every input in these components is `readOnly` and that the only button is
  the section toggle.

### 3. Fixture

`incant.Collider` in the browser fixture was out of date: it was a flat
string enum with camelCase names. I replaced it with mirrors of the native
`RigidBody`, `Collider` and `AngularVelocity` that use bare type names. A test
checks that each mirror matches `snapshotFromEngine` run over the real
`schemas/*.schema.json`, with the same keys. Sample cases:

- **Crates.** Dynamic box bodies. Crate 01 also has AngularVelocity.
- **Crate 03.** Keeps its −0.5 axis error, now at `/shape/half_extents/1`.
- **Mooring Post.** Static capsule with masks 1 and 6.
- **Wave Trigger (new).** Sphere sensor with masks 4 and 3.
- **Pier Planks.** Deliberately carries an unknown `cylinder` shape and an error at `/shape/type`.

The hierarchy count assertion moved from 18 to 19.

## Changed paths

- `editor/ui/src/components/inspector/FieldView.tsx`: union view, hints, nested claims, mask control, focusable notices
- `editor/ui/src/components/inspector/presentation.ts` (new): profiles, sections, mask helpers
- `editor/ui/src/components/inspector/InspectorPanel.tsx`: section captions, nearest-ancestor problem links
- `editor/ui/src/components/inspector/PhysicsFields.test.tsx` (new): 14 behaviour tests
- `editor/ui/src/styles/app.css`: sections, variant group alignment, vector unit, problem path, mask strip, notice focus
- `editor/ui/src/bridge/fixture.ts`, `editor/ui/src/App.test.tsx`: native-shaped physics fixture
- `handoffs/0022-physics-inspector/tools/capture-inspector.mjs`, `tools/physics_lookdev.py`
- `handoffs/0022-physics-inspector/screenshots/{initial,revised,lookdev}/`, `result.md`

`editor/bridge/*`, `schemas/*` and all Rust code are untouched.

## Commands and results

| Command | Result |
| --- | --- |
| `npm run test --workspace editor/ui` | 13 files, **309 passed** (295 before, plus 14 new) |
| `npm run build --workspace editor/ui` | ok; `tsc -b` strict passes |
| JS gzip of `dist/assets/index-*.js` | Vite reports 105.06 kB (= **102.6 KiB**); gzip -9 gives 101.2 KiB. Budget 110 KiB; brief baseline 102.14 |
| `node handoffs/0022-physics-inspector/tools/capture-inspector.mjs revised` | 12 captures |
| `./tools/cargo build -p incant_headless --release --locked` | ok (worktree `target/`, no shared `CARGO_TARGET_DIR`) |
| `python3 -I handoffs/0022-physics-inspector/tools/physics_lookdev.py` | init → 5 imports → 1 RPC transaction → validate → play; ok |
| Same script, second run into `artifacts/0022-physics-rerun` | all 26 PNG frames byte-identical (SHA-256) |

No new dependencies. No remote fonts or requests: every capture recorded
`remoteRequests: []`. No Rust was changed, so I did not rerun Clippy or the
Rust tests.

## Pixel review (browser fixture, Chrome, DPR 2)

These are synthetic fixture captures and not native WebKit evidence. Each state
was captured at 1440×900 and 1000×650. Every capture had zero axe violations,
including colour contrast in real Chrome. None had page errors, horizontal
overflow or clipped inputs.

**Initial round** (`screenshots/initial/*-inspector.png`) found three defects, all fixed:
1. **Value column offset.** Variant rows started about 9 px right of the outer
   value column, because the group's indent was taken from the value column. Now
   the 13 px indent is subtracted from the label column. The measured value
   column x is identical for every row: 1240 at 1440 wide and 807 at 1000 wide.
2. **Broken path.** The nested diagnostic path broke mid-word inline
   (`/shape/half_exten|ts/1`). It now sits on its own line.
3. **Mask label position.** The mask label was centred on the box plus strip, so
   it floated between them. It now aligns with the value box. I also softened the
   set-cell colour, because two full-accent "All groups" bars were the loudest
   thing in the column.

**Revised round** (`screenshots/revised/`): full window plus an Inspector crop for each state.
- `crate-body`: RigidBody sections; switches for Can sleep and CCD
- `crate-collider`: box with Half extents in m; Material with kg/m³; masks showing All groups
- `post-capsule-masks`: capsule note; Half height and Radius in m; Group 1 and 2 groups
- `trigger-sphere-sensor`: sphere Radius in m; Sensor On; Group 3 and 2 groups
- `crate-axis-error`: only the Y axis is red; message and full path below; badge on the component
- `unknown-shape-focus`: focus ring on the mismatch notice, "Unknown shape type "cylinder"", engine error and path below

At 1000×650, "Half extents" wraps to two lines next to its unit. This is the
existing two-line label clamp and reads cleanly.

## Real-engine look-dev (headless CLI, actual Rapier and GPU)

Source and output are in ignored `artifacts/0022-physics/`. Selected frames are
copied to `screenshots/lookdev/frame-{000000,000012,000024,000036,000048,000150}.png`.
Adapter: Apple M5 Pro. Output: 960×540, captured every 6 ticks over 150 ticks (2.5 s); play wall time 666 ms.

**Setup.**
- Five static glTF models were imported with `incant_headless import`. Each has
  its visual size baked into the vertices: floor box 12×0.2×8 m, crate 0.8 m
  cube, sphere r 0.35 m, Y capsule (half height 0.35 m, r 0.25 m), sphere r 0.3 m.
- One `command.execute` RPC transaction created the scene. Every physics root has
  unit scale, and each collider's dimensions equal its mesh's.
- The floor is a collider only. The four bodies are dynamic with CCD on. The
  crate starts tilted with an authored AngularVelocity.
- The ball has restitution 0.6. The violet "Ghost" sphere has `filter: 0`.
- An authored camera and a shadowed sun complete the scene. `validate` passed before `play --camera`.

**What the frames show:**
- **Crate.** It tumbles and lands on an edge. It then settles flat on a face.
- **Ball.** It bounces visibly, then comes to rest.
- **Capsule.** It lands tilted and rolls flat onto its side.
- **Ghost (filter 0).** It passes straight through the floor and has left the
  frame by tick 48. This is visible evidence that the masks work.
- **Contact.** Contact shadows sit under every resting body. No resting body shows
  a visible gap or any visible penetration into the floor.

**Final engine state** (report.json, tick 150). All three resting bodies have
zero linear and angular velocity, consistent with sleeping:

| Body | Rest height | Expected | Error |
| --- | --- | --- | --- |
| Crate (half 0.4) | y 0.39993 | 0.4 | 0.07 mm |
| Ball (r 0.35) | y 0.34993 | 0.35 | 0.07 mm |
| Capsule on its side (r 0.25) | y 0.24996 | 0.25 | 0.04 mm |
| Ghost | y −28.1, falling at −24.5 m/s | falls through | none |

The headless camera framing is wide, so the bodies are small. For review, the
motion and contact read clearly. As art, the scene is a plain look-dev setup and
not a representative game scene.

**Limits respected.** The scene has no character controller, mesh collider,
hierarchical bodies, kinematic body or sensor events, and makes no rollback
claim. Kinematic motion and sensors are covered only by the Inspector fixture,
not by this look-dev.

## Requests for Astra

1. **Native CUA captures (this session has no native CUA).** Open
   `artifacts/0022-physics/physics.incant.json`, or a copy of it, in the native
   editor at 1440×900 and the true minimum. Capture the Inspector for Crate, Ball,
   Pill and Floor. Specifically capture:
   - the Collider shape rows
   - the Material and Collision sections
   - the AngularVelocity row on Crate
   - keyboard focus on the Shape row
   - Play mode, if the native viewport shows the fall, with frames comparable to `screenshots/lookdev/`

   An unknown-tag document cannot be loaded natively, because Rust rejects it.
   The browser fixture is the only evidence for that state.
2. **Schema annotations (Rust).** Optional, but recommended for parity across
   clients. Add `x-incant-unit` annotations in `incant_doc`, as `lights.rs` does:
   - `kg/m³` on `density`
   - `m` on `half_extents`, `radius` and `half_height`
   - `rad/s` on `angular`

   Also add `order` entries for RigidBody (`motion, gravity_scale, linear_damping,
   angular_damping, can_sleep, ccd`) and Collider (`shape, density, friction,
   restitution, sensor, memberships, filter`). Optionally add
   `x-incant-widget: "collision-mask"` on `memberships` and `filter`. The UI
   already prefers schema values. Once these land, the matching profile entries
   become redundant and can be deleted.
3. **Diagnostic paths.** Physics validation errors such as "collider dimensions
   must be …" currently carry no JSON pointer. If they return
   `/shape/half_extents/1` and similar paths, the Inspector already places them
   on the exact axis or row.
4. **Mask numbering.** I used 1-based group numbers (group 1 = bit 0), matching
   Rapier's `GROUP_1`. If the script API or docs number groups from 0, tell me
   and I will change the summary text. Raw values are unaffected.

## Open questions

- The `kinematic` motion tooltip says "moved by its authored velocity". It
  follows the brief's "velocity-kinematic" wording. Please correct it if
  position-driven kinematics is ever planned.
- The pointer-path line under messages is mono and subtle. If Problems review
  finds it redundant next to the Problems list path, it could appear only when the
  row is an ancestor of the target. I kept it, because the brief asks to preserve
  full nested paths.

---

# Follow-up round: shared annotations and one semantic correction

## Status

Scoped verdict: **the follow-up is complete for browser-fixture review. Native
acceptance is not claimed.** Native CUA captures follow after integration. No
phase gate is approved.

Exact model and transport: Claude Opus 5.5, model ID `claude-opus-5-5`, through
the director's Claude subscription via ACP. No model substitution occurred.

The worktree started clean at `74bb4bd`, which includes Astra's `c547bed`:
Rust registry units, mask widget, order and the regenerated schemas. No
interrupted files were present.

## Director feedback acknowledged and applied

- **Damping is a rate in 1/s, not dimensionless.** My first brief said
  otherwise, and I followed it. Rapier applies `v / (1 + dt·damping)`. I deleted
  the profile tooltips that called damping a "damping coefficient
  (dimensionless)". The schema now supplies the unit `1/s` and the description
  "… damping rate, in inverse seconds." The Inspector shows both. Gravity scale,
  friction and restitution remain dimensionless.
- **Redundant hints removed.** Units and the mask widget now come only from the
  schema: `x-incant-unit` and `x-incant-widget: "collision-mask"`. `FieldHint`
  no longer has `unit` or `widget`. The AngularVelocity profile is gone. The
  profile keeps only what the schema lacks:
  - section captions
  - the "CCD" label
  - tooltips for motion, gravity scale, can sleep, CCD, friction, restitution, sensor and filter
- **One-based collision group labels.** Kept, as confirmed.
- **Runtime evidence.** Headless CLI motion is the supported runtime evidence,
  and native review is Inspector and static viewport only. I did not rebuild
  Rust or regenerate the motion fixtures, so the look-dev frames above are
  unchanged.

## Correctness fix: `sectionKeys` now honours schema order

Before this fix, `sectionKeys` reordered fields by profile keys even when the
schema supplied an `order`. It now takes `schemaOrdered`, which is true when the
component schema has a non-empty `order`.

- **The schema order is kept exactly.** Consecutive keys from the same profile
  section share a caption. Keys the profile does not name form an "Other" run and
  are never dropped.
- **Incompatible orders get no captions.** If the schema order splits a section
  into two runs, or places the uncaptioned lead row (`shape`, `motion`) after a
  captioned run, grouping cannot fit. The rows then render in schema order with
  no captions at all, rather than being rearranged.
- **The profile order applies only without a schema order.** This covers older
  bridges, where `orderedKeys` falls back to alphabetical.

New behaviour tests:
- **Pure function.**
  - With the native order, captions match the previous round.
  - With an order shuffled within sections, the order is kept and grouping still fits. An unknown key goes to "Other".
  - An order that splits a section gets no captions, in exact schema order.
  - An order with the lead row placed late also gets no captions.
  - Without a schema order, the profile order applies.
  - A component without a profile is unchanged.
- **Rendered through `App`.** The Collider schema is reordered to
  `shape, restitution, friction, density, filter, memberships, sensor`. The rows
  render in exactly that order under the "Material" and "Collision" captions. An
  order that splits a section renders in exact schema order with no captions.
- **Schema is the only source.** A Collider density schema without
  `x-incant-unit` shows no unit. A memberships schema without the widget shows no
  mask control.

## Fixture refresh

The browser fixture's `RigidBody`, `Collider` and `AngularVelocity` now carry the
current generated annotations:
- `order`
- units `1/s`, `kg/m³`, `m` and `rad/s`
- the damping descriptions
- `x-incant-widget` and `maximum: 4294967295` on both masks

The drift test now also checks that each fixture `order` equals the native
bridge output from the real `schemas/*.schema.json`. The native and browser
spacing are therefore produced by the same data.

## Changed paths (this round)

- `editor/ui/src/components/inspector/presentation.ts`: hints trimmed; order-preserving `sectionKeys`
- `editor/ui/src/components/inspector/FieldView.tsx`: units and mask widget from the schema only
- `editor/ui/src/components/inspector/InspectorPanel.tsx`: passes whether the schema supplied an order
- `editor/ui/src/bridge/fixture.ts`: current schema annotations
- `editor/ui/src/components/inspector/PhysicsFields.test.tsx`: order, units and fixture-order tests
- `handoffs/0022-physics-inspector/screenshots/followup/`, `result.md`

## Commands and results

| Command | Result |
| --- | --- |
| `npm run test --workspace editor/ui` | 13 files, **312 passed** (309 before; the old section test was replaced by four tests) |
| `npm run build --workspace editor/ui` | ok; strict `tsc -b` passes |
| JS gzip | Vite reports 105.10 kB (≈ 102.6 KiB); gzip -9 gives 101.27 KiB. Budget 110 KiB |
| `node handoffs/0022-physics-inspector/tools/capture-inspector.mjs followup` | 12 captures |
| Rust | not rebuilt or rerun; Rust is unchanged in this round, as the brief says |

## Pixel review (browser fixture, Chrome, DPR 2; not native evidence)

I compared the new captures with the previous round's `revised/` captures:
- **Changed as intended.** Only `crate-body` changed. At 1440×900 and 1000×650,
  Linear damping and Angular damping now show `1/s` inside the value box, right
  aligned like `kg/m³` and `m`. Gravity scale stays unitless. The section
  captions and row rhythm are unchanged.
- **Identical.** Every other full-window capture is byte-identical:
  - Collider box, capsule and masks, and the sphere sensor
  - the axis error
  - focus on the unknown shape

  The schema-sourced units and widget therefore render exactly as the profile
  did, and the reorder fix does not move anything for the native order.
- **One crop differs.** The 1000×650 crop of `unknown-shape-focus` differs by 2
  pixel rows. Its full-window capture is identical, so this is crop-box rounding.

Every capture had zero axe violations, no page errors, no remote requests, no
horizontal overflow and no clipped inputs. The value column is at 1240 px at
1440 wide and 807 px at 1000 wide for every row. The bottom dock shows only
Problems, Console and History.

Captures kept:
- `screenshots/followup/crate-body-{1440x900,1000x650}{,-inspector}.png`
- `screenshots/followup/unknown-shape-focus-1000x650-inspector.png`
- `screenshots/followup/report.json`, which covers all 12 states

I deleted the other follow-up captures because they are byte-identical to `screenshots/revised/`.

**Observation, not changed.** "Angular damping" and "Half extents" wrap to two
lines at both sizes. This is the existing two-line label clamp in the 34% label
column, and it reads cleanly.

## Remaining native requests for Astra (unchanged in substance)

1. **Native CUA captures, after integration.** Capture at 1440×900 and the true
   minimum. Use the Inspector on the look-dev project (`artifacts/0022-physics/`,
   or regenerate it with `tools/physics_lookdev.py`) for Crate, Ball, Pill and
   Floor. Capture:
   - RigidBody, showing the `1/s` damping
   - the Collider shape rows
   - the Material and Collision sections with masks
   - AngularVelocity
   - keyboard focus on the Shape row
   - a static viewport

   Native has no play controls, so there are no native motion captures. The
   unknown-tag state cannot be loaded natively; the browser fixture is the only evidence for it.
2. **Path-bearing physics diagnostics.** Optional. If Rust validation errors carry
   JSON pointers such as `/shape/half_extents/1`, the Inspector already places
   them on the exact axis or row.
