# Fresh Core wake/acquisition diagnostic

On 2026-10-17, a fresh shipping-profile diagnostic player completed the real
2,156-step Core win route on Metal. Its eight checks and 17 entity observations
exactly match the same-package offline CPU replay. The offline replay also
matches the complete historical logical report and all 13,798,400 PCM bytes.
This supplies a current full rendered-game diagnostic after authorized disk
cleanup; it does **not** qualify uninterrupted presentation, FPS, appearance,
performance, or a phase gate.

The native child exited zero. The original supervisor remains **failed**: its
historical audio decoder rejected the current `route_change_notifications`
field. A separate strict four-field decoder and seven refusal/type/gameplay
controls pass. Saved-data replay verifies all previous gameplay assertions,
reconstructs the v2 analysis and independently checks raw interval arithmetic.
No native retry, compiler change or relaxed unknown-field rule was used.

## Exact producer and fresh rebuild

Producer `a621ef024c6012e3ab9e43be39afc501ea57f771`, tree
`612fd7d39d469c59278ec52e878f21805c7e98c8`, is the reviewed main379 tree.
This report is composed on later main384; it does not relabel the executable as
that later source. The player SHA256 is
`757e69d0d358602c0ffc4c53ac1765daa12793a60043178b6e56f10f5acbf1d7`;
the fresh CI headless producer is
`419731918c9e46ba1a4279b2f71521c3df421b0a6a8329f4fe60a478afab2a9d`.
The inventory binds 4,725 source files, 193 compiler-package files and the actual
selected Xcode/SDK/tool closure. Rust 1.99 arm64, shipping opt3/fat LTO/abort/one
codegen unit, fast4 physics and summary profiling are audited from actual units.
The trusted adapter adds the existing native presentation/compilation observers;
it is not an ordinary public export or a distribution/performance artifact.

The recovery archive contained text, not reusable generated executables. Existing
art generation and tracked fonts reproduced 46 runtime inputs exactly. Fourteen
real public imports regenerated the missing payloads without changing project
bytes; all 23 cooked cache hashes match history. Missing old journal/backup files
were not fabricated. The owned CI target was retired before the fresh shipping
build; all owned build/input storage was removed before one CPU startup tick.
Startup `frames=800` means PCM sample frames, not 800 gameplay ticks.

The build completed in 861.38 seconds, plus a conservative ten-second setup
charge. Its earlier missing-container preflight refusal is retained and started
no native child. Scratch peaked at 4,036,461,047 bytes, package at 475,154,991,
evidence at 40,502,882 and aggregate logs at 84,267,830, within the declared
8 GiB/1 GiB/512 MiB account and six-GiB floor. All 71 managed calls completed with
confirmed cleanup; the wrapper and owned groups were independently absent.
The separate full CPU replay took 6.48 seconds, functionally overlapping another
CPU lane; it is not a timing measurement. Its PCM SHA256 is
`ccabf2c68b6388ac554ac3efb5609ddf6b4a45d1a3f316891d5f344094e99ee3`.

The older build-policy refusal remains historical: coroutine integration changed
bytecode/compiler identity, so it was not an observer-only difference. The fresh
build uses the actual later compatible producer metadata. Selected Xcode is now
27.1 with SDK27.0; 63 of the 487 historical tool/resource hashes differ. No matched
old-vs-new speedup or observer-only timing attribution is claimed.

## Observed boundaries and presentation limits

The sole native child ran 45.43 seconds; the wrapper closed at 45.61 seconds.
It rendered at physical 1280×960 with seven drawable HUD widgets, completed all
2,156 accepted steps and wrote a complete v2 capture: no missing/invalid wake or
acquisition samples, no overflow, one initial surface deferral. Both owned
processes/groups were independently absent before analysis. No screenshots or
audible output were reviewed. All three audio notification counters are zero,
with saturation false; those counters do not prove audible PCM continuity.

| Observer-inclusive interval | Valid samples | Nearest-rank p95 |
| --- | ---: | ---: |
| Scheduled pulse lateness | 2,152 | 1.088125 ms |
| Native drawable acquisition call | 2,156 gameplay | 0.028958 ms |
| Entire reservation | 2,156 | 0.148584 ms |
| Committed body | 2,156 | 4.149250 ms |
| Adjacent host-finish interval, continuous segments only | 2,152 | 17.628709 ms |

On the same largest 108 wake-lateness rows, signed pre-`NewEvents` time accounts
for 117,307,624 of 129,712,917 ns (90.436%); the remainder is after that boundary.
The largest wake outlier is different: it arrived 190,126 ns **early**, then spent
8,695,750 ns before the host pulse. Early arrival is retained, not clamped away.
On the same largest 108 reservations, acquisition accounts for 7,112,049 of
21,168,413 ns (33.597%); other reservation work is 66.403%. These are same-row
partitions, not subtraction of independent p95s or exclusive CPU/GPU costs.
`NewEvents` arrival cannot isolate Tao timer latency from OS/run-loop scheduling.

Focus was lost at steps 376, 419, 639, totaling 6.719110 seconds until gains. The
five zero presented timestamps belong to initial 0, step 376 before segment 3→4,
step 639 before 5→6, and resumed 640/641. Apple's `presentedTime` contract makes zero
not-yet-presented/frame-dropped ambiguous. All eight continuity breaks remain;
419→420 is a segment break even though both callbacks have positive timestamps.
There are 2,152 positive callbacks and five zeros, with no missing, invalid or
overflow rows. This focus-interrupted diagnostic cannot close a continuous-display
or quiet rendered-game gate.

The 2,148 valid adjacent positive intervals within segments fit an empirical
8.333333-ms quantum within 167 ns: 28 one-quantum, 2,095 two-quantum and 25
three-quantum intervals. Thirteen long intervals are immediately followed by one
quantum. The largest 25-ms intervals also occur away from focus boundaries
(237→238, 557→558, 617→618), so focus alone does not explain them. This fit is not an
independent display-refresh setting measurement.

A concrete, unproved scheduling hypothesis is phase interaction between the
independent 60-Hz rational game deadline and quantized presentation. Sustained
drawable starvation was not reproduced here. First use the already-required,
separately granted uninterrupted same-player route with the existing observations:
predeclare long/short adjacency, same-row host/wake/body/acquisition comparisons,
all zero outcomes and segment breaks. No observer expansion or scheduling/FIFO/
Tao/HAL change follows from this packet. Any focus interruption, missing coverage,
native/gameplay failure or zero timestamp remains visible and prevents a full
presentation qualification. Native call spans omit later GPU/driver work; no
shader-stall, cache-hit or causal rendering conclusion follows from zero observed
gameplay creation calls.

## Portable replay

The [manifest](evidence/native-wake-acquisition-live/manifest.json) binds 59 compact
text records, raw native report/presentation/compilation/state data, actual build
and CPU receipts, original failures, strict decoder successor and cadence mapping.
It retains source/tool/input inventories, without copying binaries, assets,
node_modules, full PCM or bulky compiler logs into Git. Optional local replay
checks those exact external artifacts and logs. Archived Python is data only;
the verifier executes only its own code and hash-pinned checked-in v2 readers.

```sh
python3 -B docs/spikes/evidence/native-wake-acquisition-live/verify.py
python3 -B docs/spikes/evidence/native-wake-acquisition-live/test_verify.py
python3 -B docs/spikes/evidence/native-wake-acquisition-live/verify.py --originals --binaries --git-source a621ef024c6012e3ab9e43be39afc501ea57f771
```

`--source-root` is valid only for unchanged executable/tool inputs. It is not an
assertion that arbitrary later main revisions produced this binary. The previous
[v1 complete trace](native-presentation-host-stages.md) and
[v2 observer CPU qualification](native-wake-acquisition-boundaries.md) remain
separate historical evidence.
