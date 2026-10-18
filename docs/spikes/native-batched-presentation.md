# Fixed simulation with batched native presentation

This default-off diagnostic follows the [upstream comparison](upstream-rendering-comparison.md).
The earlier display-link wake experiment preserved simulation but frequently
produced zero- and two-tick batches, presenting each simulation step separately.
The new route accepts every due fixed step and presents only the final accepted
state in that batch. An active display-link pulse with no due simulation step
redraws the committed state without advancing gameplay, input, audio or haptics.

The trusted player selects `--present-batch latest` only with the
`native-batched-presentation` feature, a native window and a bounded stop count.
Unsupported combinations refuse. Ordinary flags and exports retain their existing
behavior. This is not pose interpolation: repeated poses and two-step pose changes
can still be visible even with more regular presentation.

## Correctness contract

The host's explicit admission-only operation validates intermediate state and
retains required resource/rebuild changes without acquiring or submitting a
drawable. The last due step uses ordinary submission. Every accepted step still
executes gameplay, input, UI, audio and haptics. Final partial batches flush at the
requested stop boundary. Failures preserve the accepted prefix and never report
an unaccepted state as displayed. Resize refresh can consume a display opportunity
without losing pending simulation debt.

Temporal/history-dependent reconstruction, dynamic resolution and MetalFX refuse
this route until separately qualified. Ordinary capture remains schema 2. Selected
capture uses schema 3 with the planned/accepted batch and displayed host step;
committed redraws have a distinct role. The analysis counts every accepted step
and all adjacent actual presentations, including gameplay/committed role changes.
Missing timestamps and real lifecycle boundaries break continuity. Fewer
presentations are not interpreted as greater speed, and unrelated clock epochs
are never subtracted for latency or queue-depth claims.

## Local qualification at `6f69ab29`

- 196 player CPU controls pass, with one unrelated ignored control. Strict
  feature-enabled and ordinary-player lints pass. Independent implementation and
  controller reviews found no blocking issue; a process-absence errno precision
  note was corrected and tested.
- Nine distinct scoped actual Metal controls establish intermediate admission, structural
  rebuilds, rejection boundaries, final pixels and UI receipts. All eight steps
  in a mixed zero/one/multiple-step schedule execute with four gameplay draws;
  world, input and PCM match the ordinary route. Three controls pass at `d7e9fc32` in an otherwise failed suite; six pass at
  `af2ee17e`, including the corrected failed assertion. Their host implementation
  is unchanged in `6f69ab29`; later changes integrate the native event loop.
- A fresh shipping export passes dependency and symbol checks. Its private build
  and authoring input storage is removed before startup. Player SHA-256 is
  `fc330c64325c1b4f9a8c12864a77459cbd2fbecf2e28f5185570a3970f2bcc60`.
- The complete 2,156-step CPU replay matches the entire reference gameplay report
  and all 13,798,400 PCM bytes. PCM SHA-256 is
  `ccabf2c68b6388ac554ac3efb5609ddf6b4a45d1a3f316891d5f344094e99ee3`.
  Offline equality does not establish live-device audio continuity.

The native comparison uses Mac17,9 / Apple M5 Pro / macOS 27.0.1 (26A434), a
1280×960 surface and 640×480 logical viewport at scale 2. Its first ordinary arm
completes all 2,156 steps but loses focus twice, at steps 434 and 2,099. It resumes
through normal window events and finishes after 97.938 seconds. The harness
correctly refuses uninterrupted qualification. Nine gameplay presentation
timestamps are also zero and remain visible in the retained record. Neither
selected arm is launched on this failed baseline. Actual terminal exit and fresh
wrapper/child process-group absence are confirmed.

The first failed PCM assertion, earlier lint failure and resource-preflight
refusal remain retained alongside their corrections. They do not become passing
attempts. No scheduling policy is adopted, and no performance, continuous-display
or appearance gate passes.

## Quiet same-player comparison

The director-approved follow-up completes all three 2,156-step arms without
focus loss or zero gameplay timestamps. Every arm uses the exact shipping player,
package, surface and hardware above, with other native builds and GPU work paused.
All wrapper/player groups are absent after their actual terminal results.

| Mode | Actual presentation interval p95 | Intervals over 20 ms |
| --- | ---: | ---: |
| Ordinary | 16.666750 ms | 9 |
| Display-link wake only | 16.666750 ms | 24 |
| Display-link plus latest batch | 25.000000 ms | 193 |

The independent saved-data analysis includes every adjacent valid presentation,
including committed-redraw/gameplay transitions: 2,155 intervals per arm. The
initial drawable has a zero reported timestamp in each arm and breaks continuity.
The latest-batch arm's strict original receipt remains **failed**: its capture
omits wake metadata on 239 committed redraws, and its analyzer also requires the
initial drawable to have a timestamp that both ordinary controls lack. This is an
instrumentation failure, not evidence that those 239 pulses were actually reused.

The negative cadence result does not depend on accepting that failed receipt.
Of its 193 long intervals, 186 end at a two-tick gameplay batch; 153 follow a
committed redraw. The source loop can wake just before a simulation deadline,
redraw the old state, then execute two ticks on the next display callback.
Acquisition in those 153 transitions has a 0.0224 ms median, while gameplay body
work has a 6.062 ms median. These observations motivate testing independently
scheduled rational simulation deadlines, with display callbacks only controlling
draws. They do not prove that proposed implementation improves pacing. Unrelated
clock epochs are never subtracted, and no role transitions are filtered away.

[Quiet raw records and independent analysis](evidence/native-batched-quiet-20261018/manifest.json)
retain the failed receipt unchanged alongside both passing controls.

[Portable receipts, reviews and compressed raw window records](evidence/native-batched-presentation-20261017/manifest.json)
retain original lengths and SHA-256 identities; every archive round-trip matches.
The full shipping package and offline PCM remain local under
`~/.codex/incant-evidence/native-batched-presentation-001/`. Their hashes and
execution receipts are archived, not the binaries themselves. The earlier
controller review's errno precision note is historical; the later explicit
`EPERM`-only correction and refusal controls precede the native attempt.
