# First hosted Cargo unit timings

PR53 at `17e4e64b48a72db9a72bb74ece88762940c7f5a4` passed all 17 checks.
The diagnostics-only workflow changes preserved the existing commands, targets,
features and coverage. [Evidence](evidence/ci-pr53-unit-timings-2026-10-10.json)
records successful job IDs, artifact hashes, exact GPU identities and cache keys.

| Native check | Execution | Queue | Ordinary compile | Compiled units (workspace / other) | GPU compile | Explicit tools compile |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Linux | 11m00s | 2s | 249.89s | 147 / 188 | 25.73s, 1 unit | 31.33s, 7 units |
| macOS | 17m08s | 151s | 355.06s | 147 / 110 | 38.64s, 2 units | 64.53s, 7 units |
| Windows | 23m24s | 2s | 680.87s | 147 / 121 | No units | 97.36s, 7 units |

The ordinary suites passed 401/403/398 tests respectively. All three ignored-GPU
suites match the exact 44 baseline identities. Each job restored a main50 prefix
ending `9c29ba6f` against requested `1340c12d`, with PR cache writes disabled.
Native-input and intervening dependency/feature changes make this a mixed-graph
observation, not an unchanged-dependency comparison with PR51's 21m19s Windows
result. Queue time remains separate from execution.

On Windows the ordinary test step took 772s, including 680.87s compilation and
approximately 91s test execution/overhead. GPU verification took 149s, explicit
tools 98s, the required fresh shipping proof 107s, and cache restoration 35s.
The log has 21 workspace/98 other package compilation messages; Cargo reports
147/121 actual units, including build scripts, test harnesses and feature variants.
These two counts must not be interchanged.

The longest ordinary Windows unit is the default editor binary at **276.28s**;
headless is 87.09s, tauri-utils 88.69s and wgpu-core 84.94s. The later tool graph
adds seven units, dominated by the custom-protocol editor at **79.30s** and its
Tauri variant at 14.84s. Unit intervals overlap, so their durations cannot be added
as wall time. Binary entries expose no frontend/codegen/link sections; the large
binary durations cannot honestly be called linker time alone.

Linux recompiles the default editor once for GPU verification (25.37s); macOS
recompiles its binary and test harness (38.64s combined span). A scoped local
fingerprint reproduction motivated the [build-context fix](tauri-build-context.md).
Windows already compiles zero GPU units. That fix therefore addresses a verified
build-correctness issue and a possible Unix saving, not the dominant Windows cost.

Next Windows work should investigate the editor target layout and dependency
fingerprints before restructuring tests. Default/custom behavior, process/global
state isolation, exact GPU identities and fresh owned-runtime shipping proof must
remain. Workspace artifact caching, shipping dependency caching, test-target
consolidation and coverage selection are not implemented by this evidence slice.
