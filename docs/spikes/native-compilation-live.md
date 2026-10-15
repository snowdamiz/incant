# Complete native shader-creation observation

The combined compilation/presentation observer completed the Core sample's full
2,156-step route in an actual Metal window on this Mac. All eight checks and 17
observed entities match the retained offline baseline. This closes live coverage
of the six instrumented native creation calls on this route. It does not pass
the shader-stall or displayed-frame performance gates.

## Producing build and actual run

The frozen source is `fd72fa344b8093a02d1b40aafaba53fd390120fc`, tree
`9b6ee583cb87cea192404bab21f1c6c6e1a4e4ec`, byte-identical to main309.
The fresh fast-physics player is SHA256
`b5f92b7613e5081eab90cff84c549318dffb5b41e402a09b4c1599b329974a2f`.
Its actual target compiler units include both explicit observers and profiling;
the shipping profile retains optimization 3, fat LTO, one codegen unit and abort
on panic. Twenty-two managed build calls returned zero, the private build target
was removed, and the ordinary one-tick offline startup matched its logical/PCM
baseline. That startup alone was not hardware evidence.

The subsequent owned window ran on Apple M5 Pro/Metal, completed in 38.214328
seconds and exited zero. The controller returned after child cleanup. This source
still requests 640×480 **physical** pixels; the later logical-window repair is
not part of this build. Its timing cannot be compared directly with the larger
1280×960 CJK run. One actual inline CUA image showed the running English world
and HUD; there is no filesystem screenshot or new appearance acceptance.

## Native construction outcomes

All 95 native creation attempts succeeded. Four calls preceded capture binding
(two Naga libraries and two compute pipelines); their outcomes are counted but
their durations are unavailable. The 91 retained rows all belong to the catalog:

| Native call | Retained calls | Largest observed call |
| --- | ---: | ---: |
| Naga-derived library | 59 | 116.875875 ms |
| Render pipeline | 31 | 43.964333 ms |
| Compute pipeline | 1 | 11.281542 ms |

There were no observed creation calls in gameplay, no pending calls, row overflow,
native failures or clock anomalies. The host records 2,156 committed gameplay
scopes and one abandoned surface attempt. Catalog durations occur outside those
gameplay scopes. These intervals include diagnostic overhead and cannot be
summed into exclusive CPU or display time.

The observer excludes deferred driver work, function lookup, Naga translation,
descriptor construction and queue waits. Zero gameplay calls therefore does not
prove that no frame stalls because of shader or pipeline work. The remaining
investigation is actual frame pacing and CPU/GPU/display timing, rather than
another unchanged native-creation-count run.

## Display and audio boundaries

All 2,157 presentation rows returned callbacks: 2,154 positive timestamps and
three zero outcomes, with no missing callbacks, overflow or protocol errors.
The zeros are the initial presentation and requested steps 1,694–1,695. Three
focus boundaries and one surface deferral remain recorded; zero timestamps and
boundaries break continuity and are never bridged to manufacture intervals.

Within the first gameplay segment, the conditional adjacent-positive display
interval p95 is approximately 25 ms and the maximum 33.333 ms. In the later
segment those figures are approximately 16.667 ms and 25 ms. They exclude the
ambiguous outcomes and boundary intervals, so neither is a whole-run gate result.
The separate host CPU p95 bucket is 4.920001–4.930000 ms; accepted host-end interval
p95 is 17.600001–17.610000 ms. These clocks measure different work and boundaries.
No causality between a display gap and a native creation call is inferred.

The audio host reports zero overrun notifications, poll-limit hits or saturated
counters. This is not an audible-continuity test. Capture registration, clocks,
locks and final serialization add diagnostic overhead; no matched speedup,
ordinary-export performance or phase approval is claimed.

## Evidence replay

The [compact archive](evidence/native-compilation-live/manifest.json) retains
27 original records, including raw sidecars, full gameplay output, baseline,
source inventory, actual feature/codegen identity and independent review. The
two pure-data analyzers are exact copies of those used after the run; their
23-control qualification is retained. Large runtime/package files remain external
by hash. Earlier CPU-only and build-pending receipts stay historical.

```sh
python3 docs/spikes/evidence/native-compilation-live/verify.py
python3 docs/spikes/evidence/native-compilation-live/verify.py --originals --retained
```

The second command additionally rehashes the retained original records, binaries
and package inputs. Neither command launches a player, compiler, native API,
window or GPU. Both analyses must reproduce their saved output exactly, including
all ambiguous timestamps and false gate fields.
