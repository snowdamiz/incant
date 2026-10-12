# Wasm HUD current-main integration

The accepted additive HUD snapshot/read/command ABI and generated Rust SDK are
unchanged. Opcodes 56–58 preserve existing 0–55 behavior. Commands still pass
through the canonical atomic native transaction, with shared bounded text/command
quotas and poison-on-error semantics.

The integrated actual Wasmtime/compiled-wasm2c suite passes 469 cases in 84 groups,
with zero failures or ignored tests, plus eight-package all-target Clippy and three
fresh development smoke processes. SDK wasm32, runtime-only, generated contracts,
normal dependency guards, tooling, formatting and conventions checks pass.

A freshly compiled public Rust guest is imported through ordinary authoring;
Undo/Redo and journal reopening pass. The normal exporter builds a fresh shipping
player, removes its private build and copied source/cache/map inputs, and launches
three new processes. All 12-tick reports and mixed PCM match exactly. Actual
Latin/CJK shaped text, focus/edit/submit routing, native movement and changed HUD
text are checked. Five malformed/failed commands plus module checkpointing refuse
with precise diagnostics and no output. The actual 22-file package is unchanged.

The conformance guest and public-export guest are separate compilations of the
same reviewed source; this packet does not claim byte identity between them.
Final main165 ancestry adds only the reviewed Planar workflow/sample and evidence
files. All compiled source/cook inputs remain identical to the tested freeze.
[Exact sources, commands, raw receipts and package identities](evidence/native-wasm-hud-integration.json)
passed independent integration review.

Module checkpoints, GPU/window/appearance acceptance, physical devices and full
SDK/phase completion remain open. This CPU proof does not replace those gates.
