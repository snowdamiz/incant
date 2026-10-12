# Retained text in the composed native CPU host

The existing `NativeRenderHost` now prepares real shaped text and an alpha glyph
atlas alongside spatial input. The same mandatory `SceneHost` sync accepts both
at a committed tick, following script/physics/audio. It is not a second player or
text-only mode. The existing package route supplies its explicit font directory;
`PreparedPlay` uses the checked project font cache. Default unhosted games and the
old spatial-only adapter remain strict.

## Input, ownership and publication

`SceneInput.text` carries the actual shaped glyphs, source clusters, bidi levels,
missing clusters, glyph atlas handles and alpha pixels. Each text node carries the
same committed f64 global transform as its native entity. These coordinates do
not specify widget anchoring, clipping, color or an approved visual design.

Only checked cooked font data is loaded. Shared immutable byte allocations feed
ordered shaping chains, so raw font bytes are not copied per label or fallback
order. The pinned cosmic-text 0.14.2 `Font::new` also clones its fontdb binary Arc
into its owned rustybuzz face. No system fonts, source-file lookup or network path
is involved. Font catalogs are frozen for the adapter's world lifetime.

Text and spatial views must have the same actual world identity before any
candidate writes or consumer call. A changed text generation prepares all layouts
and a replacement atlas using retained font bytes. A changed spatial topology
with unchanged text reuses the accepted layout/atlas Arcs and remaps numerical
slots. Motion copies only transforms through the existing preallocated undo
transaction. Error or unwind restores old text and spatial input together; a
failed preparation never invokes the consumer. The enclosing game still treats
post-accepted-tick failure as terminal and clears offline PCM, as before.

A consumer receives this canonical text field and must implement it or refuse.
Canonical renderer resolution, retained preparation and retained staging refuse
it before GPU submission. This slice does not draw text, upload its atlas, create
a GPU device in its proof, or claim an appearance review.

## Explicit resource bounds

The native catalog retains at most 64 MiB of raw font bytes (32 fonts, at most
32 MiB each). Model/texture plus raw font bytes also share the adapter's existing
256 MiB admitted payload ceiling. Startup can stage one bounded catalog beyond
that combined ceiling before refusing; external retained adapters add to process
peak. Font contents remain shared during candidate preparation.

One preparation admits at most 32 distinct ordered font chains, 65,536 glyph
placements, 16,384 layout lines and the existing world text limits (1,024 nodes,
16 KiB UTF-8 per node, 1 MiB total). Aggregate glyph/line counts are checked before
candidate atlas allocation or any raster work. Shaping one bounded node can stage
one additional layout beyond the aggregate limit before refusal. Accepted old
layouts remain alive during candidate preparation, so the bound is old accepted
layout + candidate accepted prefix + one individually bounded layout. The shaper
and each font engine retain their own bounded-input caches/scratch; these counts
are not a universal heap or RSS measurement.

Each alpha plane is at most 4 MiB and each atlas at most 8,192 distinct glyph keys.
Old plus candidate alpha planes can occupy 8 MiB. Atlas entries, key maps, glyph
handles and shape vectors are additional counted data, not included in pixel
bytes. Each atlas also retains the existing raster scratch: at most a 1,024²
coverage mask and 17,408 outline commands, plus the pinned rasterizer's working
storage. Its expanded outline admission remains 8,192 points / 1,024 visits.
Both accepted and candidate atlases may retain that scratch until dropped.
Numeric slot maps follow the existing bounded spatial entity count and are
rebuilt only at topology/configuration boundaries. No total-process/mobile
memory or performance gate is inferred.

## Verification scope

CPU controls exercise actual source/cache-deleted layout and rasterization,
Latin/Arabic/CJK font selection, ligatures, bidi, per-entity poses, and the existing
composed model + QuickJS + audible PCM path. Other controls cover motion/error
and unwind rollback, failed atlas admission, aggregate layout refusal before
raster, restore invalidation, despawn/reuse, unchanged topology reuse and shared
font lifetime. A thread-local allocator wrapper measures 64 accepted and 64
rejected adapter sync calls with zero allocations, excluding world stepping,
startup, text changes, consumer work and the full frame pipeline.

The final affected CPU suite passes 120 tests in 31 result groups; 74 existing
GPU tests remain ignored. All-target Clippy passes for the seven affected host,
renderer, text, player and Play packages, including the optional native-window
feature's compile path. This does not execute a window or GPU.

The public proof uses existing font/model/audio import commands and reversible
script/entity transactions, the actual fresh shipping exporter and its mandatory
startup check, then three ordinary `--package` launches after deleting all private
authoring/build inputs. It observes multilingual glyph/atlas data, matching text
and mesh positions, native scripted motion and audible PCM. Missing/tampered
packaged fonts refuse before output publication. The final proof passed three
fresh 20-tick runs: eight shaped glyphs across three fonts, three RTL glyphs,
no missing clusters, 723 nonzero alpha texels and text/model X positions both
`2.0000000000000004`. All three atlas hashes and audible PCM hashes match. The
fresh aarch64-apple-darwin shipping player is 8,673,040 bytes; its actual codegen,
compiler-free symbols, normal dependency graph and source/build deletion checks
passed. No device or timing result is inferred from this local executable size.
The exact [process receipt](evidence/native-text-input-process.json) and
[source/check evidence](evidence/native-text-input.json) are retained.

The first fresh export built and passed its copied-player startup check, then
found that the old observation splitter did not accept the font-cache-extended
argument shape. The correction admits exactly that final shape and preserves
duplicate/unknown option refusal. A package regression now feeds the expanded
arguments through both the observation splitter and the actual mode parser.
The failed proof log is retained; its startup is not counted as a successful
observed text-game run.

The next draw integration's [technical handoff](native-text-draw-handoff.md)
describes the canonical ownership, units, baseline/coverage convention and
required transaction behavior; it does not approve an appearance.

Outstanding: actual text upload/draw and Claude pixel acceptance, flexbox/widgets,
IME/editable text, focus/gamepad navigation, emoji/color or unsupported outline
formats, clipping and UI styling, and reference-phone measurements. The CPU route
requires the same explicit camera selection as its existing spatial host.

Independent read-only acceptance and exact source/process identities are retained in
[evidence/native-text-input-independent-review.json](evidence/native-text-input-independent-review.json).
The review found no actionable issue within the stated CPU preparation, rollback,
admission and public export scope; drawing and complete game UI remain pending.
