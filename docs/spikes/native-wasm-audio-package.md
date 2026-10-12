# Public Rust audio gameplay and ordinary native export

The authored Rust audio game passes the public import, command, cook and ordinary
wasm2c export path on aarch64 macOS. Two real WAV clips and the Rust module are
captured through public imports. One reversible command transaction authors the
scene and selects its module; Undo, Redo and the editor-default journal path
reopened through public RPC with `--journal` preserve the exact project.

The private WAV/module sources and guest build are deleted before development
Play. A separate typed native oracle reads the same cooked scene and captured
clips, executes native controls and mixes audio without guest code or an ABI
decoder. Wasmtime matches 186 declaration fields per tick over 96 ticks: 17,856
exact scalar observations, 248 bounded public vector checks and 614,400 bytes of
stereo PCM. The test includes static and streamed clips with distinct sample
rates, spatial/listener movement, bus/source gains, pauses, stops, restarts and
seeks. Each transport epoch is recorded as exact two-u32 words.

A fresh shipping build uses the scene's captured module selector and ordinary
`--package` startup. After deletion of this proof's project/cache, trusted tools,
private build directories and source maps, three fresh processes match every
development record and the independent native PCM exactly. No fixture selector
or guest globals provide gameplay state. The actual player is 9,966,928 bytes;
this is an artifact measurement, not a size or performance gate. The exporter
checks shipping codegen, compiler-free runtime dependencies and source-parser
absence. All 1629 frozen source inputs and 15 package files remain unchanged.

Six specific failures are exercised through both public hosts: short snapshot,
short command scratch, out-of-range original-f64 gain, missing owner, incorrect
audio role and seek beyond the captured clip duration. The thirteenth case is
the explicit static-module checkpoint refusal. Every case requires its exact
diagnostic and absence of published output. Callback prefix rollback is proven
by the separate actual-host tests. Clip-duration refusal occurs in audio
synchronization after native command acceptance and has no rollback claim.

[Evidence](evidence/native-wasm-audio-package.json) retains 110 exact raw records
in a lossless archive, 33 command receipts and hashes of the surviving actual
player, PCM, reports and cooked data. The first authoring/development and shipping
attempts passed without source or fixture correction. Independent final package
review is pending; the [host evidence](native-wasm-audio.md) is independently accepted.

Copied declarations describe desired committed state, never an actual mixer
cursor or voice status. Arbitrary guest checkpoints remain unsupported. The
shared engine/toolchain and evidence remain on disk; only proof-owned inputs
were removed. This is a local CPU desktop result with no physical audio-device,
GPU/rendered, hosted-phone, performance, complete SDK or Phase 1 gate claim.
