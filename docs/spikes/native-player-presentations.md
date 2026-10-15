# Native player drawable observation — partial live qualification

## Main303 composition

The integration joins this observer with main303's typed audio-overrun policy.
Source `b94c6c7c62ae425074ef1928cb19559e0b958582` and integration
`9719bea79bdc5a88e891df275fe6f815b83a1e1d` have identical trees. The only
overlapping behavioral integration adds the current-owner audio diagnostics
inside the observer's existing report closure; shutdown, callback drain and
original-error precedence remain intact. Reviewed main changes, including the
default-off navigation diagnostic, retain their existing implementation.

All 149 focused inputs were verified. Observer-enabled player tests (105), normal
player tests (87), audio tests (39), strict all-target player lint in both modes,
and HAL lint pass. All owned commands exited successfully and were reaped.
[Composition manifest](evidence/native-presentation-audio/manifest.json) binds
exact logs, feature graphs, copied test binaries and the resolution diff.
The producer packet below remains unchanged and replayable. Its original live evidence
predates the audio fix. The [fresh complete replay](native-presentation-audio.md)
now verifies gameplay parity while retaining timing limitations.

The first PR integration check found a formatting conflict in a test shared by
Rust 2021 and 2024 modules. Binding its existing result before asserting removes
that conflict; both format checks pass. This changes no production behavior or
test expectation. The [retained failure and correction](evidence/native-presentation-audio/format-followup.json)
remain separate from the exact earlier CPU and shipping source identities.

## Original observer producer

This default-off source candidate joins the CPU-qualified HAL seam and main297's
native replay host to main298. The main298 join adds the reviewed animation
consumer source and its distinct dev-only pollster lock edge; it changes no
presentation-observer bytes. CPU and private shipping-build qualification passed.
The second actual window attempt delivered 1,415 positive drawable timestamps,
but failed on an audio backend notification before replay completion. The first
window attempt timed out with no sidecar records. Neither is a complete replay or
display-rate/performance verdict. The probe qualification and earlier failures remain in
[native-presentation-observer.md](native-presentation-observer.md).

The trusted native host opts in with `native-presentations` on macOS and both
`--presentation-output FILE` and `--presentation-capacity N`, plus the existing
`--stop-after`. N is 1..36000 and bounds observed acquisition attempts, including
startup/redraw/deferred attempts; it is independent of accepted host steps. The
stop-after limit is not a wall deadline. A separately reviewed managed runner is
still required for actual execution. Output is create-only and never sourced
from a project or package manifest. Unsupported platforms/features, incomplete
options and output collisions refuse before gameplay; backend/API availability
is checked on the actual surface before its first acquisition.

The engine uses the pinned, licensed `third_party/wgpu-hal` path patch; no registry
mutation is needed. Exact upstream file hashes and complete patch are retained
there. Normal exporter feature selection is unchanged: summary profiling does
not enable presentation capture. Vendoring changes honest dependency/build
identity, even with observation off. Any future diagnostic export must retain
its actual fingerprints, bytecode/data identity and instrumented source receipt;
no exporter admission is bypassed or broadened in this slice.

`SurfaceState::reserve` remains the sole get-current-texture owner. Observation
binds immediately before each existing acquisition and clears immediately after;
there is no extra acquisition, command buffer, display poll, present, frame wait,
focus manipulation or pacing change. Failed/deferred acquisitions have counters
and host attempts, not fabricated drawable rows. Successful cancellation uses
the original HAL discard; callbacks keep immutable acquisition context.

Every row links to an attempt and a segment. Attempts distinguish initial,
background, UI-resize and gameplay roles, session generation and requested next
host step. Only the existing accepted-host-step point marks a gameplay attempt
committed. A presented drawable does not prove that a host transaction committed;
errors before that point leave the attempt open. Refresh completion is separate.
World tick is not used as an identity because restart resets it. Segments break
on focus/suspend/resume, resize/configure, retry/recreation, restart, role changes
and observed inactivity. Recreating the native surface advances its generation.
All boundary counters are retained, including coalesced boundaries with no row.

Before gameplay the collector reserves at most 36000 rows (<=64B each), host
metadata reserves N attempts (<=48B) and 2N segments (<=40B each, matching two
bounded acquisition tries). Total record storage is bounded by 192N bytes,
6,912,000 bytes at maximum, excluding fixed headers/capacity rounding. Capacity
exhaustion stops registration and records omission; it never stalls rendering.
Callbacks do not allocate rows or retain window/application/device objects, but
block creation/copying and Arc/mutex work have per-present overhead. Snapshot,
JSON values and output allocation occur only after gameplay, bounded by record
count; serialized output refuses beyond32MiB. They are not frame measurements.

After the loop, the normal termination/report work retains its original result,
audio is stopped, the final reservation is cancelled and observation is unbound.
The host keeps its existing surface/renderer/window alive for a maximum requested
one-second callback drain; scheduler overshoot is reported, not hidden. It waits
in <=10ms sleeps and does not poll/flush GPU. Requested callbacks still missing
remain missing. There is one final snapshot; late callbacks cannot alter its
serialized copy. Setup exits produce an explicit undrained/unfinalized sidecar.
An error writing the sidecar never replaces the original game/window failure.
The sidecar does not guess the later CLI/process exit status. Normal report,
replay completion and observation checks remain independently required.

The CPU qualification covers native-presentations+profile player
library tests and strict all-target lint, the normal native-window+profile
library tests and strict lint, including the eleven pure HAL collector controls compiled from the exact
vendored source, plus direct HAL lint (selected together with the player workspace
member). This is five serial commands; upstream HAL example/dev tests are not
selected. Use pinned
Rust1.99, arm64 CI, jobs2/Rayon2, locked/offline and coordinated target ownership;
900s/child, stop first failure,6GiB free floor and2GiB sampled target growth.
Actual on/off dependency features and copied binaries/list output must be bound.
No native window/GPU/listener is included. The authored controls cover attempt
versus commit identity, restart/focus/resize/retry generations, bounded metadata,
all acquisition-refusal counters, create-only output, early/error reporting,
output cap, error precedence and CLI/package authority.

Actual replay remains separately bounded and reviewed. The first live result
below establishes partial callback delivery, not an FPS gate, uncapped GPU
throughput, or an uninstrumented shipping performance result.

## CPU qualification, 2026-10-14

Executed source `8dbc74c4191f1e793e3e593ee68bdae28665e3ae` on main298 passed
105 observer-enabled and 87 normal-window library tests (zero failures/ignored),
including the 20 added pure controls. Strict all-target player Clippy passed in
both modes, and strict HAL library Clippy passed. Both copied binaries and exact
`--list` outputs are retained. Actual graph differences are only the player
`native-presentations`, HAL `diagnostic-presentations`, and objc2-metal
`objc2-core-foundation` feature recipients. These are CI CPU checks, not shipping
or presentation measurements.

The first joined attempt002 refused a missing lock edge before compilation;
offline resolution added only objc2-metal's existing pinned core-foundation
dependency. Attempt003 then exposed private enum re-export visibility and the
included collector model's missing direct dev dependency. The successor scoped
the four enums to `crate::window` and added already-locked parking_lot0.12.5
only to player dev dependencies. Both failed generations and exact logs remain.
No algorithm, scheduling, bound or test assertion was relaxed. Attempt004 passed
all five commands in93.845s. All attempts together consumed242.270s and
992,948,965B of sampled target growth; minimum sampled free space was
120,854,564,864B. The target and builder lease were released.

External receipt: `~/.codex/incant-evidence/native-presentation-player-001/cpu-final.json`.
Its immutable source freeze, plans, results, dependency graphs, compiler messages,
binary copies and original failures remain adjacent. Final result SHA256:
`e611b72b6f108096fc2914b00e489c00e532e4b462777658615e9d0c83bde9b9`.
The later documentation commit changes no qualified production source.

## Private shipping and live outcomes, 2026-10-14

Source `1ff68940d0dae14176d54a219d2d719039409e99` produced copied executable
`cc3b316b08d633c0be04144c32b1eb54a8402aaae0e5492e1543c832f55df5e0`.
Rust 1.99, arm64, fast4 physics, optimization3/fatLTO/abort/one codegen unit and
normal window/profile selections were retained. The three observer feature
additions were verified against actual compiler artifacts. The public exporter
was not broadened: a private adapter reused its cook/build/package helpers and
labeled this package ineligible for distribution or performance gates.

All 4,507 tracked compiler/helper inputs and 74 authored inputs were bound.
All 28 cooked files (35,365,370 bytes), compiler output, bytecode and hardening
payload matched the retained Core baseline. The honest build ID changed to
`d1eac86337b09f1b65e531800bd5227125b272d668ca7f46a7f746806ecd76ce`.
After deleting the owned scratch target, the one offline startup tick matched
the full baseline report and PCM bytes. That proves offline startup parity,
not live audio-device quality. Build002 completed all 22 managed children in
511.736 seconds. Build001 failed before Cargo because the frozen PATH omitted
the Homebrew rustup shim; its logs and empty scratch remain. The successor
changed only PATH/tool admission and retained the cumulative stage deadline.

Both actual windows used the same app executable, data and 2,156-step physical
recording, a 4,096-row capacity, 120-second child deadline, 150-second stage,
6GiB free-space floor, 4MiB stream caps and 32MiB sidecar cap. Native input was
not blended into replay. Root performed only application activation through CUA;
a concurrent CPU export and one one-second process sample add observer/load
limitations. No performance comparison was attempted.

| Attempt | Actual outcome | Callback evidence |
| --- | --- | --- |
| window001 | Timeout at 120.013s after Raise/title-bar activation; owned process reaped | Precreated sidecar, stdout and stderr empty; no timestamp or completion evidence |
| window002 | Finder Open on the already-owned app; audio backend error at 66.477s; owned process reaped | 1,444 accepted host steps, 1,445 acquired rows; 1,415 positive timestamps, 28 zero timestamps and 2 missing callbacks |

The second sidecar finalized on error after its bounded one-second drain, with
two callbacks still pending. It has zero invalid timestamps, collector overflow,
protocol errors, abandoned or explicitly discarded rows, and no host metadata
loss. Six focus boundaries and four segments remain visible. One initial row
reported zero. Gameplay segments cover steps1–18, 19–33 and34–1444. One adjacent
positive timestamp pair is equal; it is retained and flagged, not repaired or
silently removed. There is no drawable-ID reversal. The external analyzer lists
1,407 strictly positive intervals only between adjacent acquired rows in the
same segment. This limited subset is not a complete cadence distribution: gaps,
zero/missing callbacks, initial/final intervals, focus changes and unfinished
gameplay preclude a complete-run p95 claim. No percentile is calculated.

The exact error was `Audio(Backend("A buffer underrun or overrun occurred."))`.
Pinned [CPAL 0.18.2](https://docs.rs/cpal/0.18.2/cpal/enum.ErrorKind.html#variant.Xrun)
classifies this notification as `Xrun`. Its CoreAudio overload listener does not
stop the stream; the vendored Kira manager also leaves Xrun streams running.
The existing engine `SceneAudio::check_device` makes every queued device event
terminal, so it ended this session. The trace does not prove audible continuity
or identify the cause of the overrun. A separately reviewed typed audio-policy
fix is outside this producer; no source/assertion was changed to pass the run.

The observer foundation has real partial callback evidence. Complete replay,
glitch-free output, complete display cadence and final-game shader-hitch
qualification remain open. Pipeline/shader object prewarming alone cannot prove
absence of later driver compilation. This packet makes no such claim.

## Later observer/audio composition

A fresh build with the separately reviewed typed-Xrun policy subsequently
completed all 2,156 replay steps. Its retained zero/equal callback timestamps
still prevent complete display timing acceptance. See
[native-presentation-audio.md](native-presentation-audio.md) for the distinct
source, artifact, full gameplay parity and limited callback result. The two
failed attempts above remain unchanged historical outcomes.

## Portable read-only replay

`evidence/native-player-presentations/` retains plans, source copies, actual
compiler logs, successful CPU/build receipts, all failed generations, raw
callback rows, CUA interventions and process-sample evidence. Native executables,
cooked payload and startup PCM remain external and hash-bound; the large dSYM
is retained externally and recorded by the build receipt. The verifier does not
extract files or execute any archived code or native program:

```sh
python3 -B docs/spikes/evidence/native-player-presentations/verify.py
python3 -B docs/spikes/evidence/native-player-presentations/verify.py --originals --binaries --git-source .
```

Default replay verifies recorded identities and replays source/build/partial
callback correspondence. The optional flags verify original retained files,
external binary/data/PCM bytes and exact qualified Git source objects. It keeps
both live attempts failed. There is no full live observation report or live PCM
parity to verify.
