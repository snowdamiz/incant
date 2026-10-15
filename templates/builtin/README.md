# Built-in project starters

The offline catalogue supplies five ordinary authored projects. Creation uses
the same asset importer, command bus, source journal and document validation as
other authoring operations. Generated games use the existing cooked native
runtime and TypeScript SDK.

```sh
incant_headless templates
incant_headless new /absolute/existing/parent/my-game --template platformer2d --name "My game"
```

The destination directory must not exist. The receipt identifies the project
file, journal, stable role IDs, recipe transaction and revision. Open the emitted
`project.incant.json` through the ordinary CLI/editor project opener. This slice
does not add an editor template picker.

| Template | Authored behavior | Controls |
| --- | --- | --- |
| `empty` | One perspective camera, no gameplay script or imported asset | None |
| `third-person` | Kinematic capsule, collision course, jump, orbit and following camera | WASD/left stick, Space/south button, arrows/right stick |
| `platformer2d` | Gravity, jumping, solid tilemap, animated sprite and following camera | A/D or left stick, Space/south button |
| `top-down` | Two-axis movement, solid tilemap, animated sprite and following camera | WASD or left stick |
| `first-person` | Kinematic capsule, collision course, jump and eye-level look camera | WASD/left stick, Space/south button, arrows/right stick |

Look controls use authored actions. They do not acquire an OS pointer or enable
relative mouse capture. The spatial starter has an ordinary cube body in the
third-person view and no body mesh in the first-person view. Planar animation
uses the native flipbook component. These are functional starter assets copied
from licensed repository fixtures; [asset provenance](assets/provenance.json)
retains their original paths and hashes. There is no new game art or appearance
qualification in this backend slice.

Each recipe prepares fresh explicit project/scene/entity/script/tile-layer IDs.
The importer creates the asset ID. The bundled atlas and its frame IDs are stable
asset-local identities shared by copies of that same source asset. Once prepared,
the command list can be serialized and replayed without generating new IDs.
Import is one ordinary import transaction; scene, actions and script source are
one subsequent reversible transaction. Undoing that recipe removes the scene and
source, while preserving its imported asset for redo. Save and subsequent opens
use the existing host-bound project-file protocol.

Raw bundled input files are copied with create-only writes into a new directory;
they are then admitted by the existing importer. Imported cache/asset metadata,
script registration and text, scene components and input actions follow their
ordinary import/command/source paths. There is no runtime authoring dependency,
shell capability for scripts, or credential requirement.

`project.incant.json` is published last, without replacement, after document,
source-history, cached sprite and journal-reopen checks. This is a file publication
boundary, not an atomic directory operation. On an earlier failure, the command
reports the incomplete directory and leaves it for inspection without a project
file. Existing files, symlinks and directories are refused; nothing is recursively
deleted or silently retried. A final directory-sync error can leave the published
file present without a successful receipt, so inspect the reported destination
instead of rerunning creation there. The host must choose an existing real parent
directory whose path is not concurrently replaced; this API does not grant a
project or agent authority to choose an arbitrary filesystem root.

The native behavior controls live in
`crates/incant_headless/tests/templates.rs`; creation, refusal, explicit replay,
undo/redo and save/reopen controls live in `crates/incant_templates/src/create/tests.rs`.
The scripts are checked with:

```sh
node node_modules/typescript/bin/tsc --noEmit -p templates/builtin/tsconfig.json
```

The Phase 2 editor-only rebuild gate, visual review and target-device performance
remain separate from these backend starters.
