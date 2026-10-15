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

The second-pair cause remains unresolved. The failure sentinel was identified
from the pinned Metal definition and exercised synthetically; it was not observed
in either device run. This correction validates decoder behavior, not the
availability or accuracy of every backend's counters. Release CI and the
rendered-game performance gates remain separate.
