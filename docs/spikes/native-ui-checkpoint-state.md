# Native widget checkpoint state

This implements the bounded CPU widget-state section of ordinary game loading.
`UiRuntime::checkpoint_bytes` saves accepted committed text, UTF-8 grapheme
selection and logical focus. `UiRuntime::from_checkpoint_bytes` constructs a
fresh owner and admits the data against a supplied committed widget tree and
newly measured viewport. It does not mutate an existing owner on failure.

This section is **not yet connected to complete native world/session saves**.
Their existing explicit UI-save refusals remain in place. A composed game host
must bind this section to its exact saved world and trusted program, create a
fresh input router, discard queued OS events and prepare its rendering consumer
from the restored state before exposing a loaded session. That integration is
the next step; this change does not claim a complete save or Game UI gate.

## Persisted and transient state

Every widget's stable ID, parent and kind must match, including widgets without
editable text. Vector order and native entity incarnations may differ. Styles,
font chains and noneditable label/button text come from the supplied tree. The
outer session loader remains responsible for world/program compatibility; this
widget section does not authenticate arbitrary caller-supplied trees.

All editable buffers and both selection endpoints are restored exactly, including
reversed selections, combining sequences and supplementary Unicode characters.
Selections must fall on extended grapheme boundaries. Saved focus must name a
visible, enabled, focusable widget in the restored layout; incompatible focus is
refused rather than silently moved. Active composition makes saving fail until
the caller explicitly commits or cancels it. Saving never does so implicitly.

Pointer capture, held keys/gamepad controls, OS window focus, IME client serials,
focus/pointer receipts, notifications and cached geometry are transient. A fresh
owner invalidates every old receipt even when numeric generations happen to
match. Loading a button press does not invent a release or activation. Native
IME contexts must be recreated for the new opaque focus token by the platform
adapter; this codec never stores or reuses a native context.

## Wire contract

The binary `INCUI001` envelope has an 80-byte header: version 1, ordinary-input
policy 1, row count, optional focus ID, body size, reserved zeros and SHA-256.
The digest covers header bytes 0–47 and the complete body. It detects accidental
corruption; it provides no authentication. All integers are little endian;
stable IDs retain their existing 16-byte ordering.

Rows sort strictly by stable ID. Each has a 48-byte fixed prefix containing ID,
tagged optional parent, widget kind, reserved zeros, selection endpoints and
editable text byte length. Only text-input rows may carry text or selection.
UTF-8 bytes immediately follow each row. Unknown tags, duplicates, unordered
rows, invalid flags/ranges, truncation and trailing bytes fail explicitly.

The limit is 1,097,808 bytes: the header, at most 1,024 row prefixes and at most
1 MiB of editable text. Each buffer is also limited to 16 KiB. Restore checks
the combined editable and supplied noneditable text against the existing 1 MiB
scene limit before layout. It does not deserialize JSON, executable code, heap
graphs, input devices or authoring documents. Save/load allocation is confined
to explicit checkpoint operations, not the simulation tick.

## Verification

On aarch64 macOS, all 41 `incant_ui` tests pass, including 14 new checkpoint
cases. Coverage includes Unicode/reversed selection round trips; canonical
bytes across reordered trees/new incarnations/viewport changes; old token and
pending-layout rejection; composition refusal without mutation; unaccepted
candidate isolation; neutral pointer release and notifications; changed
membership/parent/kind; hidden/disabled/absent focus; every truncation and
single-byte corruption of the fixture; checksummed malformed records; invalid
grapheme positions; failed measurement preserving the old owner; and combined
1 MiB text admission.

All-target Clippy with denied warnings, wasm32 compilation, authoring-free
dependency validation, formatting and generated conventions pass. Independent
source review found no actionable defect and confirmed the integration limits.
The first test run's sole failure was an invalid synthetic text measurer which
returned an extent above the layout limit for the maximum-text fixture. The
bounded fixture correction and both runs are retained in the evidence.

No GPU, native input-method execution, exported-player continuation, shipping
performance or device claim is made here. See
[the retained source, logs and review receipt](evidence/native-ui-checkpoint-state.json).
