# Native gameplay integration

This union preserves the reviewed commit ancestry of PRs 65, 69, 70, 71 and 74 on top of main's static-module lifecycle and PR75's CI/import/local-first plan update. It is published as one runtime integration so dependent branches do not each repeat the same matrix after main changed.

Native QuickJS behaviors can query physics and move characters, schedule bounded fixed-tick timer events, and publish bounded primitive logs only after successful updates. Typed observation plans inspect requested native values and input edges. The static wasm2c/Wasmtime capability path gains the same accepted input timeline through a bounded copied snapshot. None of these APIs mutate authoring documents.

The only behavioral integration addition forwards committed diagnostics through SessionVm. QuickJS forwards its bounded history; the static ABI has no log-writing function and returns immutable empty history. NativeSession and composed game readiness still hide diagnostics after terminal failures, including host failures after guest publication. Independent Astra review found no integration blocker. Existing ancestor reviews and scoped limitations remain in their evidence documents.

## Verification

On frozen source `3da3dd6`, 593 ordinary workspace tests/doctests, all-target workspace Clippy, optional GPU-player Clippy, 86 Python tooling tests, SDK conformance and runtime dependency guards pass. The 66 ignored GPU tests were not repeated locally for this nonvisual union; hosted checks retain them. The actual static-module composed suite passes 97 tests/doctests and Clippy, including successful empty diagnostic history and terminal refusal.

Two fresh shipping-profile builds pass after temporary compile inputs/build storage are deleted. The QuickJS CPU-render-input proof runs 47 process controls, including bounded log history and false/failed observation distinctions. Its 4,686,928-byte player has SHA-256 `335c2cc39af1f3ba42937d8a94ba29a8fd0c2287cf1f5e2cdaca5d81ad18fb42`, matching arm64 dSYM `77936847-dec2-3b53-9154-a04db3bf80a2`. The static game proof runs 8 successes and 26 refusals; its 4,032,832-byte player has SHA-256 `5d1d8ed33d16c92ae29e46e48fda238a60311785b24130b413ac8ae90aed5bd0`, matching arm64 dSYM `3ea0139f-aa04-33ff-b992-b949505f8f0d`.

[Machine evidence](evidence/native-runtime-integration.json) retains both complete proof results and local log/source hashes. The first integration attempt exposed a redundant Result conversion to Clippy; the final source returns the already-typed result directly, and all final checks pass.

These are local desktop correctness/build proofs. Full native SDK parity, editor/headless project Play migration, native session saves, static guest logging and rendered/game/device performance gates remain open. No phase gate is approved by this merge.
