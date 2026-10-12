# Retained native CPU Play worker

`incant_headless native-worker PROJECT --project-sha256 HASH` starts one disposable
native session. The trusted parent chooses a matching physics binary and the
exact project document identity. Startup performs the existing cook once, then
drops the authoring document and command bus before sending `ready`. No command,
shell, executable, script upload or project mutation operation exists on the pipe.

Protocol version 1 uses newline-delimited JSON. A request has `version`, an exact
monotonic `id` starting at 1, and `request`. The only operations are `step` with the
existing typed input/selected-observation parameters, `observe` with explicitly
selected entity/input/log fields, and `stop` without parameters. All envelope
fields are strict. A well-formed request consumes its ID even when native
admission refuses it; duplicate steps cannot advance the world again. Invalid
protocol/JSON requests do not consume IDs. Stepping never serializes a project.

A frame is bounded to 256 KiB. Truncated/oversized input, broken output and EOF
close the child and discard its complete native composition. Responses have an
8 MiB encoded-output limit checked after serialization; this is not a general
allocator bound. Simulation advances only on accepted `step`, not wall time.

The [retained manifest](evidence/native-play-worker.json) records:

- All 21 headless tests, including five worker lifecycle/admission tests, plus
  Clippy and builds in deterministic and fast modes. Actual target Cargo artifacts
  bind both binaries to all five solver feature sets.
- Twelve real process cases across both modes. Public command-bus authoring creates
  an attached TypeScript mover. Both retained workers match ordinary native Play
  through 12 ticks after deleting their disposable project/source/cache directory.
- Exact startup-identity and wrong-kernel refusals; EOF, truncated and oversized
  frames; unknown operations/fields, wrong version and malformed JSON; observation
  validation before ticking; duplicate-ID fencing and acknowledged Stop/reaping.
- The original failing test that caught Serde accepting an extra Stop field, and
  its corrected strict empty-struct variant. No failed attempt is counted as pass.

Independent source/execution review is accepted in
`evidence/native-play-worker-independent-review.json`. This proves a retained CPU
process boundary, not automatic editor startup, GPU viewport embedding or a
shipping player. The movement fixture has no active physics bodies. The worker
currently exposes state/logs/input, not PCM. Device performance, solver parity,
physics saves and phase gates remain separate.
