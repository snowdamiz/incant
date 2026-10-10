# Generate the editor context before compiling Rust

The editor now uses Tauri's supported build-script context generator. The pinned
`tauri-build` 2.7.1 receives `Attributes::codegen(CodegenContext::new())`, and the
binary includes its output through `tauri::tauri_build_context!()`. This moves
creation of included icon/asset files ahead of rustc's input timestamps. No UI,
icon, configuration, capability, runtime crate or workflow command changes.

## Reproduced invalidation

On macOS, the former `generate_context!()` proc macro creates a content-addressed
RGBA icon inside `OUT_DIR` and then emits `include_bytes!` for that file. Removing
only that generated input in an owned target reproduced this sequence on the
unchanged PR51 source:

1. Missing generated icon: editor compiles in 24.22s.
2. Immediate identical invocation: Cargo reports the same icon changed
   **115,096,747ns after the previous build began**, recompiles the editor in
   23.58s, and leaves the icon bytes unchanged.
3. Next identical invocation: fresh, 0.39s.

The original icon bytes and timestamp were restored afterward. This demonstrates
a generated-input ordering defect locally. The hosted Unix rebuilds below are
consistent with it, but hosted fingerprint diagnostics were not captured; their
entire cause is not asserted. Windows already avoided the GPU rebuild, so this
fix is not presented as a Windows critical-path speedup.

## Feature and boundary review

The old macro and the build helper both call the same pinned
`tauri_codegen::context_codegen` with the standard `::tauri` root, default
capabilities, no replacement asset provider and `test: false`. The macro derives
its development flag from the custom-protocol feature; the build helper reads
Tauri's `DEP_TAURI_DEV`, which represents that same feature choice. Separate Cargo
feature fingerprints keep development and custom-protocol outputs distinct.

The helper retains Tauri's ordinary resource/ACL preparation and adds rerun inputs
for configured icons, frontend files and applicable plist files. Tauri's existing
configuration and environment invalidation remain. Generated assets are still
compiled into the authoring editor; the shipped native runtime graph is unchanged.
The lockfile adds two edges to already pinned packages, with no version changes.

One production `app_context()` expansion is shared by startup and tests. Expanding
multiple context macros in one macOS executable would duplicate the embedded
plist symbol. The regression test checks the effective configuration (Tauri
omits only its authoring schema URL), package metadata, a valid RGBA window icon,
and empty development versus embedded custom-protocol assets.

## Local verification

An isolated APFS-copied dependency target was used; all editor generated outputs,
fingerprints and binaries in that copy were removed before the cold run. These
are cold **editor-output** results, not fully cold dependency builds or hosted
performance comparisons.

| Mode | Cold editor outputs | Immediate warm repeat | Warm compilation |
| --- | ---: | ---: | --- |
| Default | 32.79s | 0.27s | None |
| Custom protocol | 36.08s | 0.23s | None |

Both modes pass all seven editor unit tests and workspace all-target Clippy with
`-D warnings`. A temporary test example adds a synthetic text asset to the owned
generated frontend directory, verifies embedded decompressed bytes, changes only
that asset, and verifies the rebuilt context contains the new bytes. The example
and synthetic asset are then removed, the context is regenerated, and the
original directory timestamp restored. The immediate repeat compiles nothing.
No existing UI source or asset bytes are changed. The measured command retains the
workspace feature resolution and real model-GPU test targets:

```sh
./tools/cargo test --workspace --release --locked --test model_gpu \
  --bin incant_editor -j2 --no-run --timings
# Repeat immediately; then repeat both calls with:
# --features incant_editor/custom-protocol
./tools/cargo test --workspace --release --locked --bin incant_editor -j2
./tools/cargo test --workspace --release --locked --bin incant_editor \
  --features incant_editor/custom-protocol -j2
```

The no-run timing command compiles GPU test targets; it does not claim to execute
those tests. [Local evidence](evidence/tauri-build-context-2026-10-10.json) records
commands, generated-output identities, compilation checks and log hashes. Hosted
Linux/Windows validation and any merge-latency improvement remain to be measured.
