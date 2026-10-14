# Native presentation observation after typed audio Xrun recovery

The fresh instrumented Core player completed the full 2,156-step recorded-input
replay on Apple M5 Pro/Metal. All eight gameplay checks and 17 observed entities
matched the retained offline baseline. The owned process exited zero and was
reaped. This closes complete replay for this observer/audio composition; it does
not establish complete display cadence, a performance gate, or audible quality.

The sidecar retained 2,157 acquired rows: one initial refresh and 2,156 committed
gameplay attempts. Metal reported 2,149 positive timestamps and eight zero
values, with no missing callbacks after the final drain. One adjacent positive
timestamp pair is equal. These outcomes remain explicit; no percentile is
computed over a filtered subset. The earlier timeout and the 1,444-step fatal
Xrun attempt in [native-player-presentations.md](native-player-presentations.md)
remain failed, separately bound observations.

## Producing source and qualification

The producing source is `b94c6c7c62ae425074ef1928cb19559e0b958582`, a normal merge
of the observer producer and reviewed audio fix. Its only conflict resolution
adds the current-session audio diagnostics to the existing report closure. The
observer's finalization, original-error precedence and bounded post-gameplay
drain are preserved. The source tree was exactly equal to root's main303
integration `9719bea79bdc5a88e891df275fe6f815b83a1e1d`. Main304 subsequently changed
only documentation and one edition-neutral test assertion; this artifact was
not rebuilt or relabeled as that successor.

Six scoped CPU commands passed: 105 observer-player tests, 87 normal-window
tests, 39 audio tests, strict player lint in both modes and direct HAL lint.
Rust1.99 arm64 CI, jobs2/Rayon2, locked/offline and serial test execution were
used. Copied binaries and their inventories are retained. The chain completed
in 246.154s with 242,121,678 B sampled target growth and 101,000,368,128 B minimum
sampled free space. There was no window or GPU work in that CPU chain. A
preparation-only source-inventory assertion failure is retained; it launched no
native process. The later main304 formatting-check failure is retained too.

One fresh private shipping build then completed all 22 managed children in
587.411s. Its source manifest binds 4,514 tracked inputs and the same 74 authored
Core inputs. The actual target compiler artifacts retain arm64, Rust1.99,
fast physics, optimization3/fatLTO/abort/one codegen unit. Relative to the normal
window/profile graph, the only added feature recipients are player
`native-presentations`, wgpu-hal `diagnostic-presentations`, and objc2-metal
`objc2-core-foundation`. Runtime compiler, native checkpoints and script/navigation
attribution are not enabled. The public exporter admission was unchanged.

Copied executable SHA256:
`5afa93ee497b34b03a6d7305e44a85ea7f8771209fd820b5d9d99b4419fdf105`
(17,861,168 bytes). Its genuine script build ID is
`28f48996d15d7dcce74b6add8ee65bb80a2edef4f360be5d0115c3341e668713`.
Bytecode SHA256 remains
`29e28c57a5d290f6a44811635aee609cd1699f478c741f7de7dfe4e64618e917`.
All 28 cooked files (35,365,370 bytes), compiler output, bytecode and hardening
payload match the retained baseline. After the owned scratch directory was
removed, one offline startup tick matched the full baseline report and PCM.
That is offline PCM parity, not a recording of live audio output. All 82 retained
build artifacts were rehashed; minimum sampled free space was 95,745,556,480 B.

## Actual window outcome

The exact copied executable was placed in a distinct unsigned private
`Core Presentation Audio.app`. One managed child was allowed 120s within a 150s
stage, with a 4,096-row capture capacity, 6GiB free floor, 4MiB stream caps and 32MiB
sidecar cap. It used the unchanged 2,156-step physical-input recording and
observation assertions. No listener, retry, extra player or gameplay input was
introduced. Root used normal Finder Open on the already-owned PID 8524, then
Raise/title-bar activation. Actual CUA images showed Samples 1/4 followed by
Samples 3/4 and changed scene/player position. Those tool images are retained in
the conversation; the packet does not claim filesystem screenshot files or
continuous video. AX state alone is not a native foreground-state measurement.

The controller (session 4460) completed in 70.378s, child exit 0, empty stderr,
2156 accepted host steps and 2157 accepted frames. The profile records 2156
completed/zero failed frames. This elapsed time includes startup, focus/CUA
interventions, observer work and finalization; it is not a benchmark.

| Observation | Actual retained result |
| --- | --- |
| Gameplay | Eight checks/17 entities exactly equal the offline baseline |
| Audio owner | Zero Xrun notifications, zero poll-limit hits, saturation false |
| Callback rows | 2149 positive, 8 zero, 0 missing |
| Invalid/overflow/protocol/discard/abandon | All zero |
| Host metadata | No omitted attempts/exhaustion; 6 segments and 9 focus notifications |
| Final drain | 0.043577958s, 0 pending at last scan |
| Timestamp continuity | 2142 positive adjacent intervals, 13 continuity breaks, 1 equal pair |

The zero rows are initial refresh and gameplay steps 85, 87, 88, 137, 138, 296, 297.
They are not converted to inferred timestamps. The equal pair is between
acquired rows 1 and 2. Initial/final unobserved intervals, segment boundaries and
zero outcomes keep all-frame display timing incomplete. No samples are sorted
to repair order and no display-p95/FPS verdict is made.

The audio fix now treats only CPAL's typed Xrun notification as recoverable and
counts it; other error kinds and queue loss stay fatal. This run observed no
Xrun, so it proves successful composition and completion, not an actual live
recovery event or glitch-free output. The report explicitly says
`scope: current-session-owner` and `glitch_free_output_verified: false`.
Per-present block registration/copying, callback locking and final snapshots
introduce observer overhead. Actual final-game shader-hitch qualification remains
separate; live object counts do not establish cumulative compilation history.

## Portable evidence

`evidence/native-presentation-audio/` contains 299 records: source copies,
CPU/build plans and logs, actual compiler messages, live controller and data-only
controls, full raw report/sidecar, root build/result/CUA reviews and prior failed
outcome records. The deterministic archive is 4,636,548 bytes and expands to
66,352,793 bytes, below its 128MiB replay bound. Thirty-nine external artifacts
include copied native binaries, cooked payload and dSYM. They remain hash-bound;
the default verifier does not need or execute them. The broader historic packet
retains the original observer development/build failures.

Run data-only replay from the repository root:

```sh
python3 -B docs/spikes/evidence/native-presentation-audio/verify.py
python3 -B docs/spikes/evidence/native-presentation-audio/verify.py --originals --binaries --git-source .
```

Both forms read retained evidence only. The full form also rehashes external
artifacts and reads 149 exact producing Git objects; it never launches the
player, compiles code, executes archived scripts, or touches the user's pointer.
The producing build receipt SHA256 is
`d8049aae78370de74d5b5c6f99341733cef68e64a9491315b997b9d2367667ca`;
final live receipt is
`48649f32a56f55db739a143b8580547ba16a6509b4b3b68a46bad0d9fb6c493d`.
