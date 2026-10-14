# Matched Mac fast-mode Core 10k shipping measurements

The immutable static-emitter baseline and the same-input cache candidate each
complete the 2,156-tick Core win with 10,000 bound script attachments. All three
cache comparisons pass the 5% regression checks, but **every candidate fails the
2 ms aggregate script p95 requirement**. This records a partial CPU result, not a
phase approval, displayed frame rate, GPU result or whole-engine performance gate.

The machine reports Mac17,9, Apple M5 Pro, 48 GiB, arm64, macOS 27.0.1 build
26A434, AC power. Low-power-mode reporting was unavailable. The public exporter
selected `fast`, `shipping`, summary profiling and the headless CPU renderer
input path. Actual resolved physics features are retained; `fast` is not a claim
that every platform uses SIMD8. Rust 1.99.0/LLVM 23.1.1 and actual compiler command
lines establish opt-level 3, one codegen unit, panic abort and fat LTO on the
player executable.

## Matched results

The predeclared order is B0, C0, B1, C1, B2, C2. Each candidate compares with its
immediately preceding baseline. Baseline repeats also compare with B0. The table
shows conservative bounds from the raw 10 µs histograms, with nearest-rank p95
recomputed independently. No initial frames are excluded.

| Pair | Baseline script p95 (ms) | Cached script p95 (ms) | Baseline CPU frame p95 (ms) | Cached CPU frame p95 (ms) |
|---|---:|---:|---:|---:|
| 0 | 5.430001–5.440000 | 4.400001–4.410000 | 6.460001–6.470000 | 5.300001–5.310000 |
| 1 | 5.400001–5.410000 | 4.830001–4.840000 | 6.320001–6.330000 | 5.800001–5.810000 |
| 2 | 5.370001–5.380000 | 4.560001–4.570000 | 6.280001–6.290000 | 5.460001–5.470000 |

Requested Rust allocation peaks are 76,880,695 / 76,880,695 / 76,880,663 bytes for
the baseline and 76,880,503 bytes for each candidate. These are allocator-requested
process peaks, not RSS or GPU memory. QuickJS heap peaks increase in every pair:
allocated bytes rise from 3,918,064 to 5,364,384 and used bytes from 2,573,274 to
3,962,281 as the candidate retains metadata. This is a memory tradeoff, not a
claim of total memory improvement. Every recorded comparison passes all three
5% checks (script p95, CPU frame p95 and requested allocation peak); each still
reports `script_2ms_p95: fail`, `passed: false`, `phase_gate_passed: false`.
Per-source totals are retained but are neither per-source p95 nor the aggregate
script phase's full cost.

Both source inventories and all 62 authored inputs were frozen and rehashed
after their builds. The same project, two compiled source identities, recording,
observation plan, 28 runtime data files, package settings and machine identity are
required for each comparison. The player executable identity intentionally
changes. The runner records load before/after each child; the pre-execution
receipt records both builds and functional controls terminal, an empty compiler
process scan and no GPU grant. This does not establish that unrelated user
applications were idle. Three pairs describe these executions, not a statistical
confidence interval or universal performance claim.

## Functional and source boundaries

The authoring file has 10,045 entities: the original 46-entity Core and 9,999
additional invisible stress entities. Exactly 10,000 have Script attachments,
under two script sources. Both sources execute on all 2,156 ticks, with no
unwound or unavailable CPU samples. All attached stress rows are written by the
unchanged stress program; observable checks sample six stress/sentinel rows,
the final population and the original player. They do not read back every
entity or prove whole-world state equality.

Both unprofiled controls and all six profiled runs have identical complete
non-profile reports and PCM hashes. Original pickup ticks 127/508/1266/1842,
damage at 664, the win at 2066 with two health/four pickups, result-parent state,
removed pickups and the eight stress checks are independently checked. Each run
produces 1,724,800 audio frames, PCM SHA256
`ccabf2c68b6388ac554ac3efb5609ddf6b4a45d1a3f316891d5f344094e99ee3`.
No screenshot or display/GPU execution is part of this packet.

- Baseline: `3c81bbe6763f78c10c7d13d08975f2ffeff7caa3`, static binding emitter only,
  player `ada5fdf6df63825f970e7f2ab96cfb4fb01a8f9d4dc0129cf3355bbbebb44243`.
- Candidate: `031a7173ac865a99964e12538926e02ca64ccd71`, the accepted static emitter
  plus attachment and bulk-query row-ID caches, player
  `ba7ad28ee7e0c198d4bf24e9efea5618b06f0d6b3ebc95cec4cec530928b0acf`.
  Its implementation is merged in PR260. These measurements remain tied to the
  executed source, not relabeled as a fresh main260 run.

Neither cache changes the authored scripts or numeric-view lifetime. Any later
sample-script optimization changes the workload identity and requires separate
functional and measurement evidence.

## Failed original compilation and audit

The old-emitter source `bff83af61807435926b10d77b5a04718a464aae5` attempted the same
62 authored inputs but timed out after the existing 2,400-second compiler bound.
Its 3,520,096-byte generated `behavior.rs`, complete compiler streams and failed
export receipt are retained. The unfinished command has a null exit field and
the export explicitly reports `TimeoutExpired`; it is not a runnable baseline.
The successful static-only and cached public exports took 308.934 and 304.175
seconds respectively. These are observed export durations, not controlled
old/new compiler benchmarks or a claimed build speedup.

[Manifest](evidence/mac-core-fast10k/manifest.json) and the lossless archive retain
raw command/functional/measurement reports, preparation/source inventories,
compiler logs, old generated Rust, protocol review and actual tooling source.
Large native executables, symbols, binary assets and PCM remain externally
hash/size-bound. The read-only audit checks all retained bytes, actual Cargo
artifacts/codegen, identities, source maps and every raw histogram. Twelve
negative/boundary controls reject malformed histogram mass, changed workload
identity and uncertain threshold passes. Replays never invoke a player, build,
export or measurement:

```sh
python3 docs/spikes/evidence/mac-core-fast10k/verify.py --verify-only
python3 docs/spikes/evidence/mac-core-fast10k/verify.py --verify-only --external --git-source
```

The second command rehashes every original external file and both complete Git
inventories (20,542 and 20,580 paths) against immutable commits. It does not rely
on producer worktrees remaining on those sources. The historical freeze's
`input_receipt_sha256` binds `fast-stress-002/receipt.json`; the distinct
`shipping-inputs.json` binds copied recording/observation files. Initial auditor
assumptions that these receipts were the same file and that observation output
preserved input ordering were corrected; neither changed engine evidence.

Independent [final packet review](evidence/mac-core-fast10k/review.json) accepts
the source/build/functional/math scope and full external/Git replay. Mobile/other-device measurements
remain deferred; this result leaves the Mac script budget failed and the
separate display/GPU, hitch, memory-total and human phase decisions open.
