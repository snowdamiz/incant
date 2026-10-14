# Numeric-loan cleanup and retained entity lifetime checks

The engine reuses a successfully validated numeric hierarchy during loan cleanup
and checks retained opaque entity tokens directly. The current-main composition
contains exactly the six reviewed Rust source/test paths from producer
`03bc4b987f3e553249846c4cfce991793d4d2c48`. Independent source and CPU reviews are
retained in the [portable packet](evidence/numeric-loan-main262/manifest.json).
Final integrated review is independently accepted; publication requires the
exact-head development check. The unchanged-tree ancestry merge preserves the
reviewed original producer objects for Git-source replay.

## Correctness and current composition

Every mutable numeric visit invalidates prepared state before invoking user code.
Validation clears that state first and permits publication only after the whole
hierarchy passes. Cleanup retains the original change ticks, root matrix
arithmetic, and fallback recomputation. Error, rollback and unwind controls check
that stale scratch cannot be published; successful cleanup matches matrix bits,
snapshots and save bytes. No new per-entity allocation or external mutation path
is introduced.

Retained bindings still check each entity lifetime. The opaque token must match
the ECS world, generational entity and live stable-ID component. Reparenting and
archetype moves preserve a valid token; despawn/reuse, foreign worlds and successful
restore cannot revive it. Refused restore leaves the original token valid.

Source002 passes **466 tests across 42 harnesses in each numerical mode**, with
zero failures or ignored cases. Both strict all-target Clippy commands, native
compiler-free script check, wasm runtime check, formatting and diff check pass:
eight commands and 84 immediately copied test executables. Earlier source001
passes 171 library plus 17 binding cases per mode and strict lint; its six copies
and eight receipts remain a separate generation. An initial formatting diagnostic
is retained, not relabeled as a successful execution.

The private integration starts at main262 `4fad2c83` and applies only the two
reviewed source commits. All **3,874 executed build inputs** match exactly.
Other differences from the producer are existing documentation and the isolated
AOT probe. No test execution is relabeled as a new current-main run and no extra
native build was performed during this audit.

## Actual matched Mac shipping measurements

The baseline is source `051e0de7`; the candidate is `03bc4b98`. Both use the exact
same 74 authored input files, script source identity, cooked runtime data, input
recording, observations, compiler, shipping profile and selected fast physics
features. Besides the six Rust paths, the source comparison records only the
already reviewed `package.json` local-test wiring. The audit checks the actual
compiler streams: four optimized codegen units, 11 selected feature artifacts and
28 runtime data files per export. Candidate executable SHA-256 is
`a2794edcc1e7b21db3291eebb76cb2cf923290cc18005f91b023fe4de27fa361`.

On the recorded Apple M5 Pro Mac17,9, macOS 27.0.1 (26A434), arm64, AC power,
the predeclared order was B0, C0, C1, B1, B2, C2. Task builders/GPU consumers were
released and the runner recorded no compiler processes before measurements.
This is not proof that all user applications were idle. Each run executes 2,156
ticks with 10,000 bound attachments across two source groups.

| Pair | Baseline script p95 (ms) | Candidate script p95 (ms) | 2 ms gate |
| --- | --- | --- | --- |
| 0 | 2.760001–2.770000 | 2.340001–2.350000 | FAIL |
| 1 | 2.750001–2.760000 | 2.250001–2.260000 | FAIL |
| 2 | 2.930001–2.940000 | 2.360001–2.370000 | FAIL |

These are raw-histogram bounds, not statistical confidence intervals. All three
paired 5% comparisons pass for script p95, player-frame p95 and requested Rust
peak allocation. Candidate frame p95 is 3.100001–3.310000 ms across the runs.
Requested Rust peak is about 76.88 MB; QuickJS allocated/used heap peaks are
unchanged at 6,322,432 / 4,607,766 bytes. No total-memory improvement is claimed.

Both unprofiled controls and all six profiled runs produce identical nonprofile
reports and byte-identical PCM. The observations retain the complete Core win,
four pickups, damage, HUD state, population and six selected stress/sentinel
rows. They are not a snapshot of every entity. All 9,999 stress writes remain in
the unchanged workload.

## Evidence and limits

The packet has 508 records and 131 lossless payloads. Compiler streams, raw
histograms, command receipts, two complete shipping source hash inventories
(20,582 and 20,584 paths), both CPU generations and reviews are retained. Large
executables, symbols, assets, authoring journals and PCM remain external with
exact hashes and lengths. The prior old-emitter timeout and changed-script
comparisons remain historical in their original packets; neither is substituted
for this matched baseline.

Read-only audit:

```sh
python3 docs/spikes/evidence/numeric-loan-main262/verify.py --verify-only --external --git-source
```

Portable replay reconstructs histograms and all comparison outcomes, checks
identity bindings and 12 negative/boundary controls. `--external` additionally
rehashes retained local originals; `--git-source` checks immutable source objects.
No audit command executes a player, compiler, export or measurement.

The script **2 ms requirement remains failed in every candidate run**. This is
CPU-only actual Mac shipping evidence, not FPS, rendered GPU, AOT adoption, or
full-phase approval. Other-device measurements remain deferred and unclaimed.
