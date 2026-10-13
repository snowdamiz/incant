# Shared headless UI and reusable player integration

The accepted native Play UI routing and ordinary-player library compose with the
accepted Rust/Wasm audio ABI and actual main167. The CLI stays a thin wrapper over
the same reusable entry; native Play and exported players share the recorded UI
resolver. The optional bounded C ABI uses that entry with caller-owned request
bytes. No project mutation bypass, device renderer or target-cook exception is added.

All accepted implementation/tool inputs remain byte-exact. Cargo.lock only combines
the reviewed local dependency edges; dependency versions/checksums do not move.
The tools-only physics diagnostic from the mobile caller producer's parent is not
included. The production test source is `5da9fb4a9430a65a9b58f5624ad3f903b43989eb`;
main167 adds only reviewed release documentation afterward.

All 19 integrated commands pass: 74 runtime cases, 16 actual headless CLI/RPC
cases, 13 player cases, and 60 native-window/static-module compatibility cases,
plus all-target Clippy, runtime/wasm/contracts/types/format checks and standalone
fixture build/Clippy. The 13-player and 60-compatibility counts overlap and are not
claimed as 73 distinct tests. Native features were compiled and unit-tested; no
window, IME, GPU or screenshot was used.

The copied macOS `mobile-ffi` CI-profile archive is 86,391,344 bytes with SHA-256
`0e758d6e839a36e8c4ab6cc0b5ccbb7cca74874a4ce0b0935274e82d16a8c23d`.
Real C and Swift callers link against that exact archive and pass all 32 controls
across 39 commands. Held native input moves the typed cooked fixture to x=0.1,
release at tick2 gives x=0.05, repeated reports/PCM agree, and exact failure,
malformed/null/oversized request and create-only output controls pass. These are
real macOS CPU processes for the declared trusted build-linked fixture, not a
shipping-size measurement or an authored mobile export.

An initial earlier main166 static-module compatibility command omitted its required
trusted bundle and correctly failed before tests. A hash-verified private copy of
the accepted producer bundle corrected only the harness environment; the complete
current audio union chain then passed with that explicit input. Both prior attempts
and their exact source/log identities remain in the [evidence manifest](evidence/headless-ui-mobile-integration.json), alongside
1,875 frozen inputs, 222 retained integration artifacts and the independently
rehashed 176 original caller artifacts. Independent final integration audit is accepted. Actual main168 ancestry adds
only documentation with all 1,875 compiled inputs unchanged; exact-head fast CI
remains required.

The existing cross-target cook refusal remains intact. Later iOS compatibility,
simulator execution and device link experiments belong to their separate producer;
this slice claims neither iOS nor Android execution, physical phones, native UI,
physical audio, frame time or a phase gate.
