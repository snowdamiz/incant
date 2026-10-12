# Native UI layout, focus and composed CPU input

This slice connects authored `UiNode` components to cooked binary scenes, a
retained Taffy layout/focus owner, and the ordinary composed game/player. It is
not completion of Phase 1 Game UI: screen-space GPU drawing, platform event/IME
bridges, widgets beyond the small typed set, accessibility, and UI checkpoint
state remain open. World-space `Text` retains its existing semantics.

## Data and ownership

`UiNode` supports Root, Container, Label, Button and TextInput. Its typed flexbox
subset describes logical-pixel sizes, fractions, padding/gaps, direction, wrapping,
alignment and growth. These are neutral layout semantics; this change supplies
no colors, decoration, caret artwork or presentation defaults. Claude owns the
future drawing and appearance work.

One scene has at most one parentless UI Root. UI children have a UI container
parent, and UI containers cannot parent ordinary world entities. UI entities
have identity transforms and no native velocity/spatial/audio/physics/navigation
components; Script attachments are allowed. Normal command-bus edits and native
structural buffers validate the complete final hierarchy. Reparent/despawn cannot
leave retained parent links behind. A native incarnation, not just the authored
ULID, identifies the lifetime of a widget.

Cooked scene v13 appends a checked UI table after the v12 font/text section.
Versions 1–12 continue to emit exactly as before for scenes without UI. Font
fingerprints and payloads use the existing checked font cache and native package.
No runtime project, command-bus, CRDT or JSON dependency enters the layout owner.
The old legacy gameplay path explicitly refuses UiNode.

`NativeRenderHost` may be configured with `UiSettings`. It shares immutable font
bytes with world Text, but owns separate screen-space shaping, widget values and
atlas data. It requires the exact `WorldIdentity`, UI generation and native
incarnation set. A missing UI-capable host/consumer refuses before VM/audio
preparation or GPU submission. The CPU observer and the GPU consumer accept the
canonical data ([GPU drawing](native-ui-draw.md)); the native window host does
not yet configure UI.

## Input and transaction contract

The retained router validates all physical input, even consumed input. It applies
explicit receipts first, then physical events in packet order. Keyboard Tab and
gamepad D-pad traverse only when an eligible widget exists; activation requires
focus. A Root/Label-only HUD does not consume gameplay buttons. TextInput owns
physical keyboard controls while focused, but keys never synthesize text.

Forwarded and suppressed down/up state is retained for keyboard and gamepad.
Entering UI ownership releases a previously forwarded held control once, then
suppresses it until physical release. Removing UI never manufactures a new press.
Focus loss/disconnect clears ownership. The caller must not duplicate a pointer
receipt as an independent gameplay mouse event.

Delayed pointer receipts carry opaque owner/layout/delivery identities; delayed
text/preedit receipts carry owner/focus identities. They cannot address a new
world, rebuilt owner, replaced entity or changed focus. Well-formed stale receipts
are discarded and counted, while a same-packet global focus loss still applies.
Malformed packets reject atomically. Trusted explicit `Focus(id)` is a current
owner request, not a serialized asynchronous platform receipt.

Accepted widget events become bounded readonly `tick.input.ui` notifications.
Consumed controls are absent from gameplay edges/actions. Script/world changes
and UI state publish through the same composed tick. A malformed input packet
leaves the accepted session usable; a script/native/host failure after tick
acceptance follows the existing terminal session policy, silences requested PCM
and exposes no mixed UI/world candidate. No new rollback simulation path exists.

Text editing uses the accepted `incant_text::TextBuffer`: grapheme operations,
selection, preedit and commit are disposable runtime state. Rebuilds preserve a
value only for the same entity incarnation and authored initial text, cancel
composition, and fence old receipts. Viewport relayout is an explicit presentation
operation without a world/input/audio tick or authored mutation. Focus can remain
on an unchanged offscreen widget after viewport-only clipping; hit testing uses
the new clip, and structural rebuild validates focus visibility again.

## Retention and limits

| Resource | Admission |
|---|---|
| Widgets / hierarchy | 1,024 / depth 64 |
| Authored or displayed text | 16 KiB per widget, 1 MiB aggregate |
| Fonts / font bytes / distinct shaping chains | 32 / 64 MiB / 32 |
| Logical viewport/extent | finite 1–65,536 pixels per viewport axis |
| Logical UI commands | 256 per packet, 64 KiB transferred text |
| Physical input | existing 1,024-event packet bound |
| Taffy measurement calls | 8,192 per candidate |
| Canonical glyph placements / lines | 65,536 / 16,384 |
| UI alpha atlas / keys | 4 MiB / 8,192 |

At most one accepted and one candidate UI atlas overlap (8 MiB alpha planes),
plus the independently bounded world-Text atlas pair. Aggregate glyph/line counts
are checked before candidate atlas allocation/rasterization; shaping can stage
one individually bounded layout beyond the aggregate before refusal. Raw font
bytes are Arc-shared, including old/new owners. Changed layout trees and shaping
engines can overlap. These are source-level resource limits, not a universal
RSS, allocator, shaping-cache or reference-phone performance claim.

Unchanged viewport/content reuses the exact LayoutFrame and UiPaint Arcs without
remeasurement/rasterization. Layout retains direct widget↔layout indices; routing
eligibility is O(1), and directional navigation/pointer selection scan at most
one 1,024-node array. This is not a claim that a complete input tick allocates
nothing. Glyph handles are bound to their atlas owner and never silently rebind.

## Public package and replay

The existing offline exporter accepts optional `--ui-viewport WIDTHxHEIGHT` for
its copied-player startup check. The packaged CPU player accepts the same explicit
viewport with `--package`; this is not a separate player mode. This initial host
still requires an explicitly selected camera even for a UI-only scene.

Input recording v2 adds bounded scheduled `ui` commands and optional viewport
per frame. Version 1 remains unchanged and rejects these extensions. These are
deliberate timeline commands resolved against the current owner at their tick,
not serialized platform tokens. Focus-dependent text commands require focus from
a preceding tick; a same-frame resize precedes pointer-token resolution. Seeking
past UI history explicitly refuses until UI checkpoint state is supported.

`tools/probes/native-ui-export.py` authors the mixed fixture through public import
and RPC commands, checks Undo/Redo/journal reopen, invokes the real exporter,
deletes disposable authoring and private build storage, then launches fresh
ordinary packaged games. It observes keyboard/gamepad activation, preedit/commit,
submit, resize/pointer, native motion, world Text and PCM. Negative controls cover
missing configuration, malformed receipts, font integrity and unsupported saves.

QuickJS receives the routed UI contract. Static Wasm and development Wasm
refuse UI until their input ABI supports it. The integrated DevelopmentWasm guard
runs before module initialization; a malformed-module regression confirms that
UI refusal takes precedence. UI-used worlds explicitly refuse
world/session checkpoints; this is a temporary missing state adapter, not the
final save contract. Rendering, OS IME behavior, pixel acceptance and devices
must be validated separately.

## CPU freeze and evidence

The frozen composed suite passed 226 tests in 28 result groups; the affected
runtime/cook/document suite passed 279 in 63 groups. These suites overlap and
must not be summed as unique tests. All-target Clippy passed for the eleven
affected packages, with warnings denied. Strict generated TypeScript, bridge,
conventions and the normal-runtime dependency guard passed. Exact source/log
identities are in [the CPU receipt](evidence/native-ui-layout-cpu.json).

The first actual fresh export built and passed copied-player startup, then
exposed a real `--observe` option-length parser defect. The corrected parser has
an actual package/UI-option regression. A preliminary source-deleted replay of
that first executable without observation passed the expected UI notifications,
value, resize and PCM controls; it predates the final readonly sequence correction.
The CPU commit deliberately preceded the final process proof. Subsequent proof003
passed three fresh 12-tick runs with exact button/edit/submit notifications,
native motion, world Text and equal nonzero PCM/atlas results. Five controls
refused at their intended typed boundaries, including `World(UiUnsupported)` for
an explicitly persistence-opted-in script. Compiler-disabled module Clippy,
wasm32, fresh 67-schema equality and the lean dependency guard also pass.

Proof003 initially retained its external debug source map after deleting the
project/cache/build storage. That map was then hashed and removed, and all three
positive plus five refusal processes repeated on the same executable with exact
report/PCM equality. The original receipt is preserved; the strengthened helper
now removes maps before runs. The coordinator's separate current-main recook will
exercise that final helper from the beginning. This distinction avoids claiming
complete external source removal at the original proof003 stage. Exact receipts
and hashes are in [the final evidence](evidence/native-ui-layout-final.json) and
[the independent CPU review](evidence/native-ui-layout-root-review.json).
No rendered pixels, OS IME behavior or complete Game UI acceptance is claimed.

See [the draw-consumer handoff](native-ui-draw-handoff.md) for the exact canonical
input and transaction seam reserved for Claude's subsequent implementation.

## Actual main129 integration

The main128 composition passed 510 CPU tests in 87 groups plus scoped all-target
Clippy, native-window compilation, compiler-disabled/static and wasm32 checks.
Adding the reviewed Wasm steering source passed another 209 overlapping script,
Play and game-runtime cases, all-target Clippy and the matching headless build.
Its 67 freshly generated schemas match the committed bytes.

The current-lock public export now removes its own authoring/cache inputs, private
build storage and debug source maps before any copied-player replay. Three fresh
12-tick runs pass exact UI events, native motion, world Text and matching PCM;
five controls reject at the intended configuration/input/UI-checkpoint/font
boundaries. The shipping player is 9,053,504 bytes, SHA-256
`4e4b58b8511936639d5365c46f7e2500c731a47b01f4e708691a5ffe3f86a889`.
Actual main129 merged without conflict afterward; only docs and the independently
reviewed development-probe metadata selector changed, outside this export's
source/build closure. Source and proof identities are in the
[integrated receipt](evidence/native-ui-current-main.json).

The initial integrated compile used unstable iterator `is_empty`; the existing
stable `len() != 0` form fixes that compile error without changing the refusal.
The failure log is retained. No rendered UI, native input/IME, device or full
checkpoint gate follows from these CPU results.

Independent final integration review accepted all 84 changed source identities,
20 retained logs, 19 receipts and the actual package/replay bytes. The
[review receipt](evidence/native-ui-current-main-root-review.json) binds the
CPU scope and preserves the outstanding drawing/platform gates.
