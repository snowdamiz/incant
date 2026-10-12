# macOS text-input retirement crash

Losing focus while editing a native text field retired its Cocoa input context
and could crash the game. Claude0057 recorded four failures in the composed
window. A separate invisible-window regression reproduces the fatal abort on
actual AppKit without drawing or injecting input.

The managed protocol client inherited `NSView` and overrode `window` using a
Rust-owned weak reference. Pinned objc2 0.6.5 drops Rust ivars before invoking
superclass deallocation. AppKit’s view finalization calls that override while
removing the view from its focus chain, after the weak reference has died.
The native crash stack identifies this exact reentry.

The client now inherits `NSObject` and uses its designated initializer. It is
an `NSTextInputClient`, never a displayed or attached view; the pinned
`NSTextInputContext::initWithClient` contract accepts that protocol object.
Removing view destruction removes the callback into dropped state. The window
remains weakly owned, contexts still retain clients, and immutable session IDs
and retirement fencing remain unchanged. No object is leaked or kept alive by
a delay to hide the fault.

The same hidden native probe now passes 256 cycles, creating 768 contexts. It
checks repeated-owner idempotence, replacement, retirement, stale-session
refusal and closing with a context active. The comprehensive macOS CI workflow
runs it after the existing native-window tests; fast main checks stay unchanged.

All 126 affected UI/input/player tests pass across twenty groups, with no failed
or ignored cases. All-target native-player Clippy, dependency, formatting,
conventions and diff checks pass. The actual native probe passes again on the
final source. Independent source review accepted the superclass and lifetime
contract and the original red/green receipts without findings.

[Exact source hashes, original failure, final checks and review](evidence/macos-text-retirement.json)
retain the evidence. This does not establish visible native typing, live IME or
candidate-window behavior; a fresh composed-player follow-up remains required.
The separate Linux rendering candidate and other UI behavior findings are not
fixed by this change. No appearance, GPU, phone or phase gate is claimed.
