# Private native pointer event observation

Parent source is `01199591fa0d841bd5310bbcbf441168e0a580c2`. This private
composition does not replace that production commit or qualify the failed
normal Mac attempt. Original native-relative-pointer-mac-001/normal-001 is
retained: the native process closed normally, but reported 1 capture/release
and zero raw events against the declared 3 cycles. The second child was held.

The default-off `diagnostic-pointer-input` feature enables a host-only
`--pointer-trace` flag immediately after `--relative-pointer SCALE`. Ordinary
builds refuse the flag. No source project, script or module can enable it or
choose an output path. Observation adds no input synthesis, cursor operation,
fallback from absolute coordinates, eligibility relaxation or retry.

The original event dispatch and flow decisions remain byte-identical. Optional
read-only snapshots surround them, including termination cleanup. Only relevant
native events and state changes become rows; stable frame callbacks do not.
The observer reads both tracked focus and Tao's current focus so disagreement
can be diagnosed. Key identities are limited to Escape, Tab and F; other keys
are one anonymous category and text is never retained. Device buttons are
distinct from window buttons. Raw-motion payloads are redacted outside eligible
focused capture, while their event presence remains observable. Cursor position
bits refer only to this window's CursorMoved events and are not raw movement.

`Vec<Row>` is allocated before entering the event loop, with exactly 512 row
slots. Compile-time checks limit each row to128 bytes and row storage to64KiB.
Pushes never grow the buffer. Additional relevant rows increment a saturated
omission count, and `inconclusive=true` prevents complete-observation claims.
The final structured line, prefixed `INCANT_POINTER_TRACE`, is limited to512KiB
and written to stderr after cleanup, including an original guest failure.
Writer failure cannot replace the original result; missing/truncated output is
inconclusive. There is no per-tick JSON or log output and no timing claim.

Each row contains callback sequence, elapsed microseconds, event kind/detail,
two IEEE754 value bit patterns, redaction flag and before/after state. State
bits0..7 are tracked focus, suspended, platform-suspended, minimized, supported
size, awaiting surface, actual Tao window focus, and termination requested.
Bits8..10 are captured, release-required and matching rearm release pending.
Bit11 says context bits0..7 were observed; final cleanup omits that context
rather than inventing false focus. Counts are captures, releases and delivered
raw-motion events. A window-button detail is `2*button+pressed`, with left0,
right1, middle2, other=n+3, and an unknown future button=65539 (outside the
Other(u16) domain). Device buttons preserve Tao's numeric ID with the
same pressed bit. Key detail uses anonymous0, Escape1, Tab2, F3 with pressed bit.
Cleanup records aggregate completion independently from cursor cleanup success.

Authored CPU controls cover capacity/overflow and fixed allocation, stable-frame
filtering, device/window event distinction, redaction/exact numeric bits, error
cleanup observation, bounded output, explicit option refusal and equality of
input events/backend calls/typed failures with and without snapshots. These are
not yet executed and do not replace the pending actual diagnostic child.

The first qualification stopped at compilation: Tao's non-exhaustive
MouseButton enum required a wildcard in the diagnostic observer. Source002
adds only the unknown-button sentinel above; the original failed001 log remains
retained, and input dispatch is unchanged.

The trace can discriminate no Tao raw event from an event rejected by host
eligibility, and show which intended clicks/Escape/focus transitions arrived.
No raw event alone cannot distinguish a missing native event from one whose
NSEvent delta fields were zero: pinned Tao only emits MouseMotion for nonzero
deltaX/deltaY. A diagnostic observation is not a pass of the original contract.
