# Independent simulation deadlines: private scheduling diagnostic

This default-off source experiment tests one scheduling hypothesis. It is not an
adopted presentation policy, interpolation implementation, performance acceptance or
phase gate. The ordinary route and `--present-batch latest` remain separate and
unchanged in selection and scheduling. Source starts at main `557c196c`; it does
not include the concurrent pose-interpolation experiment.

The retained three-arm comparison's latest arm completed its 2,156-step route
but failed its original host-timing analyzer. Saved-data inspection found missing
wake metadata on its 239 committed-only draws and a startup drawable reporting
zero. Neither finding turns that receipt into a pass. Across all positive adjacent
callbacks, including committed-only draws and role transitions, 186 of its 193
gaps over 20 ms followed two-tick batches. The source clock was only visited on
display callbacks: an early callback can admit zero steps, then the next callback
admits two. This is a concrete hypothesis, not proof that a different wake policy
will improve actual display intervals. The immutable analysis is external at
`native-batched-presentation-001/independent-analysis-002/review.json`, SHA-256
`a9668788be94ba307363bb28994382f040272fda6aa00c5e1f617ac128b7fcac`.

## Selection and ownership

A private native player must explicitly enable
`native-independent-presentation` and select both `--present-batch independent`
and `--native-wake-source display-link60`. It requires the existing supported Mac,
60 Hz cooked scene, one native window and a finite `--stop-after 1..36000`.
Unsupported platforms/features and incompatible offline/checkpoint/pacing modes
refuse before window, device or GPU creation. Public export selection is unchanged.
No script, project or gameplay input gains control over clocks or OS state.

The existing rational clock remains the only simulation-time authority. While
active, `WaitUntil` chooses the earlier of its next deadline (subject to an
existing surface retry) and the display-link health deadline. A display callback
can still wake the loop, but its future target time is only recorded metadata.
The clock admits only elapsed ticks; no epsilon, future advance or variable dt
is added. Existing max-four catch-up and explicit discarded-debt accounting stay
in force. Focus, suspension, minimization and supported-size fences stay active.
Resume first refreshes through a real reservation, then resets the rational clock
and waits for its next deadline; resume does not force an immediate extra tick.

Each tick uses the ordinary game session, input/replay packet, input consumption,
UI receipts, structural/resource validation, audio and haptic paths. Only GPU
presentation is deferred through the existing mandatory `AdmitOnly` consumer.
Its target slot is empty: an unexpected target acquire fails. No drawable is
retained across event-loop callbacks. Intermediate resource replacements stay
owned by the accepted consumer. Existing refusal of temporal/motion/MetalFX and
dynamic-resolution policies remains unchanged.

On an actual consumed display callback, the host reserves once and refreshes the
latest committed world without advancing simulation, input, audio or haptics.
Stop and restart additionally require a final old-generation draw before owner
retirement. These are explicitly labeled boundary flushes, without an invented
display callback. A deferred boundary flush fails the diagnostic and retains the
accepted simulation receipt; it cannot silently destroy an undisplayed accepted
state and report success. Initial, background and UI-resize refreshes retain their
existing separate roles. UI resize and a delayed resize notification are fenced
by the currently observed native extent and an actual reservation/relayout before
subsequent UI ticks.

An important selected-mode difference is explicit: unknown surface occlusion or
loss is discovered at the next draw acquisition, not before every independent
simulation tick. Known focus/minimize/suspend/size failures still block ticks.
After a failed display acquisition, existing pause/input/surface retry behavior
applies. This experiment must not be presented as equivalent surface admission to
the ordinary path.

## Busy-loop and observation limits

At most four ticks are admitted per due calculation. A busy loop rechecks the
actual current time on the next callback; it does not simulate callback target
time. A past rational deadline may cause an immediate subsequent event-loop wake,
as in ordinary bounded catch-up; there is no `Poll` loop or per-tick sleep/wait.
The extra intentional wake stream is the 60 Hz simulation timer alongside the
already coalesced display callbacks. OS input callbacks and existing health work
remain; this is a bounded-per-turn policy, not a guarantee of bounded total CPU
load under sustained overload. The finite host-step bound and external child
supervisor are still required.

Sidecar v4 is deliberately incompatible with old ordinary/batch analyzers:

- `simulation` / `admitted` rows have schedule identity and completed/finished
  CPU spans, but no reservation, acquisition or drawable. Haptic failure retains
  an unfinished admitted row; a failed tick retains an open row.
- `committed` rows bind the exact current accepted session and host step. Pulse
  draws record the consumed native callback sequence, Rust-Instant arrival and
  native target metadata, plus the current event-loop wake. Stop/restart record
  their reason without a fabricated callback. A shared event-loop wake across
  admissions and a draw is expected, not a reused display callback.
- Simulation rows do not split display segments. Real focus, resize, retry,
  recreation, restart and display-role boundaries remain. All zero, missing,
  invalid, abandoned, equal and positive drawable outcomes must be retained.
- Attempt capacity is shared by simulation and display; 2,156 ticks therefore
  need more than 2,156 attempt slots. The bounded Core probes below used 8,192.
  The existing 36,000 maximum, compile-time 512-byte attempt bound, two bounded
  segments per attempt, one-second final callback drain and 64 MiB sidecar cap
  remain. Snapshot and JSON allocation are outside the preallocated-row bound.
  Exhaustion is explicitly partial, never complete coverage.

No Rust-Instant timestamp is subtracted from Metal time. The experiment does not
convert a native target time, callback count or display interval into GPU time.

## Executed qualification and remaining lifecycle scope

The producing implementation remains `e73793638c7f5d35cc872c648dcb57149480a892`.
The shipping successor `b0d37fb8af69f5ff74658011a525f9a37af77d3e` changes only the
reviewed, short-circuit-equivalent nested condition requested by strict Clippy.
Both are based on `557c196c69a8a71a1202a4e19d9b60cde54988a2`; the concurrent
pose-interpolation experiment is not part of this source. Neither this diagnostic
nor its measured display results are an adopted scheduling policy or phase gate.

CPU002 passed 202 selected-feature and 151 ordinary player controls, with one
ignored control in each selection. Its strict Clippy failure is retained. CPU003
passed both remaining strict lint commands after the mechanical correction;
unchanged behavior suites were not relabeled as having run against the successor.
Controls cover capability refusal and the ordinary route, rational clock phase,
bounded catch-up/debt, focus/resume reset, actual callback ownership, separate
simulation/admission and presentation roles, boundary flush metadata, and bounded
partial storage. All native commands were supervised and reaped. The aggregate
CPU receipt is `cpu-final-001.json`, SHA-256
`018a0c2cc5ae3e97ecc770a7e42f4dab8852ac9fe672e42269a80facad323dfa`.

### Fresh shipping and full CPU replay

One fresh private shipping build produced the copied player SHA-256
`c99d4b5cc35955597d680fb0c0c72cb0782d2f8846612a0fe39280f6d7dca281`.
Its actual Rust 1.99 arm64 fast4, fat-LTO, abort, one-codegen-unit selection and
recipient feature graph were audited. Source closure contains 4,815 tracked
inputs; the prepared runtime input tree contains 46 files. That runtime tree is
not described as the separate original 74-file authoring tree. A source-free
one-tick startup passed. All 56 managed producer/startup children returned zero,
were reaped, and the owned scratch target was removed. The shipping report is
`shipping-001/package/diagnostic-build.json`, SHA-256
`f8cd885b152ebf9b4af7d37d6e91bbc19b1f1e88fc65a2b80701c0d1f05ad3f3`;
the producer result is `shipping-001/metadata/result.json`, SHA-256
`9a5cb1483bdc7d37461b4bafa172d97783c764dc54162aad56efc3b57f2f779d`.

The same copied player/package then completed the unchanged 2,156-step recorded
Core route once in the CPU host. All eight assertions and 17 final entity
observations matched the historical report exactly. Every byte of the 13,798,400
PCM output matched SHA-256
`ccabf2c68b6388ac554ac3efb5609ddf6b4a45d1a3f316891d5f344094e99ee3`.
The result is `functional-001/result.json`, SHA-256
`24dfe2f59ba0bff847041339e889ae94171737dd8d1a149c1abf57ae303816bf`.
This establishes the package's CPU gameplay/audio oracle. CPU replay does not
exercise the independent native scheduling policy, on-screen delivery or audio
hardware quality.

### Same-player ordinary/independent native pair

Root ran one ordinary and one independent native window serially in the agreed
quiet lease. Both used the exact same copied player, cooked data, recording and
observations. Both completed all 2,156 steps with exact CPU logical observations
and zero recorded focus losses. Every attempt, role, boundary and drawable outcome
is retained. The supervised children and wrappers were reaped and their PIDs and
process groups were independently absent afterward. The root closeout is
`paired-root-closeout-001.json`, SHA-256
`777994acd4a0371b89964ad3507271151d680dbd4f1999cf9f75462141c3aefb`.
Independent saved-data replay is `paired-independent-review-001/result.json`,
SHA-256 `09fd52114d0c0bb76cc9abe6b5803babb307e4342417a731c757f2eb595fe4cd`.

| Retained observation | Ordinary | Independent |
| --- | ---: | ---: |
| Drawable rows, all roles | 2,157 | 2,158 |
| Positive presentation callbacks | 2,155 | 2,157 |
| Reported-zero callbacks | 2 | 1 |
| Valid positive adjacent intervals | 2,154 | 2,156 |
| Positive adjacent subset p95 | 16.666750 ms | 16.666750 ms |
| Positive adjacent subset maximum | 25.000042 ms | 25.000042 ms |
| Retained valid intervals over 20 ms | 10 | 12 |

The subset is defined by adjacent recorded drawable rows with compatible
surface/session ownership, no intervening real boundary or incomplete attempt,
and two positive increasing presentation timestamps. It includes every display
role; it does not bridge zeros or failures, remove tail samples, or imply
continuous full-run delivery. Metal presented-time zeros retain the API's
not-yet-presented/dropped ambiguity. The reported intervals are not GPU execution
time. No performance, visual, complete-game display or shipping 2 ms gate passed.

The independent run recorded 2,156 actual `simulation` admissions, 2,156 owned
committed draws and two startup/refresh draws. Every valid adjacent positive
independent display pair contained exactly one admitted simulation step. All
2,156 independent scheduler batches had due=1, discarded debt=0 and no forced
resume. This removes the old observed zero/two-tick batch pattern in this run;
it does not establish an interval improvement. The ordinary run also admitted
one tick per due batch. Its decoder's absence of a separate `simulation` role
must not be described as absence of simulation.

Same-Rust-Instant wake-lateness p95 was 0.200375 ms for ordinary and 0.144041 ms
for independent; one ordinary initial forced-resume sample has no deadline and
remains missing. These are diagnostic host spans. No Metal timestamp or native
future target was subtracted from them. Six of the 12 longer independent display
intervals coincide with authored events having 8.19–11.72 ms simulation spans,
sub-millisecond following draw bodies and small acquisition spans. The other six
have 2.48–4.37 ms simulation spans: the event spikes are not the sole explanation.
This one fixed pair does not justify adoption or a causal performance claim.
Reported device-audio xrun, route-change and poll-limit counters were zero in
both runs; live device PCM and audible quality were not measured.

### Appended restart oracle and pending native lifecycle controls

A separate append-only input derivative retains every original recorded event
and all eight original assertions at host step 2,156, adds the recorded restart
key down/up at 2,157/2,158, then continues to host step 2,216 with six additional
assertions. The original recording and original route remain immutable.
One CPU execution completed with generation 2, global host step 2,216, current
world tick 59, and the old-generation restart at host/world step 2,157. All 14
assertions and 17 final entity observations passed. The original 13,798,400 PCM
bytes matched the unchanged prefix. The new full 14,182,400-byte PCM oracle is
SHA-256 `916116579b7eb8a175e9c1f2739bcc319b675032a71c21cc4b27f96910c7f5d3`;
there is no claim of a historical full-output match for the appended segment.
The result is `lifecycle-cpu-001/result.json`, SHA-256
`9032fc52ecf14dab84b5d06c02d801634b40cd5c56eb3d701ba61441d4eff690`;
its closeout is SHA-256
`3cbe87e9c55052c02263837d53d82dfa5e0916ed24cc76786efb59e8189d685e`.

The separate native focus/resize attempt failed its unchanged 90-second child
bound (90.207 seconds overall). It retained two focus-loss events; the last state
was unfocused and simulation-suspended at host/world step 2,151 in generation 1.
No requested size change occurred: every saved physical extent remained
1280 × 960. The final gameplay report and both presentation/compilation sidecars
were empty, so the original analyzer correctly refused incomplete data. The
partial state log does not establish successful resume or resize qualification.
The child recorded cleanup confirmation; root's fresh PID and process-group
absence check for wrapper 14159 and player 14169 is retained as
`window-lifecycle-focus-resize-001/root-absence-001.json`, SHA-256
`3265215e65e79c76a9423b977e0088b5d58f9c89f8b6113897e2a801605f9ab7`.

Native restart was never launched. Its prepared control would require an owned
old-generation Restart draw before replacement and an owned final Stop draw,
checking current-session accepted counts separately from cumulative presentation
counts. The CPU restart oracle does not prove either native flush. Direct
per-draw extent identity is unavailable in the capture; state-log OS dimensions
and completed refresh identities cannot fill that gap. State-log and host timing
clocks also have distinct origins and must not be subtracted.

The director subsequently prohibited computer use, screen capture and native
window launches. No further native lifecycle attempt is authorized by this
packet. Focus/resume, resize and native restart/stop lifecycle gates remain open.
The default-off diagnostic can be reviewed with these limits; the evidence does
not support selecting it by default or claiming performance or visual acceptance.

### Portable evidence and integration boundary

[The manifest](evidence/native-independent-simulation-deadline/manifest.json)
binds compact source, commands, raw logs/reports, all-role arrays, plans and
closeouts. The initial Clippy failure, analyzer/preparation errors and failed
focus attempt remain failed. Executables, cooked payloads and PCM stay external;
the archive contains their identities, not substitute bytes.

Run the data-only verifier from the repository root:

```sh
python3 docs/spikes/evidence/native-independent-simulation-deadline/verify.py
```

It hashes every archived record, checks source snapshots and terminal results,
recomputes all-role positive-adjacent intervals and simulation ownership from raw
rows, compares complete logical reports and checks the append-only restart route.
It never executes archived scripts or launches the engine. `--self-test` checks
refusal of altered ownership, future admission, incomplete stages, dropped rows,
boundaries and malformed JSON. `--originals` rehashes
the exact recorded external text files; `--binaries` additionally verifies copied
executables, cooked data, producer payloads and both full PCM outputs, including
the exact old-route prefix. Without that option, PCM/binary claims are explicitly
recorded identities rather than a fresh check of absent bytes. `--git-source PATH`
rechecks both 28-file frozen source generations against their actual Git objects;
the separate 4,815-input shipping inventory is archived with its identity and is
not rehashed by that option. These optional checks require the retained local
originals or Git history and refuse missing/changed inputs.

This documentation successor changes no producing Rust, shader or package byte.
Root owns integration and STATUS. Any later source composition must preserve the
feature's default-off selection and identify its source separately; no original
receipt or source freeze is rewritten to match a later main branch.
