# Shipping observation-cost protocol — proposal

This is a source-only proposal. No shipping build, functional player, or timing
run has been executed. Accepted profiler producer `717c48ba` remains unchanged
on its own branch. The final current-main intake SHA, executable runner and its
inputs require a separate freeze/review before execution.

Use **one freshly exported candidate**, then compare its detailed and maintained
modes. Both arms use exactly the same executable, cooked scene, script artifact,
features, machine and input. Their instrumentation identities remain different.
There is no schema bridge, cross-mode matched 5% verdict, or claim that the
change accelerates guest loops. Profilev2 results remain historical.

## Sources and workload

Accepted implementation is `1b20ad5a9a772d334cb03a4ee1572627813454fa`, based on
main272 `62a3d09ff7d8ae7282c57bc49620950650337b7c`, with the independent final
receipt `0637a7747f030ec4f4bc0f58a4853cf62c3f2115ba82eba2e392d6bb16d9f9dc`.
After the sole publisher integrates it, freeze that exact current-main SHA and
prove the relevant implementation remains identical before building. A branch
name or a guessed future SHA is insufficient. Any integration delta needs review.

The unchanged fast Core workload is the 74-file authoring input retained under
`~/.codex/incant-evidence/mac-core-numeric-loan-shipping-001/authoring`.
Its historical freeze SHA is
`7c60e652f6f54784b0c7410c62555fae57848bed7edc523d34dc2586900b4bf8`.

- Project: `027621691da3043e25cd30a02c76a8fb92f1a4464448c3c6c7d04e6f8b38c30b`.
- Gameplay: `f471293cf6a3b6a355f0f2c8ec6406bf5f3942f438411121b2e63ae6e6f31095`.
- Stress: `7285e9009633052ba86ee0996c87f58097eb47fdaf0a73df10f42597b0119f1a`.
- Exactly 10,000 Script attachments over 2 sources: original controller plus 9,999 bulk instances. Existing lifetime checks, every write/readback, checksum, sentinel, input and VM budgets remain unchanged.
- 2,156 ticks at 60 Hz; fast SIMD8 physics; 640×480 typed UI viewport; 48 kHz offline PCM. All frames, including first-tick setup, remain included.
- Input and observation identities come from the retained `core-script-stress-001/fast-stress-002/shipping-inputs.json` receipt, reproduced in the adjacent JSON plan.

Retained player `a2794edcc1e7b21db3291eebb76cb2cf923290cc18005f91b023fe4de27fa361`
from source `03bc4b987f3e553249846c4cfce991793d4d2c48` is a historical functional
anchor only. The main272 changes since 03bc include authoring/cook/lock evolution
and test-only script tracing. Do not imply an unchanged artifact/build identity
or compare its old timing as a current baseline. The current exporter must recook
against the new conservative script build identity; copying old executable
bytecode into the candidate is forbidden.

## Build and functional prerequisites

The one build uses the unmodified source-free public exporter, summary profile,
Rust 1.99, explicit `aarch64-apple-darwin`, shipping opt-level 3/fat LTO/one codegen unit,
locked/offline resolution, jobs 2 and Rayon 2. Freeze actual compiler, SDK, tool,
vendor, lock, generated artifact and normal-feature closure; retain full compiler
and export receipts and rehash after compilation. No runtime source parsing,
Wasmtime, authoring dependency, GPU or device backend is added. Storage and the
exact target lease must be checked before a separate build grant.

Before quiet timing, run the new package without profiling, with detailed
profiling, and with maintained profiling for the complete 2,156-tick route.
Compare the full reports after removing only `profile`, plus exact PCM bytes and
hash, against one another and the retained original unprofiled report. Run the
existing Core-win/stress oracle. Retain every failure and every actual command.
This preserves existing observations; it does not claim an external full dump
of every row on every frame.

## Six fresh timing processes

Order is fixed: **D0, M0, M1, D1, D2, M2**. Each process uses the identical package
and records all 2,156 frames. No warmup exclusion, failed-sample retry, fastest-run
selection, averaging of quantiles or deadline/budget change is permitted.

D uses `--profile --profile-heap-detail`; M uses `--profile`. Raw profile version 3,
heap version 1 and sampling `script_tick_end_per_vm` must agree with the requested
mode. The consumer must refuse a mismatching or legacy report rather than relabel
it. Require 2,156 heap observations, 0 invalid observations, 32 MiB limit, detailed
sample count 2,156 for D and 0 for M, and null estimated-used fields for M.

Use an explicit reviewed `heap_mode` argument on the existing measurement helper,
leaving its default behavior and strict comparison rules unchanged. The helper
must record the actual argv and verify the returned mode; a hidden subprocess
monkeypatch or forged measurement identity is unacceptable. Source controls should
cover malformed requests, wrong returned mode, legacy responses to an explicit
mode, and unchanged default/legacy ingestion. This small runner seam is still to
be implemented and qualified; the JSON proposal is deliberately not executable.

Before the sequence, require actual all-lane quiet acknowledgments: no builders,
GPU, benchmark/player, bulk hashing or archive work. Capture actual Mac model,
chip, OS, power state and load context. Package/source binding work precedes the
quiet interval; retained helper package checks remain identical between runs.
Bound each process to 45 seconds and the complete supervisor to 300 seconds.
Cleanup/reap every owned process group on failure. A timing error terminates the
sequence with all attempts retained; it does not authorize a retry.

## Interpretation and baseline

`script.tick` remains the 2 ms gate: the complete ScriptHost scope includes host
setup, callbacks, native numeric validation, cleanup/commit, and the selected
heap observation. Do not subtract a mean or an estimate from its p95. Recompute
nearest-rank intervals from raw 10 µs histograms; require all expected successful
samples and no missing/invalid/overflow counts. Evaluate each of the six actual
mode-specific intervals against 2,000,000 ns, preserving fail/inconclusive results.
This remains CPU simulation evidence, not a 60 fps/GPU/complete phase approval.

Cross-mode output is descriptive only: retain D and M identities and report the
interval difference `[D.lower − M.upper, D.upper − M.lower]`. That is a difference
between two independently sampled distribution quantiles, **not** a per-tick or
p95 telemetry duration. Observer-induced cache/scheduling effects remain part of
the observation-method experiment. Never call the strict comparator across D/M.

Existing `script.source` scopes enclose each callback and end before heap telemetry.
Require exactly 2 ordered source rows, each with 2,156 calls, 0 unwind and 0 unavailable
CPU samples. Report their wall/CPU totals divided by 2,156 as contextual means.
They are not callback p95s and do not establish a guest-loop speedup. In particular,
do not subtract their means from a whole-tick p95.

**M0 is the fresh maintained baseline**, selected before execution, even if its
2 ms result fails. Retain M1/M2 as repeated-run context and apply the unchanged
strict 5% comparison to M1/M0 and M2/M0 only. D0 can similarly anchor D repetitions.
These are within-mode stability comparisons, not engine improvement acceptance.
No v2→v3 or detailed→maintained 5% comparison is produced. A future engine change
must match M0's explicit current instrumentation and all existing identity fields.
