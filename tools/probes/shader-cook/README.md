# Shader translation and backend metadata probe

This standalone, pinned authoring-time investigation translates the renderer's
six current WGSL families, one entry point at a time, using Naga 29.0.4. It emits
MSL, SPIR-V, HLSL and metadata identifying runtime arrays and workgroup memory.
It does not load shaders onto a GPU or implement the production shader cooker.

From the repository root, choose a new output directory:

```sh
./tools/cargo run --manifest-path tools/probes/shader-cook/Cargo.toml --locked --release -- "$PWD" "$PWD/artifacts/shader-cook-output"
```

The probe intentionally uses compact resource bindings and disables vertex
pulling. Those choices isolate compiler feasibility and metadata requirements;
**its generated libraries must not be inserted into the renderer**. Production
integration must use the actual complete pipeline layouts and preserve robust
buffer/vertex bounds and workgroup-memory allocation.

HLSL uses shader model 6.0, explicit probe register/heap bindings and checked
entry-point reflection. Its registers do not describe the engine root signature.
Translation alone does not validate DXIL; a target DXC compile and actual driver
tests are still required. The report records translated entry-point names for
that later compiler step.

To compile an emitted Metal stage without initializing a GPU:

```sh
xcrun -sdk macosx metal -std=macos-metal2.4 -c artifacts/shader-cook-output/grid-build.metal -o artifacts/shader-cook-output/grid-build.air
xcrun -sdk macosx metallib artifacts/shader-cook-output/grid-build.air -o artifacts/shader-cook-output/grid-build.metallib
```

An installed Xcode Metal Toolchain is required for those two commands. The
retained [investigation](../../../docs/spikes/shader-cooking-interop.md) records
actual results and the missing runtime integration. This tool has its own lock
and no normal dependency edge from a game player or renderer.
