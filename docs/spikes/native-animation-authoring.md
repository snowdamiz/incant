# Native animation authoring and startup

The typed `Animation` component uses the ordinary command bus, project validation,
provenance and Undo/Redo. It requires a model renderer and refuses coexistence with
sprite/tilemap preview precedence. Its clip index, seconds, speed, play and loop
flags are initial configuration; the disposable native world owns subsequent time.
The published JSON Schema and TypeScript component type come from Rust.

`load_project_animation_resources` reads only selected animated entities' cooked
model caches at startup. Every project cache directory must be a real in-project
directory, and the existing bounded model decoder rejects redirected entries,
corruption, key mismatch and unsupported data. The sealed resource table retains
only the fingerprint and clip durations derived from decoded keys. It drops each
full model before loading the next. Scene cooking rejects missing/stale resources,
static models, nonexistent clips and times past the selected clip's end. The
older resource-free cook entry points retain explicit refusal for Animation.

Both authored-script and fallback routes in `PreparedPlay::load` consume these
resources, as does the actual `embed_project` export producer. Its dependency
tracking includes each consumed cooked model file. There are no source-model
reads, document validation or JSON work in animation ticks. Shipped normal
runtime dependencies remain free of authoring, command bus, cook and agent code.

Local evidence: 90 tests across the document/command/cook/play packages, all-target
Clippy, generated SDK and dependency checks pass. Tests use actual imported glTF
skin/clip data; source deletion precedes loading, and full cache deletion precedes
native execution. Both editor/headless startup script routes and both real export
producer routes execute in the tests. Other cases cover transaction rollback,
provenance, Undo/Redo, cache-directory and entry symlinks, stale fingerprints and
asset-local bounds. See [exact evidence](evidence/native-animation-authoring.json).

An earlier test incorrectly used the deliberately spatial-refusing offline game
host; it was corrected to exercise the actual native session from PreparedPlay's
owned inputs. A first compile used the wrong public path for `model::key_path`;
that error and both logs are retained. Final source is formatted and rechecked.

This is startup/cook/CPU evidence, not a new shipping-profile executable proof,
visual acceptance or reference-device measurement. Renderer and TypeScript
consumer integration are being checked separately. Blend trees, state machines,
IK, root motion, Wasm animation controls and the complete Phase 1 gate remain open.
