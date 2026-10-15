# Native window host timing

The existing `profile.frame_wall` boundary measures the simulation attempt and
its rendering consumer. It starts after surface reservation and input/UI
preparation. The optional native-window `host_cadence` report adds observations
of that omitted host work without changing simulation, presentation or scheduling.
It is emitted only when the player is built with `profile` and starts with an
active `--profile` or `--profile-tracy` capture. Ordinary builds omit the module
and calls; an inactive capture allocates no cadence storage or timestamps.

`host_wall` starts immediately before surface acquisition. It ends after a
successful tick, input/UI publication, session-request handling and normal
haptic dispatch. A restart-triggering tick ends after haptic suspension and
before replacement of the session; replacement/startup work is excluded.
`accepted_interval` measures consecutive CPU host-completion timestamps,
including active event-loop waits and surface backpressure. Neither observation
is a GPU-completion, scanout or displayed-frame timestamp. These metrics cannot
by themselves establish 60 fps or a shipping performance gate.

Focus loss, suspension, every resize/scale event, inactive presentation, deferred
surface acquisition, initial presentation and session replacement break the
interval segment. Redraw-only presentations are not successful simulation
samples. A failed step does not emit a successful sample; the enclosing host
already returns an error. A resumed segment starts with one host-wall sample and
no cross-pause interval. The existing dropped-tick and surface counters remain.

Each of two histograms reserves 10,001 counters once per active capture: exact
zero, then 10-microsecond intervals up to 100 milliseconds, plus a separate
overflow count. Recording allocates nothing. Nearest-rank p95 is an inclusive
integer-nanosecond interval; overflow at that rank, no samples or count
saturation yields null rather than a clamped time. Raw counts and invalid-clock
and interruption counters remain in the report. The storage bound concerns
histogram counters only, not the rest of the profiler or process.

CPU controls cover waits, interrupted segments, empty capture, exact histogram
boundaries, overflow, saturation and invalid clocks. Local source qualification
at `e266e950` passes all 80 profiled native-window library tests, strict all-target
Clippy and a separate unprofiled native-window library compile. These use pinned
Rust 1.99 arm64, locked/offline CI profile and jobs/Rayon 2. The earlier passing
source001 CPU checks did not catch a missing resize interruption for windows
without UI. Independent source review caught it; source002 interrupts every
resize directly and clarifies the restart endpoint before qualification.

The [manifest](evidence/native-window-cadence/manifest.json) retains exact inputs,
commands, raw logs and copied executable identity; the
[review](evidence/native-window-cadence/review.json) binds the final source.
Replay with `python3 docs/spikes/evidence/native-window-cadence/verify.py`.
Checkpoint CI retains unprofiled window tests and adds profiled library controls;
its scoped all-target lint includes the optional profile module.
No actual-window timing, shipping performance or phase result is claimed here.
