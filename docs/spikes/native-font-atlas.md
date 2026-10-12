# Retained CPU glyph atlas

`incant_text::GlyphAtlas` turns actual shaped glyphs into retained monochrome
alpha pixels using explicitly packaged fonts. `TextEngine::cache_glyph` takes an
immutable shaping result and glyph index, resolves its original font asset, and
returns an opaque atlas handle. The implementation uses pinned Zeno 0.3.3, the
rasterizer used by the existing cosmic-text/Swash ecosystem, with the existing
rustybuzz/ttf-parser stack. It never discovers host fonts or reads source paths.
No new third-party package or version resolution was introduced.

A glyph key contains the stable font ULID, immutable font-content SHA-256, glyph
ID and exact positive f32 size bits. A shaped result also retains its originating
engine identity: loading different font bytes under the same ULID cannot
reinterpret an old glyph ID. The font database and rasterizer share one immutable
byte allocation per font. Digests identify content; they are not authentication.

The atlas is append-only, with stable rectangles and one zero texel of padding
on every side. Cache hits return the retained entry without parsing/rasterizing
or allocating new storage. A handle is bound to its atlas identity and generation;
foreign handles and handles preceding `reset` fail lookup. Reset retains pixel
and entry allocations, clears coverage and keys, and advances checked generation
and revision counters. Insertion failure leaves published pixels, placements,
handles and revision unchanged. Private raster scratch can grow on a failed miss.
There is no implicit eviction or relocation: a full atlas returns a typed limit
error, and callers explicitly reset it or create another atlas. Callers can read
the revision and pixel slice for a future renderer upload boundary.

## Raster contract and bounds

Coverage is 8-bit nonzero-fill alpha at an integer-origin baseline, with signed
Y-down bearings. Shaped positions and advances remain the layout authority.
Glyph zero is explicitly marked missing; an empty outline such as space consumes
a key but no texels. Font size is 1–512 pixels. This first raster path supports
base/default-instance TrueType `glyf` outlines, including bounded XY-positioned
compound glyphs with supported transforms. It does not execute TrueType hinting
instructions, grid-fit compound offsets, accept custom variation axes or apply
subpixel positioning. It rejects CFF-only, bitmap/color/SVG font tables,
point-matched compounds and scaled component offsets instead of substituting a
flattened or unrelated glyph. Existing shaping and font-cook capabilities are
unchanged; a font that shapes successfully may explicitly refuse rasterization.

The compound-offset behavior follows the
[OpenType glyph-table contract](https://learn.microsoft.com/en-us/typography/opentype/spec/glyf):
unscaled offsets are the default, while unsupported scaled offsets refuse.
[Rust Zeno documentation](https://docs.rs/zeno/0.3.3/zeno/) describes the CPU mask
backend. No font program or project-selected code executes in the raster path.

| Resource | Enforced bound and point of refusal |
| --- | --- |
| Font chain | Existing 32 faces, 32 MiB per face, 64 MiB total before construction |
| Atlas alpha storage | At most 2048 × 2048 = 4 MiB, allocated once after configuration checks |
| Atlas entries | At most 8,192 configured keys, including empty glyphs |
| Expanded glyph graph | At most 1,024 visits and 32 stack entries; cycles and invalid references refuse before outline storage |
| Expanded control points | At most 8,192, with checked contour/flag/coordinate/instruction ranges before outline storage |
| Collected outline | At most 17,408 commands, checked independently by the sink |
| Raster dimensions | At most 1024 × 1024 = 1 MiB alpha scratch, checked before mask allocation |
| Transformed coordinates | Finite and within ±65,536 pixels before rasterization |

The atlas, mask and command buffers have these explicit storage ceilings. Entry
index nodes are bounded by the configured key count. Zeno's transient scan
conversion structures and pre-existing shaping internals are not asserted to
have a measured universal heap/RSS bound. Allocation failure is not recovered as
a Rust validation error. These limits are not device-budget/performance evidence.

## Observable checks

The scoped suite passes 31 tests: ten new raster/atlas/packaged-reader tests,
15 existing text tests and six existing asset/font tests. Actual OFL Latin,
Arabic, Hebrew and CJK glyphs produce nonempty fractional coverage; RTL shaping
and font identity remain intact. An independent analytic line/quadratic-Bezier
area integral agrees with integrated raster alpha to the stated tolerance, and
baseline bearing/dimension checks exercise the Y-down conversion. Numeric tests
verify padding, unchanged reuse, size keys, distinct loaded content, foreign and
stale handles, empty versus missing glyphs, and atomic dimension/key/full-atlas
refusal. Malformed/cyclic/oversized glyphs refuse before outline or mask capacity
is allocated; compact repeated children and deep chains exercise expansion caps.

The packaged test uses the real checked `.ifont` reader and
`CookedFont::into_source`, deletes the cache directory, then shapes and rasterizes
CJK. It constructs the documented cache envelope as test data; it does not claim
new public exporter/font-archive integration. The small CJK numerical fixture is
an OFL subset of a pinned Google Fonts Noto Sans SC revision, renamed
`IncantCJKFixture`. Source, license, recipe and generated-byte identities live in
`crates/incant_text/tests/fonts/cjk-provenance.json`; the recorded fonttools 4.60.1
recipe reproduces its bytes exactly. This is test data, not a typography/art
selection for the game or editor.

Scoped all-target Clippy, standalone wasm32 compilation, normal dependency
guards, formatting, conventions and lock/source identity checks pass. The wasm32
result is compile evidence only. The normal text graph remains free of document,
command-bus, agent, renderer and JSON dependencies; the real packaged-reader test
uses a dev-only asset-reader dependency. The runtime font feature remains opt-in.
[Machine-readable evidence](evidence/native-font-atlas.json) records exact inputs,
commands, logs and the earlier corrected test/build failures.

Independent review verified all fourteen frozen inputs, the recorded logs and
lockfile invariants, and accepted the bounded CPU scope with no actionable
findings. [Review receipt](evidence/native-font-atlas-root-review.json).

## Remaining integration

Renderer upload/draw consumption, binary scene text bindings, automatic font
packaging for exported games, flexbox widgets, focus/gamepad navigation, IME and
mobile keyboards remain. CFF, color emoji, broader script coverage, hinting and
custom variable-font rasterization are additional capability work. Claude owns
rendered acceptance and visual design; no image, screenshot, GPU, native window,
provider or device was used here. This slice does not complete Game UI, the Core
Sample, the reference-phone performance gates or Phase 1.
