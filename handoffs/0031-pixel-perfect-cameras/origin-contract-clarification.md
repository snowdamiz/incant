# 0031 clarification: placement reference frame

Consultation, 2026-10-10, on the contradiction the current packet found in
[origin-contract.md](origin-contract.md) §3 rule 6. This is not an implementation, schema,
rendered acceptance or gate decision. Astra owns the math, schema and numerical tests.

## Status and feedback

- **The contradiction is real, and rule 6 is withdrawn.** A rule that sees only camera and
  origin positions cannot give both world-grid statics and exact following at near ties.
- **Recommendation: attachment chooses the reference; nothing is inferred.** Free marked
  roots keep the world grid. A marked root under a *placed ancestor* (a marked entity or
  the rendering camera) rounds its offset from that ancestor. Each rigid group then makes
  one decision, and the marker needs no mode field.
- **Lighting.** Agreed: the continuous pose includes cluster lookup by continuous
  position, because shifted tiles can miss lights. Nothing here changes lighting intent.
  Projection-only lighting stays under review; contact breathing stays an open A12 question.
- **Scope.** The packet came in the runner prompt; the on-disk `brief.md` is the earlier,
  unedited one. Only this file and a link in origin-contract.md changed, not 0032 or 0033.
  No source, schema, UI or renderer edit, build, Cargo, screenshot, capture or new
  rendered evidence; no phase gate approved or claimed.

## Why one decision per group

If two origins meant to stay a whole number of px apart are snapped separately,
representational error can put them on opposite sides of a tie. The packet shows this for
camera and follower. It also breaks my §2/§4 claim that a marked child at a whole-px
offset "never separates". Apply the camera's current rule to both origins: a parent at
0.5 + 2e-15 px on the right axis snaps to 1, but its child at local +3 px snaps to 3, a
2 px gap for that frame. At 0.5 − 2e-15 px on the up axis the gap is 4 px.

Rounding one origin's offset *from the other* lands near a whole number, half a pixel from
any tie. A frame cannot tell a follower from a static half-grid origin, so the reference
must be declared. The hierarchy already declares what moves with what.

## Recommended contract

Per pixel-perfect camera and frame, a marked root's **placed ancestor** is its nearest
ancestor that is marked or is the rendering camera. Other cameras count only if marked.
A **free root** has no placed ancestor; an **attached root** has one.

1. **Free roots keep the world grid** (origin-contract §3 rules 1–5). Placement depends
   on the root's own origin and the camera's grid, never on the camera's position.
2. **Attached roots round relative to their placed ancestor:** the ancestor's placed origin
   plus the root's continuous camera-plane offset, rounded to whole px per axis with the
   camera's tie rule. The offset comes only from the transforms between them, the camera's
   rotation and pixels per unit; translating the group never changes it. The rendering
   camera's placed origin is its snapped position.
3. **Governance reaches the camera.** Unmarked drawables still move with their nearest
   marked ancestor (§2). A pixel-perfect camera under a marked root moves rigidly with that
   root's placement offset, then applies its own snap, replacing §2's camera sentence. A
   marked camera governs its unmarked descendants like any marked root.
4. **Nothing is inferred.** There is no tolerance test, motion detection or history, and
   no change to retained state. Roots moved together only by scripts or physics share no
   decision, such as a follower scripted beside the camera or a rider on a platform.
5. **Unchanged:** §1 and A0, marker-blind perspective and orthographic cameras, per-camera
   independence, replication and letterboxing, the three origins, declared range and cost.

**Tradeoff chosen:** free content gets world stability; screen stability comes only through
attachment. Two costs are documented:
- **Script-only followers** keep a window near camera ties, which Astra declares and may
  narrow. It is tiny but sits where accumulated float error lands, so real motion can hit
  it. In one frame it can equal a static half-grid origin, so stillness cannot be promised.
- **Still content under a moving ancestor** steps against the world. Keep it unparented.

| Content (replaces affected rows of origin-contract §4) | Promise | Strength |
| --- | --- | --- |
| Free root, static, any origin including half-grid | Same cell at every camera position | Exact |
| Free root, moving | Steps on the world grid, never against static content | Exact |
| Free roots moving together without attachment | Each follows rule 1. Their offset may change by 1 px for a frame inside the declared window | Exact to the rule; no stillness promise |
| Attached root, constant whole-px offset | Holds the offset in every frame, at every translation in the declared range | Exact; ½ px margin |
| Attached root, constant off-grid offset | Holds the rounded offset. An exact half resolves the same way in every frame | Exact |
| Attached root, changing offset | Moves relative to its ancestor in whole px | Exact to the rule |
| Attached root, still in the world while its ancestor moves | May step 1 px against static content | Documented |
| Camera under a marked root, constant whole-px offset | The root holds its home pixels and the world scrolls in whole px | Exact |
| Unmarked, no marked ancestor | Today's behaviour, including the near-tie follow jump | None (§1) |

**Authoring.** Scenery stays unmarked or free; actors are free roots. Parent a locked
camera under its marked subject; under an eased camera the free subject steps with the
world. Mark camera-locked content under the camera, or mark a rig carrying both. Unmarked
parts keep fractional offsets; marked parts round. Mixed modes become mixed attachments.

## Options considered

| Option | Outcome | Verdict |
| --- | --- | --- |
| A. Camera-relative roots only | Followers are exact, but free actors step against the world inside one camera cell (the packet's E = 0.25 example). Static on-grid roots inherit today's race: E = 2000 lands at 1999 with the camera exactly on the 0.5 tie. With the opposite tie direction, it lands at 2001 for C = 0.5 + 1e-14 | Kept only for roots attached to the camera |
| B. World/camera mode field | Each mode is honest, but the field can contradict the hierarchy. A camera-mode child separates from a world-mode parent. A camera-mode player slips while an eased camera lags | Not preferred: two sources of truth |
| C. World grid only | Statics are exact; followers and marked children keep the window | Kept as the free-root rule; not enough alone |
| Old rule 6: tolerance inheritance | Contradictory; hides a discontinuity in an undeclared band | Withdrawn |

## Changes to origin-contract.md

- **Withdrawn:** §3 rule 6 and §6's "whole-px tolerance in rule 6"; Astra declares the
  free-root window instead. **Narrowed:** §3 rules 1–5 cover free roots only.
- **Corrected (§2, §4):** rule 3 replaces §2's camera sentence. Camera-locked stillness
  needs attachment. The n + f alternation is for free roots; attached roots hold the
  rounded offset. "Never separates" holds by rule 2; attached half offsets resolve one way.
- **Answered:** the consistency part of open question 2, by attachment. Parenting meets
  question 5 for locked cameras; tracking without parenting stays a later camera feature.
- **Kept:** the typed `incant_cmd` marker, nearest-wins subtrees, the unmarked default,
  projection-only lighting under review and the acceptance method.

## Affected acceptance

- **A1.** Two captures per configuration: *camera-attached* (marked A, B, O under the
  camera) and *marked rig* (a marked rig carrying camera and followers). In both, unmarked
  twins A′, B′ and free marked twins F_A, F_B move by script beside the rig. Pass:
  - A and B are at home pixels and exact size in every frame.
  - O holds (20, 10) px, since (+20.5, +10.25) rounds by the tie rule.
  - Unmarked followers under the marked rig hold; A′, B′ equal the markers-removed run.
  - F_A and F_B equal the world-grid prediction in every frame. Frames where that
    prediction leaves home count as the window, not as stillness failures.
- **A2.** As A1: attached followers are perfectly still, and O never alternates. A free
  off-grid twin follows the predicted world-grid cadence.
- **A3.** Attached results equal the origin case frame by frame. Free roots equal their
  predictions on represented values. Whether F_A and F_B stay home is reported.
- **A4.** Unchanged, plus S4: a marked child at local (+0.3, −0.7) px under S0. S4 holds
  S0 + (0, −1) in every frame. No free root steps inside a camera cell.
- **A5.** The shield and charm become attached roots, and "never separates" is gated.
- **A8.** A camera's attachment applies only to its own frames.
- **A13, new: mixed roots,** at 320×180 0° and 255×143 0.37 rad with A2's motion and the
  near-tie set. A camera under a marked player, at a whole-px then a (+0.25, −0.25) px
  offset, keeps the player at constant pixels; marked and unmarked followers under it hold.
  A marked child held still in the world under a moving marked parent reports its steps
  against the card, each ≤ 1 px. An outside second camera equals its single-camera run.
- A0, A6, A7 and A9–A12 are otherwise unchanged.

## Model, effort, paths and limits

- **Model and effort:** Claude Opus 5.5, ID `claude-opus-5-5`, per this session's system
  context. Max is required, and `tools/acp/config.json` pins `claude-opus-5-5` at `max`.
- **Verification:** the packet says the first runner printed "Claude session verified:
  Opus 5.5; thinking: max"; the resumed runner must verify again. I see no runner output,
  transport metadata or effort readout, so I cannot confirm either. I substituted nothing.
- **Paths:** new: this file. Appended: a link section in `origin-contract.md`. `result.md`
  is unchanged because the packet limits writes; it already links origin-contract.md.
- **Commands:** read-only git, grep and reads of the packet, `camera.rs`, `pixel_grid.rs`,
  `pixel_follow.py` and the spike doc. `shasum -a 256 -c evidence/screenshots.sha256`
  reports 49 OK. A scratch `python3 -I` stdin run emulated the camera's one-axis tie rule
  for the numbers above. It wrote no file, ran no engine and is not a test.
- **Limits:** analysis only, with no frames. The ½ px margin rests on one measured adapter.
  Placement needs parent links at render time; I did not inspect the cooked runtime.

## Open questions

1. Director and Astra: is attachment the follower mechanism, with the window documented?
2. Astra: what are the window's width and measurement method, and the declared range?
3. Later: should a camera track a marked target's placed origin without parenting?
4. 0033 or later: should the Inspector show each marked root's resolved reference?

## Boundaries, 2026-10-10

[origin-contract-boundaries.md](origin-contract-boundaries.md) bounds v1 to one marker per
branch, with the camera as the only other reference, and states per-role displacement and
light/shadow limits. It supersedes the nested-root rules above. The text above is unchanged
as history.
