# Bound authored-script native embedding proof

The actual Rust emitted by `CompiledScript::embedded_rust()` for a bound, two-source project compiles and runs in a standalone shipping player. This is an executable host==target integration proof, not a new public export pipeline or a save/device/performance gate.

The helper `tools/probes/project-script-embedding.py` copies the selected engine inputs into private disposable storage, installs its engine-owned probe template as a temporary workspace member, preserves every existing pinned dependency resolution, transforms three actual TypeScript modules using the fixed Node cook, and builds the generated Rust. The trusted build script constructs initial typed fixture data (no existing project is edited), validates it through the real program-aware authoring cook, verifies source-catalog pairing, writes its actual authoring JSON and cooked scene, and includes only freshly emitted Rust. It never accepts project native code, a project Cargo manifest, a build script, or a supplied bytecode cache.

After packaging the executable and matched crash symbols separately, the helper deletes its private engine/TS/project/Node-output/generated-Rust/target storage **before** any copied-player run. The original repository, review packet and host compiler installations remain outside that storage. Runtime PATH contains only the player directory; this does not claim absolute system Node is inaccessible. The normal graph and actual target rustc invocations exclude authoring/document/command/agent/JIT dependencies and the QuickJS compiler feature. Linked-symbol checks require the bytecode reader/evaluator and reject the source parser; all six dynamic-source routes also fail during real guest updates.

## Reproduce

From an integrated checkout with pinned Node dependencies and native Rust/llvm-tools installed:

```sh
python3 tools/probes/project-script-embedding.py /tmp/incant-bound-embedding-proof
```

For this isolated review packet:

```sh
PATH=/opt/homebrew/bin:$PATH python3 /tmp/incant-project-script-embedding-work/packet/tools/probes/project-script-embedding.py /tmp/incant-project-script-embedding-work/proof-1 --engine-root /Users/andreyyurlov/.codex/worktrees/wabt-download-retry/incant
```

The destination must be new. The actual Cargo invocation uses `-j2`, `--profile shipping`, `--locked`, `--offline`, an explicit native target and fresh target storage. No GPU or root target is used. Output `result.json` records copied artifact identity, codegen, template/source hashes and individual control results. No workflows are changed by this packet.

## Observed local result

On `aarch64-apple-darwin`, Rust 1.99.0, all **16 fresh processes passed**: 12 positive runs at 0/1/4/16 ticks (three repeats each), three late-failure controls, and one pairing-refusal process. Positive runs verify lowercase authored source normalization, native source/entity order, one shared helper-module instance, shared numeric loan with later detachment, per-instance movement, frozen nested props with hostile escaped keys/strings, and all six evaluator/constructor refusals. Failure controls verify earlier numeric prefixes roll back for late eval/Function/throw and the owning NativeSession hides world/input/log state after accepted-tick failure. Pairing controls reject another scene and constructing a bound host without its world.

- Stripped player: **5,249,016 bytes**, SHA-256 `09cf7657ca460d2d66fff627162a4c7fa1c1a08982ef1923816adc67d25022c9`.
- Verified arm64 executable/dSYM UUID: `39d5d55f-6b91-3584-aad9-5f7231657af3`.
- Actual generated Rust SHA-256: `f0e006ffcaba78ad055ff2fb7260234565e4629100ef8908d75170e2bc0f8f17`.
- Original executed result SHA-256: `5159a53d3fb2257de29e4beb4412c2fa2974239eb2bcefdb3884874ce0f6ff79`.
- Clarified result SHA-256: `a047355a533c7b01d6c3f0ec767fb3d6298c9afce5f736f5a99bf2e519453f47`.
- Executed helper SHA-256: `6103274308bbc49efa785a91e4e1927eb7c118f3cd00eaac08b407d08a72d85c`.
- Packaged helper SHA-256: `2fec316a9ba6c07bd2f28e5412e7fc2a935a33aac60821b3de5c11c784cb0bd5`.

The only post-execution helper changes remove an unused Python import and narrow `node_available_to_runtime` to `node_on_runtime_path`. The original executed helper/result are retained separately; no build, fixture, runtime or assertion behavior changed. Rustfmt and actual shipping compilation passed; Clippy was not separately rerun for this disposable fixture. Linux/MSVC/mobile execution remains unverified by this local run. No phone performance claim follows from shipping codegen alone.

## Integration boundary

The generated bound Rust names `incant_types::StableId` directly. The probe therefore declares **incant_types as a normal runtime dependency**. Existing generic native-player manifests do not all declare it normally; this proof does not silently fix or claim their project-bound export integration. A future export player must supply that dependency (or deliberately change/review the generated public path). The compiler/authoring packages remain build-only dependencies. No unsafe loader or production mutation path is added.

Exact evidence and source hashes are in `docs/spikes/evidence/native-project-script-embedding.json`; the local full artifact bundle remains `/tmp/incant-project-script-embedding-work/proof-1`.
