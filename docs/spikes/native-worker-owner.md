# Trusted native worker owner

`tools.play_worker.start()` builds the fixed physics variant selected by the
project, copies its actual compiler artifact under the engine cache lock, and
yields a retained CPU worker. The child must confirm the exact project document
hash, protocol version and physics variant before use. The same preparation
context now serves the existing one-shot `play_native.py` command.

The owner exposes `step`, selected `observe` and `stop`. It supplies monotonic
request IDs, rejects non-finite/oversized JSON locally, validates response identity
and bounds encoded responses. Writes and reads both have deadlines. A refused
native request consumes its ID without poisoning a healthy connection; protocol,
IO and deadline failures close the child. A failed connection cannot subsequently
produce a successful completion receipt.

Interactive pipes reuse the existing trusted-tool process ownership. POSIX keeps
the unreaped group leader until descendant cleanup. Windows assigns its gated
wrapper to a non-breakaway, kill-on-close Job Object before launching the command.
The wrapper reads exactly three gate bytes without buffering the first worker
request. Successful Stop, caller exceptions, timeouts and context exit all reap the
owned tree before the private binary is removed. This is lifecycle ownership for
fixed engine executables, not a sandbox or guest shell capability.

The [evidence manifest](evidence/native-worker-owner.json) retains 43 artifacts.
Actual developer execution starts deterministic Play, publicly changes the project
to fast while the old session stays deterministic, then starts a matching fast
worker. Both reach the same 12-tick state as ordinary native Play; the fast worker
continues after removing its private source/cache. A caller exception also cleans
up its worker and private executable. Actual solver compiler artifacts are checked
for each selection.

The full pre-correction tools suite passed 292 Python cases (four Windows-only
skips), 22 Node cases and formatting/convention checks. Independent review found an
unclosed stdin descriptor when Darwin exit-observer construction fails. A real
regression reproduced it; the one-line cleanup correction then passed all 54
process/worker tests (four Windows-only skips). Earlier successful execution is
reused only for the unchanged normal paths. Windows runtime execution remains a
release-CI check. There is no editor GPU embedding, shipping-performance or
physical-device claim, and this mover fixture has no active physics bodies.

[Independent review](evidence/native-worker-owner-independent-review.json) accepts
the source and retained execution packet after the stdin cleanup correction.
