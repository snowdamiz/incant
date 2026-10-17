# Default-off wake and acquisition boundary attribution

Thirty actual native CPU controls and both collector-enabled and ordinary
all-target strict lint pass on the frozen producer `51eab228`. A fresh
shipping/window observation remains pending. This adds boundaries to the existing
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
unchanged v1 behavior. All 31 Python controls and six protocol controls pass.
The native suite includes six new boundary controls, 13 existing player controls
and 11 imported HAL collector controls. It exercises actual version 2
serialization, fixed capacity, refusal/unwind and once-per-pulse identity.

The first managed attempt passes all 30 tests, then refuses its incorrect
expected count of 19: preparation omitted the imported HAL tests. That failed
receipt is preserved. A source-identical successor verifies every test name,
copies the successful 37,007,664-byte executable (SHA-256
`d98aa87bf9ca9ea4560888b687e46fec3cefe59a57f9bd1ce7e86d2b2ae10146`)
with its exact nine-feature audit, and runs only the two pending strict lints.
No test is repeated. Dependency Tao warnings remain in the raw logs.

Both wrappers and all nine owned process groups are independently absent.
Cumulative qualification takes 134.746 seconds. The existing FONT allocation
grows from 64,799,776,768 to 65,147,740,160 bytes, below the 65,336,647,680-byte
scoped ceiling; prior shared files/accounting remain. Main377 joins after the
qualification through documentation only; it does not relabel the producer.
No GPU, native window, player CLI or timing benchmark runs in this qualification.

The [portable CPU packet](evidence/native-wake-acquisition-boundaries/manifest.json)
retains the original count refusal, source inventory, raw outcomes, cleanup,
commands and executable hash without duplicating binaries. Replay it with:

```sh
python3 -B docs/spikes/evidence/native-wake-acquisition-boundaries/verify.py
python3 -B docs/spikes/evidence/native-wake-acquisition-boundaries/test_verify.py
python3 -B docs/spikes/evidence/native-wake-acquisition-boundaries/verify.py --originals --binaries --git-source 51eab2289a8a14ce1da82ac90ddeabcee9d1470f --source-root .
```

`--source-root` checks executable/tool inputs, excluding changed outcome docs.
Fresh native output still needs the strict version 2 contract and actual
shipping executable/package/recording bindings before interpreting any timings.
This qualification identifies no timer, drawable or pacing cause.
