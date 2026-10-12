# Native Rust/Wasm HUD controls

The additive `incant_hud_v1` interface uses operations 56–58 for bounded copied
pre-command widget/text declarations, routed UI outcomes and typed HUD controls.
Operations 0–55, input-v1 bytes and all earlier definitions are unchanged. UI
outcomes distinguish committed runtime input values from immutable declarations.
Both module paths now accept composed UI; the existing router filters consumed
keys before gameplay input. Commands use the same ordered final-state validation
and transaction as native TypeScript. Invalid active calls poison the entire tick.

The immutable source checkpoint passes six parser/native boundary tests, three
actual Wasmtime/wasm2c tests (11 raw cases and five composed failures per host),
16 generator tests, generated equality and formatting. The actual composed hosts
consume packaged fonts after source cache deletion, expose notifications exactly
once and preserve accepted output on failure. The Latin-only unit font does not
prove Chinese glyph coverage. The forthcoming public package proof uses real
fallback fonts. One raw test initially mistook low-level stale-epoch handling for
actual host behavior; its corrected expectation retains existing fail-closed
host semantics. Original failing output is preserved.

The [source packet](evidence/native-wasm-hud-source.json) binds the reviewed
implementation and focused results. Broad checks and public source-deleted
shipping proof are still pending. Static/development guest checkpoints remain
unsupported. No native-window, GPU, rendered-appearance or device performance gate
is claimed. The separate headless native-play UI viewport route remains pending.
