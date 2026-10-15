# Native presentation host stages

The default-off `native-presentations` observer now retains the host clock stages
needed to distinguish scheduler wake lateness, work before an attempt, surface
reservation, committed gameplay, and work after commit. This extends the existing
bounded observer; it does not change the fixed-step clock, admission, input,
rendering, presentation, or audio policy.

Rust source `b4033db1fb675648a0e212c9d81ef6634d567d20`, based on
`30a8d8e2e53168b342cbf844f99fb6730ca676d7`, passed CPU qualification. The five
qualified Rust files remain byte-identical in the fresh shipping producer described
below. No live window or GPU was run for this slice. In particular, the earlier complete
Core callback observation predates these host-stage fields and cannot establish
their live coverage or diagnose its cadence. There is no performance or phase-gate
claim.

## Recorded boundaries

The `host_timing_contract` is version 1, integer nanoseconds from one Rust `Instant`
origin at capture creation. It explicitly has no shared epoch with Metal.

| Observation | Boundary and derived duration |
| --- | --- |
| `pulse_ns` | Entry to `MainEventsCleared`, before polling and eligibility work |
| `deadline_ns` | Existing clock deadline before calculating due work; absent on forced resume |
| `began_ns` | Gameplay attempt immediately before surface reservation; `began - pulse` includes polling, eligibility, possible UI refresh, and earlier catch-up attempts |
| `reserved_ns` | Successful drawable reservation and view creation; `reserved - began` includes any acquisition/reconfiguration work |
| `completed_ns` | Existing host commit after tick, surface consumption, input commit and host-step increment; `completed - reserved` is the committed body, not GPU completion |
| `finished_ns` | After observations, session-request handling and haptics; on restart, after haptic pause and before session replacement |

`finished - completed` includes postcommit work and haptics.
`finished - began` is the finished host attempt, excluding later event-loop cleanup,
waiting and display delivery. These regions do not include driver/GPU execution
after submission. Missing timestamps remain null; a genuine measured zero is
valid. Checked host-clock conversion and stage order failures increment
`timing_errors`. An error after commit can retain a committed attempt with no
finish, and a later attempt cannot complete that old row.

Catch-up ordinals share one scheduler pulse and its original deadline. The
deadline is not an individual target for every ordinal. Repeated
`discarded_ticks` describes the batch once. Forced resume has no deadline, so it
has no wake-lateness sample. Due-zero and inactive pulses create no gameplay
attempt and are outside this observation.

## Read-only analyzer

```
python3 -B tools/presentation_host_stages.py capture.json --max-input-mib 64
python3 -B -m unittest discover -s tools/tests -p test_presentation_host_stages.py -v
```

The analyzer accepts only the emitted version 1 contract and bounded typed fields.
Its selected input ceiling is 32 or 64 MiB, default 64 MiB; output is capped at 64 MiB.
It launches no process, reads no project, and writes only JSON to stdout (refusal
to stderr with exit 2). Duplicate JSON keys, non-JSON constants, unsupported
contracts, invalid numeric types/ranges, inconsistent callback counts and identity
bindings refuse explicitly. Timestamp regressions, missing stages, invalid
scheduler metadata, capacity exhaustion, omissions and partial finalization stay
visible in the result.

Each attempt retains its raw identity, original host fields and callback rows.
The input SHA-256 binds all original bytes. Scheduler batches are deduplicated by
session, pulse and deadline, with matching due/discard metadata and consecutive
ordinals checked. Invalid batches do not contribute a fabricated discarded-tick
sum. A truncated but otherwise valid batch retains its known discarded count
once and is explicitly partial.

Stage summaries expose valid, missing, invalid and not-applicable counts, followed
by sums and nearest-rank p95 over the valid subset only. Pre-attempt spans can
overlap earlier catch-up work; their sum is not exclusive CPU work. Host finish
intervals use consecutive successful, fully timed gameplay attempts with
consecutive host steps, the same session and the same unambiguous acquired
surface/segment. Refreshes, partial statuses, unmapped/multiple acquisitions and
boundary segments break the chain. The analyzer never subtracts Metal seconds
from Rust nanoseconds and does not calculate displayed cadence.

`complete_requested_host_timing` is only a consistency statement about retained
host rows and finalization. A process-exit receipt and gameplay/source/package
bindings remain external requirements, even when this field is true.
`performance_gate_passed` and `display_cadence_measured` are always false. Missing,
zero, equal or invalid Metal callbacks remain raw evidence, not host-duration
corrections or silently removed display gaps.

## Bounds and qualification

At most 36,000 attempts and callback rows and 72,000 segments are retained. Static
element bounds are 192 bytes per attempt, 64 per callback row and 40 per segment:
12,096,000 bytes at the maximum. This excludes vector headers, callback blocks,
snapshot and JSON copies. The final writer's 64 MiB output ceiling is not an RSS
bound. Per-present callback registration has allocation/ownership overhead;
neither this observer nor the analysis proves zero measurement overhead.

The exact CPU command/feature inventory, source freeze, independently reviewed
source, raw logs and copied-test identities are in the
[portable manifest](evidence/native-presentation-host-stages/manifest.json).
Rust 1.99, `aarch64-apple-darwin`, CI profile, locked/offline resolution, two jobs and
Rayon 2 were used. The observer-enabled library passed 148 controls with
`native-presentations,profile,native-compilation`; ordinary `native-window` passed 92.
Strict all-target Clippy passed both feature selections; rustfmt, conventions and
diff checks passed. All managed children returned after cleanup. No native test
failure occurred in this chain.

The pure-data analyzer has 18 passing controls covering exact stage arithmetic,
catch-up deduplication, contradictory/truncated ordinals, forced resume and zero,
partial statuses, missing fields, clock regression, acquisition and segment
boundaries, malformed contracts/numbers/identities, callback contradictions,
bounded input and independence from Metal's epoch. These synthetic controls do
not establish live completeness.

```
python3 -B docs/spikes/evidence/native-presentation-host-stages/verify.py
```

Default replay verifies the portable records, frozen inputs, raw CPU results,
analyzer controls and the shipping/replay results below. `--originals` additionally
compares retained local originals; `--binaries` hashes the two external copied test
executables and the shipping player without running them. The verifier never compiles or launches the engine. A later explicitly
qualified live run is needed to attribute actual host/display gaps, with every
boundary, pending/zero/equal callback and observation overhead retained.


## Fresh shipping build and full CPU replay

Producer `e93ac30a9cce9deadd4013ac5fa7abbc3e60a2ac` completed one fresh private
shipping build and its offline one-tick startup check. The frozen inventory binds
4,539 compiler/tool/vendor files and the same 74 authored Core inputs. The prior
qualified host-stage files are unchanged. This producing source is distinct from
later documentation or localization changes; its package remains frozen.

The actual compiler is Rust 1.99 for `aarch64-apple-darwin`, with the original
shipping fat LTO, one codegen unit, abort-on-panic and fast four-lane physics.
The actual dependency graph admits only the four reviewed observer recipients:
`incant_game_player`, `incant_render`, `wgpu-hal` and `objc2-metal`. Five target
compiler units, including both player library and binary, carry exactly those
features. No runtime compiler or native checkpoints were enabled. The private
adapter reuses the normal cook/export helpers and preserves genuine compatibility
identities; this artifact is ineligible for distribution or performance gates.

| Retained identity | SHA-256 |
| --- | --- |
| Producing source inventory | `1e05e5045d1b89ddd2b5989f947c87f10048983171f798a29f9ea3824d60acf6` |
| Completed build report | `e4a314d206c33c4894f329da8a157223dc7b10dd5bdfbfc968dcc270c06f9096` |
| Copied player | `45c5374583b7543af7e4b5d77a6776d79f7a5eeb3c945c9b640adf5b3a970ca6` |
| Full CPU replay result | `da287124ef1f719bd1c710a62925a25fd50edce9de4dd691a424d1cd444fec0c` |
| Full PCM | `ccabf2c68b6388ac554ac3efb5609ddf6b4a45d1a3f316891d5f344094e99ee3` |

All 22 managed build/startup children returned zero after cleanup. The fresh
scratch target was removed before startup; all 82 retained artifacts were
rehashed. Sampled free storage stayed above 98.65 GB. The stage lasted 477.484
seconds within its 3,000-second bound, with no retry. Storage and cancellation
checks are cooperative and sampled, not hard preemption of filesystem calls.

The separately granted, unprofiled CPU replay ran exactly 2,156 ticks. Its complete
logical report matches the retained Core baseline byte for byte, including eight
checks and 17 entity observations. The 13,798,400-byte PCM matches exactly; stderr
is empty. Its single child returned zero after cleanup in 5.539 seconds, within
the 45-second child and 90-second stage limits. This duration is an execution
receipt, not a benchmark or script-budget result. No source, package, workload,
threshold or baseline was changed for the replay.

The offline render-input viewport was 640×480. The producing native-window source
requests 640×480 **logical** units; no native window was opened here, so no physical
extent or scale factor was observed. CPU replay does not exercise drawable
acquisition, host scheduling, native shader creation, display callbacks, device
audio, or the new host-stage fields. There is still no actual native host-stage
observation, display-cadence result, visual acceptance, shader-stall qualification
or performance/phase-gate pass.

The expanded packet preserves the original 48 CPU/analyzer records and adds the
fresh plans, grants, raw logs, source inventory, exact selected compiler events,
startup report and complete CPU replay. The compressed archive is about 2.8 MB;
executables, PCM, cooked binary assets, debug symbols and the full verbose compiler
stream remain external. Default replay verifies recorded PCM and executable
identities; it does not claim to rehash bytes absent from the archive.

```
python3 -B docs/spikes/evidence/native-presentation-host-stages/verify.py
python3 -B docs/spikes/evidence/native-presentation-host-stages/verify.py --originals --binaries
python3 -B docs/spikes/evidence/native-presentation-host-stages/verify.py --source-root /path/to/preserved-e93-checkout
```

`--originals` also rehashes omitted raw/cooked/PCM artifacts, checks all 74 authored
inputs, and reconstructs the exact receiver-event subset from the retained full
compiler stream. `--source-root` compares all 4,539 source files against the frozen
inventory without changing that checkout. These optional modes require the
retained local originals; missing files refuse explicitly. The verifier never
executes archived scripts, compiles code, launches the engine or starts capture.
