# Public Rust physics game package proof

This follow-up exercises the ordinary public authored Wasm route with physics
queries. It builds on the independently accepted [physics SDK](native-wasm-physics-queries.md)
without changing production ABI, physics algorithms, session or world codecs.
The new proof is stored outside `/tmp` so its package and receipts can survive a
machine restart. Corrected proof002 passed; independent final review and main
integration are pending.

## Connected path

The probe initializes a fresh project, imports a trusted Rust guest's Wasm through
the public importer and authors a nine-entity physics scene through journal-backed
`command.execute`. Scene selection, physics components and failure-control input
actions commit together. Public Undo/Redo and journal reopen verify that exact
transaction. The private Rust source, original Wasm and guest compiler storage
are removed before development and export consume the captured cooked module.

The scene contains a 3D ground, wall, kinematic character and sensor, plus planar
floor boxes separated by a gap, a kinematic actor and a sensor. The guest batches
seven native queries: ordinary and sensor rays in both dimensions, a masked miss,
a planar gap miss and character motion. It verifies event scene/dimension
metadata and observes both enter and exit transitions. Character displacement is
applied through ordinary numeric columns; query observations and the accepted
clock also drive a marker entity. A later ordinary column write moves the
character out of the 3D sensor. No rendering or physics debug drawing is involved.

A separate native oracle loads the actual exported `world.scene`, uses the
canonical native solvers directly and advances the same fixed-step gameplay.
It neither imports a Wasm host nor decodes the Wasm query ABI. Its per-tick values
produce sixty typed assertions over the marker, character and planar actor.
Captured-only Wasmtime development and three fresh ordinary `--package` players
must satisfy those assertions and agree on observable game state, logs, clocks,
audio frame counts and PCM hashes.

The ordinary exporter selects the authored module, translates its captured
prepared bytes with pinned WABT and generates the shipping descriptor/factory.
Its fresh shipping-profile compiler and dependency/symbol audits remain active.
No `--static-module` fixture selection is used. After export, only this proof's
new private project, cooked cache, tools, compiler storage and source maps are
removed and their hashes recorded. Fresh processes run with a minimal environment
from the player directory. Repository engine/SDK/probe source and packaged crash
symbols are intentionally retained outside the runtime data tree.

## Refusal controls and limits

Four ordinary input actions trigger failures after staged numeric changes,
a spawn and a log: an invalid native character owner, a caught short result
buffer, a caught original-f64 domain violation and a late invalid native command.
Both development and the packaged process must fail at the exact expected
runtime diagnostic without publishing PCM or stdout. Successful controls use
the same `--input-replay` and `--input-recording` routes, so an unknown flag or
missing input cannot masquerade as an expected guest failure. These process-level checks complement the SDK suite's
in-memory transaction rollback tests; they do not claim rollback of an already
accepted session step or resumability after terminal failure. Captured-cache
identity mismatch must also fail, and static module checkpoint requests remain
explicitly Unsupported before output publication.

This is a local desktop CPU package proof. It does not establish rendered
appearance, phone performance, whole guest/physics checkpoint support, complete
SDK coverage or a phase gate. Main integration and current-main checks belong to
the sole publisher.

## Measured result

Corrected proof002 completed three fresh ordinary shipping processes, each for
20 ticks, with all 60 typed native-oracle assertions passing. Development and
packaged runs had identical observed game state, 20 committed info logs, clocks,
16,000 stereo audio frames and PCM hashes. The native oracle observed one enter
and one exit in 2D, and three enters and one exit in 3D. All ten refusal cases
matched the exact expected runtime diagnostic and published neither PCM nor
stdout; checkpoint output was also absent.

The actual macOS arm64 player is 9,437,488 bytes. Thirteen retained package files
were directly rehashed. Fourteen deleted private file paths were checked absent,
plus seven explicitly owned preflight source/compiler files removed separately.
Twenty execution receipts survive without authoring-command contamination.
The [durable evidence](evidence/native-wasm-physics-package.json) binds the three
frozen proof sources, package/export identities, native-oracle observations,
four successful process reports, ten refusal reports, input fixtures, eight
compressed logs and 37 receipts. The actual package remains under
`~/.codex/incant-evidence/physics-package-002` for direct audit. No lost `/tmp`
artifacts from the earlier SDK conformance are required for this package result.

The guest compiled for `wasm32-unknown-unknown`; the native oracle built and
passed scoped Clippy. Formatting, Python compilation and the corrected tooling
suite passed. The ordinary exporter independently verified fresh shipping flags,
matched crash symbols and absence of authoring, Wasmtime, cooker and runtime
source-compiler dependencies in the shipped normal graph.

## Earlier attempt

Proof001 completed three fresh ordinary players and all sixty per-tick native
oracle checks, including both dimensions' enter/exit events. Independent source
review then tightened the probe's evidence: authoring transaction commands had
shadowed its execution ledger, and generic nonzero-exit refusal checks did not
exclude unrelated CLI errors. The corrected probe uses separate receipts,
requires each exact observed runtime boundary, exercises successful replay
routes, and omits a child HOME override. Its Rust guest and native oracle are
unchanged. Proof001 remains preserved as preliminary evidence rather than the
final acceptance packet.

The first recovered Python tooling run failed because its restricted PATH lacked
Node after restart. Node 22.23.3 was found in the user's fnm installation; rerunning
the unchanged suite with Node available passed 233 tests with 4 existing skips.
Both outcomes remain recorded. The oracle's scoped Clippy and the actual no_std
Wasm guest compilation passed before either package run.
