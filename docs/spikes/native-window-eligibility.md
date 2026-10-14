# Native-window eligibility diagnostics

Pearl Cove's fresh shipping package passes the existing 781-step win, 201-step
urchin-loss and 221-step shore/cliff CPU routes. All eleven observation checks,
canonical HUD strings, actor flipbooks and 7,699,200 bytes of retained PCM pass.
This does not establish native-window route completion or presentation timing.

Two actual 781-step window attempts reached their unchanged 90-second child
deadline without a final report. The first included root's normal Finder/AX
activation and viewport interaction; the second had no CUA interaction. Both
owned process groups were reaped and package/recording inputs stayed unchanged.
Root's first attempt separately captured crisp Cove/tile/sprite/HUD tool images
and a visible player position change. Those tool images were not exported as
filesystem PNGs, and do not establish a complete win, animation, focus sequence
or performance pass. Direct execution of the bundle binary, rather than a
LaunchServices-originated process, remains a hypothesis requiring evidence.

The optional trusted CLI `--window-state-log FILE` adds read-only eligibility
snapshots to an explicitly bounded native window. It requires `--stop-after
1..36000`, refuses the lifecycle probe and creates its destination exclusively
before opening devices. Neither package data nor a scene supplies this path.
Existing files and final symlinks are refused; parent directories are trusted
host storage. The planar runner's opt-in flag writes `eligibility.jsonl` inside
its fresh output and retains the log hash even when the child times out.

The log records cached focus alongside Tao's `Window::is_focused()` query,
platform and simulation suspension, minimization, supported physical dimensions,
surface reservation outcomes/counters and pending retry time, session generation,
host steps, world tick and monotonic elapsed time. A native window focus query
does not independently prove application foreground activation. No automatic
focus, background simulation, replay event, pause rule or deadline is changed.

Snapshots coalesce transitions within 250 ms and otherwise emit a one-second
heartbeat. Cumulative focus-event and coalesced-sample counts make that loss of
fine event ordering explicit. The file contains at most 512 records and 512 KiB,
including its header and a terminal truncation record when capacity is reached.
Completed unbuffered writes survive process exit/timeout; this is not a power-loss
durability guarantee. Normal exit adds a final host-step record after stopping
the game. A logging error never replaces an existing game/window error.

The enabled observer performs native queries, serialization and filesystem I/O;
its execution is diagnostic, not a timing or display gate. By default it adds no
native query, world read, serialization or file I/O.

Source `ed16a31f` passed 90 ordinary native-window library tests and 113
presentation/profile library tests, plus both strict all-target Clippy checks.
These configurations overlap; they are not 203 distinct controls. Each executed
all eight diagnostic controls, including exclusive destination admission, finite
event-storm output, actual completed-file observations, a child exiting without
Rust Drop, CLI default-off behavior and original-error precedence. Ten Python
sample/runner controls, scoped formatting, generated conventions and diff checks
also passed. CPU test executables and raw Cargo output are retained by hash.

The [compact packet](evidence/native-window-eligibility/manifest.json) separates
the finalized CPU qualification from the original main300-based `1dff662b`
shipping package (`b21a15ff`). Its three CPU routes and both actual window
timeouts use that exact historical player; they are not executions of the new
diagnostic source. The first export attempt's merged-stderr supervisor error is
retained alongside the corrected public export; the compiler succeeded in both.
The two failed window attempts retain their empty streams and observed group
cleanup, including the original exceptional-return ledger limitation. Root's four
actual tool images have a scoped receipt, not a checked-in PNG archive.

Against main304 `709e1a41`, the implementation delta is six code/tool paths:
package CLI admission, window integration, the logger and its tests, and the
planar runner and its tests. Remaining compiled/toolchain/sample inputs match;
the root vendor-test formatting correction is already identical. Intake must
preserve main304's newer STATUS and presentation documentation. The packet's
docs-only successor does not change the producing `ed16a31f` checkout.

Replay without devices or process execution:

```sh
python3 docs/spikes/evidence/native-window-eligibility/verify.py
# Optional retained originals/binary hashes and integrated source closure:
python3 docs/spikes/evidence/native-window-eligibility/verify.py \
  --originals --binaries --git-source HEAD --source-root .
```

This intake establishes diagnostic plumbing and the historical CPU behavior.
The separately granted fresh diagnostic-source export is outside these finalized
results. No actual eligibility-log window has run at this intake, no complete
native route passed, and no FPS, audio-quality or phase gate is claimed. Source,
raw outcomes and exact identities remain under the retained
`native-window-planar-001` evidence directory.
