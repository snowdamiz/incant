# Orthographic camera appearance and Inspector review

## Priority revision: final native captures supplied

Review artifacts/0030-native-captures/manifest.json and its ten original native
JPEGs. They come from integrated source 59f3384, SHA recorded in the manifest.
The same binary includes PR25's already accepted compact Agent and compound
Inspector; integrated-ui.diff records those differences from your branch.
Normal app size is 1440x874; minimum is 1000x650, with explicit 280px Inspector
width for the final minimum states. The earlier tiny-min.jpg retains 331px;
tiny-min-inspector.jpg is the 280px state. Legacy, explicit perspective,
orthographic, tiny values and Tab focus are captured. Astra restored sizes.

The project is your original engine-r1 scene/cache with two shared-command
Camera additions (explicit perspective and tiny values). Native viewport uses
its existing preview camera; these are Inspector review captures, not an
authored-camera selection claim. Full images contain the saved account label;
keep them ignored and commit only account-free crops. The OS sharing pill
obscures traffic lights, outside this scope. The original Documents fixture
open stalled in OS __open, so the identical app/fixture was relocated into the
development artifacts directory; it opened immediately, with zero engine
errors/warnings and no permissions changes.

Astra's bridge test fixture correction b769347 is already in your branch.
Re-run tests before updating the earlier failure wording. Combined root passes
318 Rust, Clippy, 387 UI tests/build, five tool tests, contracts and the native
release/package. The public camera save/repeat workflow passes on the frozen
integrated binary. Independent analysis matches your numeric report exactly;
all 72 images across your two original engine runs match byte-for-byte, and all
18 committed frame hashes verify. No renderer changes require new look-dev.

Review the native Camera presentation, labels, units, default/unused tags, tiny
values, minimum spacing/overflow and keyboard focus. Return a clear final scoped
native verdict, or a concrete new defect to fix. Do not invent an engine or phase
gate claim. Preserve measured shadow limits. Commit result/evidence/crops with
Built-by: claude. You may make a necessary Camera UI fix if the captures reveal
a defect; explain it so Astra can integrate and recapture the changed state.

## Priority revision: correctness review, 2026-10-10

Initial checkpoint d9f8769 is accepted for its stated engine/browser scope.
Fix the following Inspector correctness issues before the native checkpoint:

- `formatNumber` rounds valid tiny positive values such as 0.000001 to zero.
  Preserve nonzero significant values (including negative small vectors) with a
  clear compact representation of your choice; keep ordinary values readable.
  Add observable regressions for small Camera clip/vertical size and a vector.
- `projectionKind` currently returns orthographic solely from `kind`, even if
  vertical_size is missing, wrongly typed, or there are extra fields. Your result
  says the Unused tag is suppressed for malformed projections. Make behavior
  match that claim and test it; do not silently repair malformed data.

Astra implemented your schema/bridge requests: Camera units now come from Rust
metadata; projection is explicitly first; schema projection help uses your
proposed user-facing sentence; FieldSchema carries optional unknown `default`,
and the bridge preserves own defaults across references/unions (including false
and null), without filling authored values. Update fixture parity, order tests
and presentation to consume that default safely instead of a duplicate hardcoded
value when available. Retain the visible distinction between default and authored.
Source and generated schema/bridge changes are supplied in your worktree.

Re-run strict UI tests/build and relevant browser captures at both widths. No
engine rendering implementation changed, so preserve the immutable binary and
previous real-render acceptance without repeating unchanged captures. Document
the shadow-depth quality limitation already measured; no cascade refactor is
required for this scoped checkpoint. Native acceptance still awaits Astra's
rebuilt captures. Commit your changes with Built-by: claude.

Claude Opus 5.5 through ACP owns all scene design, visual geometry, cameras,
rendered judgment, Inspector layouts/styling and screenshot review. Read CLAUDE.md,
docs/spikes/orthographic-cameras.md, tools/probes/orthographic-cameras.py and the
existing camera Inspector. The director's current preference is neutral connected
panels, clean spacing and a restrained accent. Assets live in the left workspace;
the bottom dock is Problems/Console/History. No new floating-card redesign.

## Scope and acceptance

Review actual orthographic projection in an original compact real-engine scene.
Use the immutable artifacts/tools/incant_headless with artifacts/tools/binary.json;
verify SHA before and after captures. Show at least two objects at clearly
separated depths with equal authored size, straight parallel lines, an oblique
view, local illumination and a directional shadow. Compare camera moving along
its forward direction at fixed vertical_size (size and parallel-view shading
must stay consistent), changing vertical_size, and switching projection. Use
real rendered frames, not painted replacements or UI mockups. Review framing,
unwanted foreshortening, clipping, lighting/shadow consistency and resizing at
16:9 and 4:3. Keep numeric correctness claims tied to measured observations.

Extend the existing read-only Camera Inspector to present projection type and
orthographic vertical extent clearly. Legacy cameras must still show perspective.
Preserve the existing mutation boundary: do not invent GUI-only writes, unsupported
controls or an agent-ready claim. Choose good labels, units, spacing, help and
keyboard behavior yourself. Display unknown/malformed values explicitly instead
of silently coercing them. Verify normal and minimum widths and primitive/perspective
regressions, plus strict UI tests/build. Return concrete evidence of any bridge or
engine correction needed from Astra. Native captures will be supplied by Astra
through CUA after integration/rebuild; do not claim native acceptance without
seeing them. The Mac is currently unlocked and computer use is authorized.

## Contract and limits

`Camera` retains required `fov_degrees`, `near` and `far`. Optional `projection`
defaults to perspective for legacy JSON. Explicit forms:
`{kind:"perspective"}` or `{kind:"orthographic",vertical_size:8}`. Vertical size
is the visible world-space height; width follows output viewport aspect. Field
of view is retained but does not affect orthographic projection. Near must remain
positive and far greater; this supports shared logarithmic light-depth clusters.
All authored component changes use normal incant_cmd RPC commands with provenance.
The renderer uses parallel view rays for orthographic PBR. Do not claim pixel
snapping, sprite/tilemap rendering or 2D physics; those are future implementation
work outside this increment. No phase gate is approved.

## Paths and execution

Allowed changes: editor/ui/src for Camera Inspector visual presentation and its
behavior tests; this packet's helpers/result/evidence. Do not change Rust, SDK,
mutation routing, CI, account settings or unrelated handoffs. Use supplied binary,
not another worktree's build target. No Cargo is needed in your worktree.

Useful paths: crates/incant_doc/src/camera.rs; crates/incant_render/tests/cameras;
editor/ui/src/inspector; handoffs/0019-authored-cameras and 0021-directional-shadows.
Install pinned node dependencies with npm ci if needed. Strict-check your TypeScript,
compile behavior with tools/build_script.mjs, and use public init/import/rpc/play
commands. Original mathematical glTF may be authored and imported. Never edit
project JSON directly or use shell access from inside an engine agent.

Capture 960x540 and/or 800x600 engine images, at most 128 frames and 256 MiB raw
per capture. Reuse per-run immutable projects; helper output directories must
refuse overwrite. Use small bounded scenes, not a benchmark load. Repeat the
meaningful rendered comparison and record raw hash equality or justified exact
limits. Save useful raw outputs in ignored artifacts/0030-orthographic-cameras.
Commit a concise set of unedited representative engine images, helpers and
compact evidence. Keep account-bearing native full images ignored and commit
only account-free crops when supplied. Do not publish, sign or merge.

Return result.md with exact model/transport, source/binary provenance, commands,
tests, a clear scoped appearance verdict and any remaining native/engine defects.
Every commit ends in Built-by: claude. An initial checkpoint can accept engine and
browser scopes while explicitly waiting for Astra's rebuilt native captures.
