# Native relative-pointer lifecycle

Base: main284 (`9db293541ea7bd0ba03089a395c934c3bf2e2264`).

The native window host adds `--relative-pointer SCALE` after optional
`--font-cache` and before optional `--stop-after`. Absolute input is unchanged
when this host option is absent. The option does not grant a project, script,
module or agent any operating-system cursor authority.

`SCALE` is finite and within 1/1024..1024 virtual logical-pixel equivalents per
raw device unit. Pinned Tao reports device-dependent MouseMotion displacement,
not pixels; this is an explicit sensitivity convention, not a measured DPI
conversion. Existing `InputEvent::PointerMotion` units/SDK/recording ABI are
unchanged. Each raw component and accumulated scaled run is bounded to one
million in its respective units. Invalid values refuse atomically before
pending input or diagnostics change. Consecutive deltas sum, preserving discrete
event boundaries; they never create an absolute position. The report retains
the final raw and scaled deltas separately, plus event/capture/release counts.

The first focused, active in-window left click acquires capture; the cursor is
hidden only after successful acquisition. Its press and matching release are
consumed. Other mouse presses/releases and wheels route to gameplay while
captured, never GUI pointer handling. Captured CursorMoved/Entered/Left events
are consumed. Only DeviceEvent::MouseMotion is delivered; duplicate raw axis
events are ignored. Global device motion outside focused active capture is not
validated, stored or forwarded.

Escape releases capture and resets pending physical state. Focus changes,
platform suspend, resize/scale changes, non-presentable surfaces, acquisition
waits, session replacement, exit and errors also release. Focus return/resume
never reacquires: another explicit click is required. A lost rearm release does
not swallow a later new press. Enter/leave capture invalidates UI pointer
receipts and resets physical focus state to avoid stale buttons/position. Text
and keyboard keep their existing routes. Escape returns ordinary absolute
interaction until the next rearm click.

One RAII capture owner retains the native window. Revocation precedes OS
cleanup; visibility is restored even if release fails. Failed acquisition
attempts release immediately and preserves its original error. Drop retries
outstanding cleanup, including unwinding; ordinary abort/process kill cannot
run Rust Drop. OS release errors remain errors, not claimed successful release.
No global input or new capture is admitted after termination.

## Scope and verification

Focused controls cover default/inactive behavior, explicit rearm and matching
release, gameplay versus UI routing, exact summed deltas through InputRuntime,
no invented absolute position, finite/range/capacity refusal, failed acquisition,
release failure, drop and unwind cleanup. They use a mock cursor backend and do
not establish real OS capture. Existing native input/UI/lifecycle tests remain
part of the CPU suite. A real mouse/cursor exercise requires a separately
coordinated launch because it temporarily affects the user's pointer. No GPU,
shipping/performance or real-device claim is made by these source controls.
