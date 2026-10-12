# Authored animation trees and complete Wasm poses on main147

The union preserves the reviewed authored-tree producer `21e73c29`, v4 SDK
`8dd91697` and ordinary Rust package proof `144be370`, on actual main #147.
All 81 changed implementation/build files exactly match accepted producer bytes.
Merge resolutions preserve the current retained IK implementation, hosted UI and
offline audio checkpoint ownership, and unchanged Cargo resolutions. No new
solver, animation arithmetic, renderer appearance or API behavior was introduced
by integration.

The frozen source `4487c5ae` passes all 18 validation commands: 793 affected CPU
cases across 136 result groups (82 GPU cases intentionally ignored), 401
Wasmtime/compiled-wasm2c cases across 71 groups, both all-target Clippy scopes,
matching headless and direct native oracle builds, wasm32, lean runtime graphs,
exact fresh schemas, SDK generation/strict TypeScript, formatting, conventions
and 237 Python tests with four existing skips. The test scopes overlap and are
not a distinct combined total. The conformance bundle's 151 modules/756 files
were independently verified while present; focused guest artifacts and the
original manifest remain retained.

A fresh QuickJS shipping player runs three full 56-tick games, six split/resume
pairs and two exact refusal controls after private authoring/build/source-map
removal. Complete sample logs match the independent oracle. Repeated and split
PCM bytes and final saves are equal. A separate fresh Rust/wasm2c shipping player
runs three 20-tick games after its own source/cache/build/tool/map removal. All
3420 scalar observations and 228 selected checks match the direct native oracle
and captured-only Wasmtime process; complete logs and actual PCM are equal.
Eleven exact unsupported/invalid requests refuse without publishing output.
The ordinary exporter's separate startup control is retained in each ledger.

Every frozen source input and actual package/output hash was rechecked after the
runs; all 1696 source/build inputs remained unchanged. The outer runner and all
owned children completed. Two assumptions in the separate post-run audit helper
were corrected, with failures retained: the QuickJS Cargo command redirects to
`package/build.log`, and the exporter's initial startup process must be counted
separately from the final copied-player proof. No engine, probe or package bytes
changed for those audit corrections.

This is local macOS CPU and unsigned ordinary-package evidence. No GPU, native
window, rendered appearance, reference-phone, performance or phase gate is
claimed. Full guest checkpoints, weighted tree-state timed transitions, layering,
authored IK and root motion remain separate work. The later grouped-pose/fade and
native-input/renderer corrections are outside this frozen union. Earlier producer
checks remain retained with their original source and artifact identities.

[Exact source comparisons, commands, logs and process audit](evidence/native-animation-trees-main147.json).
