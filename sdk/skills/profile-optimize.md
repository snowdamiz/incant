# Profile and optimize

## Workflow

1. Fix the representative scene, seed, input recording, entity count, build profile and device. Obtain profiler CPU/GPU and allocation data from an actually available host capability; without it, identify the measurement gap and do not invent timing.
2. Attribute the expensive system before editing. Check per-entity callbacks, repeated native crossings, allocations, repeated navigation and copied columns. Prefer one update over NativeContext.chunks with typed columns and reused bounded buffers. Keep deadline checks, capability checks and rollback behavior intact.
3. Choose one hypothesis with a predicted observable saving. For hot loops that remain expensive, use the generated Rust SDK and native Wasm module route only when the trusted host can build/cook it; production native modules use wasm2c and development uses Wasmtime. Do not claim arbitrary TypeScript became native code.
4. Keep the original implementation available for a matched comparison. Preserve world, HUD, audio and deterministic replay outcomes. If the speedup is absent, revert the candidate through ordinary authoring history and record the negative result.

## Verification

Run the identical warmed workload in the shipping profile with the same actual hardware/OS. Compare p50/p95/tail cost, allocations and whole-frame impact, not nominal resolution or operation counts alone. Test limits and cancellation. A faster isolated loop is not a faster complete game; record the full cost of bridges, compilation and memory traffic.
