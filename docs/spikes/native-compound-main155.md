# Compound animation transitions on current main

This integrates the independently reviewed grouped-pose sampler and authored state fades from `44cd390f6ebd1972d21b3de1530c0cf287991dae` with actual main #155 (`fca5d5fe714748215fc285d24b998d33f6655fc1`). Transitions involving clip, two-input and nested weighted states retain independent clocks and interpolation modes. CPU consumers apply the grouped pose through one staged sampling/IK boundary; checkpoint continuation preserves outgoing and incoming state.

The integrated production source is frozen at `ca08565e961700bb0b6884fad1dce927c36ac329`. Of 88 accepted producer inputs, 82 are byte-identical. Six independently reviewed seams preserve current main's root-motion exports and IK-v1 capability path, extend compound snapshot refusal to IK-v1 length/read calls, and align the exact world-save diagnostic with the already-admitted v3–v10 range. Existing rollback and eight legacy pose-reader controls remain intact. Cargo manifests and lockfile equal main155.

## Validation

The affected native suite passes 766 CPU tests across 115 groups with 82 intentionally ignored GPU tests, followed by all-target Clippy. Real Wasmtime/wasm2c conformance passes 420 tests in 74 groups with no failures or ignored tests, all-target Clippy and three development processes. These suites overlap and are reported separately. The wasm32 renderer compile, lean dependency graph, generated SDK/bridge, strict TypeScript, formatting and convention checks pass. Python tooling passes 238 cases with four existing skips.

The ordinary export proof cooks a new current-source/current-lock package with the shipping profile. Public authoring uses the command bus and verifies Undo/Redo, recovery and unchanged authoring after CPU Play. After removing its authoring, build storage and generated debug source maps, the copied player runs three complete processes and ten save/resume pairs. Each complete run checks 112 full pose records and 224 independent vectors. Exact final save bytes, concatenated PCM and logs match the uninterrupted run. Corrupt checkpoints produce their exact admission error with empty stdout; failed observations produce the complete failed diagnostic report while withholding checkpoint and PCM files.

## Preserved failures and cache boundary

The first integrated run retained four obsolete `v3-v9` test strings after production already admitted v10. The correction updates only the exact diagnostic and its expectations; refusal and atomicity assertions remain.

A later wasm compile reused stale metadata from a target previously shared across checkouts. Its dependency records omitted the new grouped/pipeline inputs. Fifty original cache artifacts were preserved. A private APFS clone, reserved to this checkout, received a scoped wasm cleanup of `incant_types`, `incant_assets_runtime` and `incant_render` (7 MiB); the unchanged sources then compiled, and new dependency records include the omitted inputs. Successful CPU/Clippy results are retained separately. The shipping proof uses fresh disposable build storage. This is a measured recovery from this cache state, not a general Cargo cache correctness claim. A small checkout-identity guard is a separate follow-up.

## Limits

No GPU, native-window, appearance, phone or performance acceptance is claimed here. Existing Wasm v1–v4 and IK-v1 explicitly refuse compound snapshots; the reviewed complete compound ABI is a later integration. Whole guest checkpoints and connected root-motion gameplay remain open. Checkpoint13 independently evaluates frozen main155; this feature does not move that candidate or approve a phase gate.

[Source, command, failure and package evidence](evidence/native-compound-main155.json). Final integrated closure review is pending.
