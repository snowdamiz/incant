# Native window terminal error preservation

The [native text follow-up](../../handoffs/0058-native-text-retirement/result.md)
recorded one excluded, heap-instrumented process exiting with only `Error: Failed`.
Its original failure and the cause of termination were unknown. Source review
identified two ways that the native window host could lose that original error:
later shutdown callbacks queried a failed game session and replaced it, or IME
retirement returned its own error before the saved game error was returned.

The host now stops game callback delivery as soon as a callback fails, a native
close arrives, or `--stop-after` is reached. It also skips the trailing text-session
synchronization in a callback that requested exit. Surface cancellation and final
IME retirement still execute. Error precedence is the original typed callback
error, the event loop's failure code, then the cleanup error. Successful callbacks
continue to receive events normally. No game state, rendering or IME ownership
protocol changes are involved.

Behavior tests exercise a failing frame followed by multiple shutdown callbacks,
explicit exit, successful delivery, and failing cleanup. They check invocation
counts and the preserved error type as well as messages. The native-window build
and its tests exercise the actual host integration; they do not open a window.
All 57 player tests pass (including four new regressions), as do native-window
all-target Clippy, formatting and conventions. An independent source review
accepted the two changed source files. All 1,354 frozen inputs remained unchanged.
[Exact commands, logs, source identities and prior cache failure](evidence/native-window-terminal-errors.json)
are retained. The first attempt used a stale shared-target build script from a
newer checkout; cleaning the 46 local workspace package artifacts corrected the
cache without changing production code. Current-main integration is pending.

This fixes the identified error-loss paths. It does not establish why the
historical instrumented process failed, change audio-overload handling, or claim
a new native interaction, device-performance or rendered-pixel acceptance run.
