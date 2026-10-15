# Refuse unavailable GPU timestamp counters

The optional GPU profiler now refuses an individual unwritten `[0, 0]` query
pair even when other pairs in the same readback are valid. Previously only an
entirely zero payload was refused, so a missing pass could appear as zero work.
It also refuses `u64::MAX` at either endpoint. Metal defines that value as
`MTLCounterErrorValue`; treating equal failed samples as a quantized zero interval
would incorrectly report a measurement.

The decoder retains subtraction in integer space, valid equal nonzero samples,
independent overlapping pass intervals and near-limit counter precision. A
successfully mapped but invalid sample releases its pool slot after unmapping;
only a failed mapping permanently retires the slot. No query scheduling,
rendering algorithm or frame-admission policy changes.

## Qualification

At source `01411a13f8734a105dd2fc0857f41d0fbcde97c4`, all eleven targeted timing
tests pass on Rust 1.99, arm64 macOS, profile `ci`. Strict all-target renderer
Clippy, workspace formatting, generated conventions and diff checks pass.
Controls cover mixed valid/unwritten pairs, failed sentinel endpoints in labeled
and unlabeled pairs, pool reuse, reversed counters and valid equal/near-limit
counters. The copied test executable and exact inputs/results are retained in
[the CPU receipt](evidence/gpu-timestamp-invalid-counters/qualification.json).

Two separate bounded Metal diagnostics returned a valid first clipping pair
followed by `[0, 0]` for the reducer despite exact complete numerical output.
Separating the passes into two query sets with indices 0/1 and aligned resolve
offsets did not remove the missing second pair. Both complete timing attempts
failed and remain retained; no zero duration is substituted. Their first clipping
intervals were 331.149458 and 336.040666 ms for the same small diagnostic packet.
These are cold isolated observations including instrumentation overhead, not a
warmed comparison, full-frame measurement or shipping performance verdict.
[Raw identities and outcomes](evidence/gpu-timestamp-invalid-counters/diagnostic-observations.json).

Those two attempts did not isolate the second-pair cause. The failure sentinel was identified
from the pinned Metal definition and exercised synthetically; it was not observed
in either device run. This correction validates decoder behavior, not the
availability or accuracy of every backend's counters. Release CI and the
rendered-game performance gates remain separate.

## Deferred resolution follow-up

A subsequent diagnostic keeps both sampled compute passes in the same first
submission, waits for its completion within the original bounded device wait,
then resolves and reads the counters in a second submission. The shaders, query
sets, dispatches, input packet and numerical expectations stay unchanged. On
this Mac (Mac17,9, Apple M5 Pro, macOS 27.0.1 build 26A434), both pairs are now
positive and ordered. The complete 59,900 numerical records are byte-identical
to the earlier failed observation.

The raw intervals are 362.333458 ms for clipping and 0.030583 ms for reduction.
This is one cold instrumented observation, excluding pipeline creation, uploads
and readback. It diagnoses counter availability; it establishes neither a
speedup nor a shipping frame budget. The expensive exact-integration algorithm
is not adopted or reopened on the strength of these counters.

The production profiler already defers resolution until sampled work completes.
Its implementation and strict rejection of missing/sentinel counters remain
unchanged. The historical failures and a preparation-only path error remain
retained; the successful helper compilation, CPU controls and GPU child all
completed and were reaped before the later screen-capture pause.

The [compact packet](evidence/gpu-timestamp-invalid-counters/deferred-resolve.json)
retains the diagnostic source/diff, independent review, raw counter bytes,
invocations and comparison records. The verifier recomputes the intervals and
checks those bindings without executing native code. Numerical payloads and the
helper executable remain external by hash; `--originals` additionally rehashes
them. Neither replay mode captures the screen or launches a GPU workload.

```sh
python3 docs/spikes/evidence/gpu-timestamp-invalid-counters/verify-deferred.py
```
