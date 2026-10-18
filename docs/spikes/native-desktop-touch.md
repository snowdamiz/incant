# Native desktop touch delivery

The desktop packaged-player window adapter now routes Tao `Touch` events through
the existing native input runtime. Started, Moved, Ended and Cancelled become
ordered Down, Move, Up and Cancel packets in logical pixels. A fixed 16-contact
table keys the full native device and 64-bit contact identity. Runtime IDs are
monotonic 32-bit values, never truncated native IDs or recycled slot numbers.
Capacity, duplicate starts, invalid scale/coordinates and exhausted identities
refuse explicitly. Touch pressure and stylus properties are not exposed.

The existing tap, long-press, swipe and pinch recognizer remains authoritative.
The desktop action admission now accepts its existing tap/long-press/swipe
bindings; it still refuses unsupported mouse buttons and gamepad IDs. No SDK,
recording format, recognizer equation or authored mutation path is added.

An interactive UI hit captures a contact for its complete lifetime. One UI
pointer is shared with the mouse; concurrent UI presses are suppressed, and
cannot release the primary capture. Noninteractive HUD areas pass gameplay
touches. Crossing into interactive UI cancels a gameplay contact; it never
converts that contact into a UI press. UI cancellation cannot activate a widget.

Focus loss and suspension neutralize accepted input without needing a native
End. Layout/scale/surface changes cancel captures. Session replacement clears
the old contact authority while retaining the runtime ID allocator. Late native
movement/releases cannot create a new press. Starts received while the surface
or displayed hit-test coordinates are unavailable remain suppressed until End;
presentation cannot turn them into presses. UI owner/layout replacement also
conservatively cancels held gameplay contacts, even if an unrelated widget moved.

Native event admission snapshots only the fixed contact table and queue cursors.
A sequence/range/capacity refusal restores queued gameplay/UI receipts, contact
ownership and the ID allocator, including any ownership-fence cancellations
attempted during that event. The existing composed runtime retains its input
rejection semantics; the window host does not acquire a new automatic retry mode.

The separate mobile touch adapters and their delivery epochs are unchanged.
This slice delivers events supplied by Tao; it adds no platform touch driver or
physical-device evidence. Mouse events synthesized by a platform remain mouse
events under the existing platform contract. It does not claim native touchscreen,
Windows/Linux execution, gestures on real hardware, UI appearance or a Phase 1
gate pass.

Source controls cover all four native phases, wide/reused IDs, bounded contacts,
same-contact UI/gameplay ownership, mouse/touch capture, focus/surface/layout/
session cancellation, late events, queue rollback, exact gesture/action replay,
and rejected-tick VM/world/input/PCM preservation. CPU execution and strict lint
are pending a separately granted native target lease; no window/GPU/capture was
used for this source preparation.
