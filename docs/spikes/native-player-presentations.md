# Native player drawable observation — source candidate

This default-off source candidate joins the CPU-qualified HAL seam and main297's
native replay host to main298. The main298 join adds the reviewed animation
consumer source and its distinct dev-only pollster lock edge; it changes no
presentation-observer bytes. CPU qualification passed; no native-window capture
has run. It supplies no display-rate or performance verdict. The probe qualification and all earlier failures remain in
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

Only after source/CPU review may a separately bounded actual replay establish
whether callbacks deliver enough timestamp coverage. The first live result is
coverage evidence, not an FPS gate, uncapped GPU throughput, or an uninstrumented
shipping performance result.

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
