# Explicit native-window profiling export

The public native exporter now admits `--profiling summary` and
`--profiling tracy` with `--mode window`, matching the existing player runtime.
Without an explicit profiling option the dependency graph remains unchanged.
Unknown modes and routes still fail before project or output access; feature,
compiler-artifact, environment, code-generation and runtime-symbol audits remain
in force.

An explicit `--ui-viewport WIDTHxHEIGHT` also applies to a window package's
offline, one-tick startup check. That check runs the copied player after removal
of its private build directory. It does not open a window or start profiling.
Actual window playback rejects offline viewport arguments and uses the window's
real dimensions.

Ten profiling/process controls and two viewport-export controls pass. Independent
source review confirms the feature and startup boundaries.

A fresh shipping export at `ceda3a94` passed dependency, feature, code-generation
and symbol audits, then ran its copied player after private build storage was
removed. This used Rust 1.99, Node 22.23.3 and Xcode 27 on Mac17,9 / Apple M5 Pro,
macOS 27.0.1 (26A434). The immutable Core project has 10,045 entities, including
10,000 scripted entities; the package contains the authored models, audio and HUD.
The summary-profile player is SHA-256
`eb1034bbd3c5329437344b93d33edf90fb74c2d29e408a9047d65eb48845054c`.

That player subsequently opened its Metal window and exited zero after 600
simulation ticks and 601 accepted presentations, including the initial frame.
It reported 600 completed profile samples, no failed frame or skipped surface,
and a 640×480 HUD with seven text widgets and 102 glyph quads. Accepted
presentations are enqueued work, not proof of display completion. The original
harness failed because it expected all 13 logical UI nodes to be text widgets;
six are containers. The raw failure and independently reviewed data-only
correction remain intact. Post-run package checks after that failed assertion
did not execute. No screenshot was obtained before the window exited.

The run exercises host cadence reporting, but CPU completion timestamps do not
establish displayed frame rate or whole-frame GPU cost. It was a no-input
functional run, not representative active gameplay or a performance/phase pass.
The historical export's symbol commands were outside its managed command ledger;
the subsequent [command-lifetime repair](export-command-lifecycle.md) does not
retroactively supervise that run. Its 91 Python controls pass, with four
Windows-only checks deferred to hosted CI.

The [portable manifest](evidence/profiled-window-export/manifest.json) retains
raw outputs, source identities, original failures and reviews. Replay its
integrity and scoped observations with
`python3 docs/spikes/evidence/profiled-window-export/verify.py`.

Replay the focused controls with:

```sh
python3 -m unittest discover -s tools/tests -p 'test_native*profil*.py' -v
python3 -m unittest discover -s tools/tests -p test_native_ui_export.py -v
```
