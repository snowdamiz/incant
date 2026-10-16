# Default-off wake and acquisition boundary attribution

Source implementation only; native CPU qualification and a fresh shipping/window
observation remain pending. This adds measurement boundaries to the existing
optional `native-presentations` collector. It changes no Tao timer, fixed clock,
FIFO/drawable policy, input, simulation, retry, publication or audio behavior.
No measured saving, pacing repair or phase-gate result is claimed.

The complete historical [host-stage replay](native-presentation-host-stages.md#complete-same-source-live-replay)
measured p95 4.278 ms pulse lateness, 13.492 ms reservation and 4.095 ms committed
body. Its pulse is `MainEventsCleared`, delivered after Tao's earlier
`NewEvents`/AfterWaiting phase. Reservation includes `get_current_texture`, which
can wait for a drawable, plus other host work. Those broad spans cannot identify
a timer defect or attribute the reservation to GPU execution.

## Version 2 contract

A fixed numeric wake snapshot is consumed at each `MainEventsCleared`, even when
no tick is due. It retains sequence, arrival, StartCause and requested deadline,
plus the count of wakes since the previous pulse. Each gameplay attempt binds
only the current pulse; catch-up ordinals share that one snapshot. Missing,
overwritten, no-wait, unknown-cause and mismatched-deadline cases remain explicit.
An earlier pulse's wake cannot be reused. Overflow latches diagnostic loss.
Signed early `WaitCancelled` arrival remains negative when subtracting the
matching requested deadline; it is never saturated or zero-filled.

Two fixed acquisition slots retain the beginning, end and actual outcome of
each existing `get_current_texture` call, inside the observer bind/unbind and
before view creation. The second slot exists for the already bounded retry.
Timeout/occlusion/outdated/lost/validation and incomplete/unwound calls remain
visible; no successful reservation is invented. These are acquisition-call wall
spans, not exclusively Metal `nextDrawable`, GPU work or thread CPU consumption.

The new capture and host contract are version 2. A new strict analyzer accepts
that version and exact field/kind/limit contracts. The original version 1
analyzer remains byte-identical, so its archived replay and hashes are preserved.
No historical capture or source identity is relabeled. Unknown versions refuse.

The per-attempt conservative static bound increases from 192 to 512 bytes.
At the unchanged 36,000-record maximum, attempts/callback rows/segments account
for at most 23,616,000 logical payload bytes, excluding vector headers, callback
blocks, snapshots and JSON copies. The 64 MiB writer cap and capacity admission
remain; larger JSON refuses rather than becoming an unbounded stream. New wake
tracking uses fixed state/counters and adds no per-wake allocation or string
record. No extra clock read occurs when capture is unselected. Timing reads and
metadata writes add observer overhead; this is not a memory/performance bound
on the complete game or driver.

```sh
python3 -B tools/presentation_boundaries.py V2_CAPTURE.json --max-input-mib 64
python3 -B -m unittest tools.tests.test_presentation_boundaries tools.tests.test_presentation_host_stages
```

The analyzer preserves all original host/callback/finalization checks, then adds
signed deadline→NewEvents and NewEvents→pulse partitions. It keeps valid/missing/
invalid/not-applicable counts; a valid subset cannot become complete attribution.
The 5% largest valid lateness batches are selected once, with catch-up counted
once. On the same attempts, acquisition time and the rest of reservation are
partitioned for the 5% largest valid reservation spans. Raw paired sums and their
majority component are descriptive, not a quality metric or causal verdict.
No marginal percentile subtraction or Rust/Metal epoch arithmetic is performed.

Pure controls cover signed early arrival, forced resume, absent/overwritten/
stale wakes, catch-up identity, deadline mismatch, sequence overflow, acquisition
success/refusal/retry/unwind, larger record admission, exact schema refusal and
unchanged v1 behavior. Rust controls are authored but unexecuted until the
separate native-builder grant. Native output will need to be checked against
this strict contract and actual executable/package/recording identities before
interpreting any timings.
