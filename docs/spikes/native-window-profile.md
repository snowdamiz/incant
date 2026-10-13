# CPU profiling in the native player window

The desktop `--window` route accepts explicit `--profile` and, for the standalone
CLI built with `profile-tracy`, `--profile-tracy`. The same capture owner now covers
the window lifetime and appends its final summary through the player's existing
report callback. Embedded callers still cannot start the process-owned Tracy
transport; build-info requests refuse profiling, and ordinary launches remain off.

`player.frame` measures an actual simulation attempt and records success or
failure. Redraw-only refreshes use the separate `player.presentation` CPU zone;
event-loop waiting, minimization and unfocused idle periods do not manufacture
simulation frames. Restart has its own fixed zone. These inclusive CPU zones are
not GPU timings. Existing per-system/script, heap and allocation observations
remain attached to the same capture. The callback receives the native report only
after the native session and its audio have retired successfully.

Ten local checks pass against frozen inputs: default player tests (20), window
plus Tracy feature tests (69), the actual offline profile CLI test (one), summary
tests (four), strict Clippy, wasm32 summary compilation, runtime dependency guard,
formatting, conventions and diff checks. Counts overlap across feature builds.
The new tests distinguish successful/failed steps from presentation refreshes,
preserve returned values/errors, and exercise the public native route's argument
failure before it creates a window, proving capture retirement on that path.

The [manifest and retained raw evidence](evidence/native-window-profile.json)
record the exact commands and inputs. No native window was launched in these
checks. A routed native execution and actual Tracy client capture remain pending;
this does not satisfy phone, shipping performance or profiler-completion gates.
