# Five project-template backend starters

The authoring-only `incant_templates` crate and host CLI catalogue/creation
commands provide five recipes: empty, 3D third-person, 2D platformer, top-down
and first-person. Their CPU authoring and cooked headless controls pass.
[Usage and publication contract](../../templates/builtin/README.md)
describe their controls, assets, reversible edits and failure behavior.

The implementation closes a catalogue/creation gap by composing existing
components, import and command operations. It does not add a runtime movement or
physics implementation. TypeScript uses native bulk state, committed physics
queries, structural flipbook commands and native input actions; preparation cooks
those sources once before disposable simulation. Core and planar samples remain
unchanged. No GUI selection path, editor-only rebuild, appearance, window or
performance acceptance is asserted.

The scene/source recipe has explicit fresh IDs and one command transaction.
Import precedes that transaction using the ordinary import snapshot/commit path.
The host creates fixed raw asset input bytes separately; untrusted project data
cannot supply their paths. Initial publication uses a create-only final project
filename after source and journal checks at the permanent directory location.
Moving a populated staging directory would break existing root-bound source
identity, so the implementation deliberately reports incomplete directories and
does not claim whole-directory atomicity.

The CPU checks exercise all five real created projects, exact version
pinning, provenance, transaction undo/redo, source removal/restoration, durable
save/reopen, explicit command replay, no-overwrite publication and symlink refusal.
Actual headless sessions cook the generated scripts and check jump/ground/wall
collision, camera following/look and native sprite animation while authored bytes
remain unchanged. An authored starter invoking the fallback script fails its
control. Tests serialize cook/CLI child creation to avoid inherited advisory
locks from a sibling test.

The production source is `92891857bc53f094aca8800f84845a195098d844`, based on
main321 `bbdb4f8f3a5050992015628b8517a1d898e91975`. Test-only successor
`debbf621a535fa252f4ad215596a5c6635e0db6e` changes no recipe, controller or
runtime code. Qualification used Rust 1.99, arm64, the CI profile, locked/offline
dependencies and two Cargo/Rayon workers. These are CPU behavior results, not
shipping-profile or target-device measurements.

The retained execution generations are deliberately separate:

- Plan001 never launched Cargo. Independent review found missing ambient
  compiler/profile/feature override guards; runner002 adds those checks, with
  19 pure admission controls, before the first native qualification.
- CPU002 passed six authoring/publication tests and the empty/CLI and planar
  headless cases. Its spatial case failed an exact `player.z == 0` assumption
  after capsule contact sliding: actual Z was `0.019087493419647217`. The runner
  stopped, retaining the failure, executed test and CLI binaries, and raw logs.
- CPU003 passed the focused spatial case for both 3D templates, followed by
  strict all-target Clippy for both affected crates, strict TypeScript, formatting,
  conventions and diff checks. The earlier passing cases were not rerun or
  counted again. Together the retained runs establish nine distinct passing
  tests across the five starters.

The spatial correction tests gameplay behavior instead of assuming axis-exact
floating-point collision resolution: 30 unobstructed steps must advance at the
authored speed, and the route must stay within one authored capsule radius of
its starting movement plane. Jump, wall stop, exact following-camera relations,
look rotation and unchanged authored bytes remain asserted. No controller output
was clamped and no physics algorithm changed. The radius bound is a starter
behavior criterion, not a general solver precision guarantee.

[Portable evidence](evidence/project-templates/manifest.json) retains 97 records:
both source generations, all plans/runners, grants, raw command logs and results.
The roughly 627 KiB archive excludes binaries. Five copied executable records
(including the failed test) are hash-bound externally; `--binaries` needs those
retained local originals. Default replay checks source/log correspondence and
results without running Cargo, tests, scripts, a player or capture tools:

```sh
python3 -B docs/spikes/evidence/project-templates/verify.py
python3 -B docs/spikes/evidence/project-templates/verify.py --originals --binaries --git-source
```

The full offline evidence replay passed. It verifies immutable producer Git
objects, not arbitrary later main source. No native window, GPU, screen capture
or CUA operation occurred in this qualification. GUI template selection,
editor-only rebuild, appearance review and device/performance gates remain open.
