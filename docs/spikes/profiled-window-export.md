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
source review confirms the feature and startup boundaries. This does not yet
establish an actual shipping window export, rendered-game performance, GPU
completion, displayed frame rate or a phase gate.

Replay the focused controls with:

```sh
python3 -m unittest discover -s tools/tests -p 'test_native*profil*.py' -v
python3 -m unittest discover -s tools/tests -p test_native_ui_export.py -v
```
