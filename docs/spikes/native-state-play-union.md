# Native retained state and shared Play integration

This bounded union preserves the reviewed navigation/retained-world-save,
planar query, desired audio control and shared Play-startup commits on main88.
The only substantive merge resolution makes component, flipbook, audio and
navigation preparation finish before any subsystem publishes state. A combined
regression tests both command orders: invalid navigation motion keeps the world,
audio, tick, routes and pending queue unchanged; the later valid batch commits.

World-only v2 saves retain spatial origin/incarnation, navigation and flipbook
state. Sticky solver/audio-use refusals remain, and VM/timer/input/audio/solver
continuation is not implemented. The authoring-only `incant_play` factory shares
cooking and preparation with headless Play without adding authoring dependencies
to shipped native players. Native editor controls are a separate workstream.

## Integrated local validation

Frozen source `56eff03dda452ab5d8440efe6b6cc6b2cf3a63ed` passes 837 ordinary
workspace tests, all 75 serialized ignored GPU tests (including both headless
screenshots), full all-target Clippy, 15 optional native-window cases and Clippy,
compiler-disabled host Clippy, 536 UI tests, strict types/build and 17 Node tests.
The pre-portability Python suite passes 178 tests with two Windows-only skips.
All 62 freshly generated schemas match committed bytes; SDK/bridge/conventions,
workflow lint, wasm32 runtime compilation and the 16-root runtime dependency
guard pass. Exact public tools build successfully. The public source transaction
probe passes 32 requests in two fresh processes with Undo/Redo/recovery and native
motion; the independent native-world-save fresh-process proof also passes.

The fresh shared shipping session verifies four independent players, 251 target
compile units and both global deletion barriers before any packaged launch.
All original runtime, scene-audio, physics-script and GPU/flipbook/SDK controls
pass. Game proof retains 32 baseline plus 32 SDK repeated success processes,
43 refusal entries and four animated/static comparisons. Audio retains 16
processes. Each player retains its own executable/codegen/symbol/source-free
identity. Concurrent development makes this run unsuitable for speed claims.

[Evidence and log identities](evidence/native-state-play-union.json) bind these
results to the frozen reviewed source. Later script navigation, native editor
wiring, exact solver snapshots and complete session persistence are excluded.
Hosted platform checks remain a separate frozen release checkpoint; these local
results do not satisfy reference-phone, appearance or phase gates.

## Repaired-main integration

Actual main89 (`f8955bec`) merged as `68b0944` with only a STATUS conflict. All
76 changed runtime-union source hashes remain identical to the complete local
freeze; the three portability repairs are byte-identical to reviewed main.
Focused verification passes two explicitly enabled TypeScript compiler cases,
51 Play/headless tests, all-target Clippy for cook/Play/headless and a rebuilt
public headless executable. The real source transaction probe again passes 32
requests in two processes. All 199 Python tests pass with four Windows-only
skips; actual older macOS Python passes 35 lifecycle cases with four Windows
skips. The initial compiler-only command selected no gated cases and is not
counted; its explicit-feature repeat supplies the actual two-case evidence.

Checkpoint 3 is independently frozen at main89, so this development integration
does not move that candidate. Its repaired Linux AMD64 local-service job has
already passed all 28 protocol checks and four restart/persistence controls;
remaining platform confirmation is governed by all 23 release contexts.
