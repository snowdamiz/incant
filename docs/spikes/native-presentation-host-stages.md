# Native presentation host stages

The default-off `native-presentations` observer now retains the host clock stages
needed to distinguish scheduler wake lateness, work before an attempt, surface
reservation, committed gameplay, and work after commit. This extends the existing
bounded observer; it does not change the fixed-step clock, admission, input,
rendering, presentation, or audio policy.

Rust source `b4033db1fb675648a0e212c9d81ef6634d567d20`, based on
`30a8d8e2e53168b342cbf844f99fb6730ca676d7`, passed CPU qualification. The five
qualified Rust files remain byte-identical in the fresh shipping producer described
below. No live window or GPU was run for that CPU qualification. In particular, the earlier complete
Core callback observation predates these host-stage fields and cannot establish
their live coverage or diagnose its cadence. There is no performance or phase-gate
claim. The later incomplete live observation below establishes partial host-stage
coverage only; the complete route remains unqualified.

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
qualified complete live run is needed to assess the full route, with every
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
requests 640×480 **logical** units; no native window was opened during CPU replay,
so that replay observed no physical extent or scale factor. CPU replay does not exercise drawable
acquisition, host scheduling, native shader creation, display callbacks, device
audio, or the new host-stage fields. The later partial run below does not establish
complete displayed cadence, visual acceptance, shader-stall qualification or a
performance/phase-gate pass.

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

## First live observation: incomplete after focus loss

The unchanged `e93ac30a` / `45c53745…` player and cooked package ran the original
2,156-step recording once. It committed **263 steps**, then remained unfocused
with simulation suspended. Root closed its owned window; the native process and
wrapper both exited 1 with `native observation finished before all checks`.
Stdout is empty. The final gameplay checks and native audio diagnostics were not
returned; the prior complete offline state/PCM parity remains separate evidence.
There was no retry. The wrapper was terminal after 100.492 seconds, within the
120-second child/150-second stage bounds; root confirmed owned PIDs/groups
62934 and 62944 absent. Sampled child RSS peaked at 224,804,864 bytes, not a hard
bound on transient, driver or GPU memory.

The state log records focus loss at 6.148713 seconds and 93 consecutive inactive
snapshots at step 263, through 99.612303 seconds; finalization is at 99.870589.
These are coalesced state-log times, not exact focus-event timestamps. That tail
is not a 94-second frame interval. All snapshots report a 640×480 physical extent;
no final UI scale-factor report is available. Observer I/O, CUA actions and the
incomplete route preclude comparison with another run's performance.

There are 265 host attempts: one initial deferral, one initial refresh and 263
committed gameplay attempts with valid stages. All 263 scheduler batches have
one due tick; one is forced resume, and no discarded ticks, host timing errors,
omitted attempts or invalid batches were reported. The retained gameplay subset
has these nearest-rank p95/max host-clock observations:

| Host quantity | Samples | p95 / maximum, ms |
| --- | ---: | ---: |
| Reservation | 263 | 3.019 / 7.284 |
| Committed body | 263 | 4.133 / 17.345 |
| Finished attempt | 263 | 6.632 / 22.465 |
| Scheduler wake lateness | 262 | 1.047 / 5.844 |
| Adjacent host finishes | 262 | 17.809 / 26.252 |

For the actual longest interval, steps 126→127, 26.251666 ms decomposes into
13.267625 ms between the prior finish and next scheduler pulse, 0.002125 ms
before reservation, 0.112958 ms reserving, 12.868292 ms committed body and
0.000666 ms postcommit work. Wake lateness for that pulse is 1.037417 ms;
the whole inter-attempt wait is not scheduler lateness. Steps 192→193 instead
include the maximum 7.283500 ms reservation. These individual samples locate
host spans; they do not identify work inside the committed body, deferred driver
compilation or GPU execution. Marginal percentiles must not be added/subtracted.

**This sample does not establish a specific pacing defect or justify an engine
source fix.** Maximum wake lateness occurs at step 2 after the first forced-resume
attempt took 22.465041 ms; only 0.043875 ms separates that attempt's finish from
the next pulse. The recorded lateness is therefore consistent with prior host
work overrunning the next deadline, rather than a 5.844 ms scheduler sleep error.
The body and reservation outliers above identify spans to investigate; no final
profile report is available to separate their internal work.

Callbacks retain 261 positive timestamps and three zero outcomes, including one
initial refresh and gameplay steps 260–261. One adjacent positive pair has equal
timestamps; 258 strictly positive adjacent intervals remain. No zero/equal row is
repaired or bridged into complete displayed cadence. The callback data has an
early delivery/observation discrepancy worth preserving: steps 2→3 have
a 50.023 ms positive callback interval but a 14.760916 ms host-finish interval,
while steps 1→2 have equal callback timestamps. These are separately measured
durations linked only by attempt identity, without cross-epoch subtraction.
They cannot distinguish display delivery, callback semantics or deferred driver
work, and do not support changing submission or wake policy. The native creation collector
retains 91 successful catalog calls (59 libraries, 31 render pipelines, one compute
pipeline), plus four successful unbound-prefix calls. It observes no Gameplay
creation call. Its local `coverage_limited_or_inconclusive=false` describes those
call records; the enclosing run still failed at 263/2,156. It cannot rule out
deferred driver work or establish shader-hitch freedom.

The additive [22-record packet](evidence/native-presentation-host-stages/live-partial-001/manifest.json)
preserves raw sidecars, controller/grant identities, failed receipts and root's
closeout. Its [derived summary](evidence/native-presentation-host-stages/live-partial-001/summary.json)
retains exact stage statistics and five largest individual host intervals.
The verifier recomputes host analysis using the hash-bound repository analyzer;
archived code is data only. Six data controls reject altered timing, route,
focus-tail, creation and gate claims. Earlier CPU and complete offline replay
records are unchanged.

```
python3 -B docs/spikes/evidence/native-presentation-host-stages/live-partial-001/verify.py
python3 -B docs/spikes/evidence/native-presentation-host-stages/live-partial-001/verify.py --originals
python3 -B -m unittest discover -s docs/spikes/evidence/native-presentation-host-stages/live-partial-001 -p test_verify.py
```
