# Five project-template backend starters

Source preparation adds an authoring-only `incant_templates` crate and the host
CLI catalogue/creation commands. The five recipes are empty, 3D third-person,
2D platformer, top-down and first-person. [Usage and publication contract](../../templates/builtin/README.md)
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

The planned CPU checks exercise all five real created projects, exact version
pinning, provenance, transaction undo/redo, source removal/restoration, durable
save/reopen, explicit command replay, no-overwrite publication and symlink refusal.
Actual headless sessions cook the generated scripts and check jump/ground/wall
collision, camera following/look and native sprite animation while authored bytes
remain unchanged. An authored starter invoking the fallback script fails its
control. Tests serialize cook/CLI child creation to avoid inherited advisory
locks from a sibling test.

At this source milestone the TypeScript strict check has passed. Rust compilation
and the authored behavior controls have not run; their result must be recorded
before claiming the starters qualified. No native window, GPU, screen capture or
CUA operation is part of this work.
