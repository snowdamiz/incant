# Typed text scene bindings and packaged font data

This slice connects the existing public font importer and command bus to cooked
native text declarations. It adds bounded font packaging and an actual
source-deleted numerical consumer. It does **not** complete game text rendering,
Game UI, or successful mixed-scene exported-player startup. Existing plain game,
canonical render input and legacy gameplay paths explicitly refuse text until
an actual composed text consumer is connected. No second player/CLI mode is added.

## Data and lifetime contract

`incant_types::Text` declares literal UTF-8 text, an explicit ordered font chain,
font size, line height, width and wrapping. Font fallback order is preserved.
There is no implicit host font lookup or interpretation of a string as a locale
key. Layout metrics are numerical inputs; typography, placement, clipping and
widget appearance have not been selected or approved by this slice.

The scene writes version 12 only when it carries text or a font catalog. The
existing v1–11 encoding selection remains unchanged for scenes without these
fields. After the v11 prefix, v12 adds canonical stable-ID font references with
recipe fingerprints, then canonical entity text declarations. Counts, UTF-8,
finite metrics, tags, duplicates and references are checked before publication.
The existing save envelope v3/v4 retains the nested v12 scene; it introduces no
new time state. Trusted destination restore additionally requires identical
immutable font references and the existing origin/scene/rate/catalog guards.
Unbound legacy saves refuse text capability; checksums are not authentication.

The native world retains actual shared text components. Numeric motion,
reparenting and unrelated archetype changes retain their allocation identities.
`SetText` replaces/removes configuration in the existing structural transaction;
all final data and references are checked before publication. Despawn removes
text, reused entity IDs do not inherit it, equal updates do not bump generation,
and restore advances the checked text generation while preserving destination
world identity. Borrowed `TextView` exposes that identity, generation, immutable
font bindings and current declarations. No per-tick document/JSON conversion,
font loading or shaping is added.

## Bounds and package behavior

A scene admits at most 32 catalog fonts, 32 fonts per ordered chain, 1,024 text
nodes, 16 KiB UTF-8 per node and 1 MiB total UTF-8. Font size is 1–512, line height
1–4,096 and width 1–65,536, all finite. The common 128 MiB scene limit still
applies. Existing atlas/shaping/raster bounds remain unchanged.

`FontCatalog::load` reads and validates all bounded manifests first. It totals
claimed raw font sizes before opening any payload; accepted metadata is consumed
without rereading it. A font is at most 32 MiB and the full catalog at most 64 MiB.
Each stable asset counts independently even if identical file content deduplicates
on disk. Payload header, exact length, recipe/content hashes and the sfnt face
remain validated by the actual cooked reader. No source dependency path is read.
These are payload/configuration bounds, not whole-process RSS or a universal
shaper/allocator bound.

The actual `embed_project` producer records font references in its existing
receipt. The native exporter's data packager copies only those manifest/blob
files, checks exact font envelopes and raw-content hashes, accounts logical
catalog bytes before payload reads, and retains the existing combined 2 GiB
package cap. Startup receipt hashes and create-only publication stay unchanged.
The exporter still requires its real player startup tick; that gate is not
bypassed. Consequently text games remain explicitly refused there until the
composed game text consumer is implemented.

## Verification and remaining work

The focused fixture uses public font import and atomic agent-provenance commands,
Undo/Redo, scene cooking, the actual export producer and data packager. It removes
font source, project, producer and original cache, then launches a fresh test
process to load the packaged v12/font data, shape Latin/Arabic/CJK and rasterize
nonempty atlas pixels. This is a numerical test process, not a shipping-player or
rendered-pixel acceptance claim. The small CJK fixture supports its documented
subset; the first fixture incorrectly requested missing characters and was
corrected to that explicit subset, without changing font or shaping behavior.

Runtime controls cover canonical round trips, checksum-valid malformed sections,
aggregate admission, retained identity, atomic rejected edits, despawn/reuse and
save/catalog substitution. Plain-game and renderer refusal occur before VM/cache
startup; legacy startup and sync refuse without disturbing ongoing motion.

Next, extend the existing composed CPU game host and canonical input with actual
retained layout/atlas state, shared font bytes and combined consumer budgets.
Then connect that same input to renderer upload/draw with Claude appearance
acceptance. Successful ordinary mixed text/spatial/audio export, widget layout,
IME/editable text, gamepad focus, GPU/device measurements and reference-phone gates
remain outstanding. No GPU, screenshot, provider or device evidence is claimed.

## Final local check receipt

The frozen source passed 311 Rust tests in 73 result groups (zero failures or
ignored tests), all-target Clippy with warnings denied, 11 Python package-copy
tests, strict SDK/template typing and the generated bridge/SDK checks. The Rust
schema generator preserved all 26 existing document schemas and added only Text.
Normal runtime dependency guards passed. The font-enabled reader and native world
also compile for wasm32-unknown-unknown; this is not browser execution evidence.

[The durable evidence](evidence/native-text-bindings.json) pins all 37 reviewed
inputs, both generated outputs and the exact logs, including preliminary fixture
failures. [Independent root review](evidence/native-text-bindings-root-review.json)
accepted that scope with no findings. All owned commands were terminal before
this commit; no runtime or visual acceptance is inferred from these checks.

## Actual main118 integration

The original reviewed feature is preserved on actual main118. Its 38 reviewed
source/test/generated hashes are unchanged; the lock retains current main
resolutions and only the original three cooker dev-dependency edges are added.
The integrated scoped run passes 320 Rust tests in 75 groups (zero failures or
ignored cases), all-target Clippy, both wasm32 checks, 11 Python package/font
controls, all 27 actual document schemas, strict SDK/template types and generated
bridge/SDK checks. The actual-main merge changes no tested tree bytes.
[Integration evidence](evidence/native-text-bindings-main118.json) retains exact
commands and log hashes. This remains the data/package foundation with explicit
Text consumer refusals; no rendering or mixed-game export acceptance is inferred.

## Composed CPU consumer integration

The earlier foundation refusal statements describe that historical source. The
[composed CPU text input](native-text-input.md#actual-main121-integration) now loads
packaged fonts through the existing game host and passes the actual mixed shipping
export after deleting source and private build storage. Plain unsupported routes
and GPU text rendering retain explicit refusals. This does not claim text draw,
widget, IME or visual acceptance.
