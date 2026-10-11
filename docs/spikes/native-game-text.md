# Packaged-font game text shaping

`incant_text` provides actual CPU text shaping from an explicitly ordered list of
font assets. It returns immutable glyph IDs, stable font ULIDs, absolute UTF-8
cluster ranges, baseline positions, advances, bidi levels and line geometry.
Arabic/Hebrew direction, Latin ligatures/combining marks, Unicode line breaks,
wrapping, resizing and missing-glyph reporting use the real shaping backend.
Unchanged input returns the retained `Arc`, without reshaping or copying glyphs.

The crate depends on no project document, command bus, runtime world, renderer or
agent. Loading owns packaged bytes; it never enumerates installed fonts or opens
a font path/network resource. An explicit `und` locale and private bounded family
aliases keep host locale and family-name collisions out of selection. Asset IDs
are supplied by the caller, so the ULID dependency does not enable random ID
generation. The library compiles for `wasm32-unknown-unknown` independently.

## Admission and resource boundaries

- At most 32 single-face TrueType/OpenType assets, 32 MiB each and 64 MiB total;
  duplicate IDs, empty chains and TTC collections refuse before font parsing.
- Every face must load in the actual rustybuzz-backed font system during
  construction. Metadata-only sfnt files return a typed error before shaping.
- The API selects concrete font files. Matching attributes are neutral metadata;
  original italic/bold/condensed outlines are unchanged, and incidental name,
  weight and style fields cannot reorder the supplied asset chain.
- No font name table is decoded for matching. Constructing fixed `FaceInfo`
  records prevents overlapping name records from amplifying string allocations.
- Each text update is limited to 16 KiB. Metrics are finite and bounded; returned
  output is checked for finite geometry, UTF-8 cluster boundaries, at most 65,536
  glyphs and 16,384 lines. These are input/output limits, not a measured hard
  bound on every internal allocation or shaping duration.
- One buffer and one result are retained. A rejected update does not replace the
  last accepted result. `.notdef` clusters remain explicit in the result.

The current pin is cosmic-text 0.14.2, the last release using the plan's specified
rustybuzz backend. Later releases change shaping engines. This slice does not
silently replace that stack; dependency updates remain separately reviewable.
Original OFL fonts and their exact upstream commit/hashes are recorded in
`crates/incant_text/tests/fonts/provenance.json`.

## Verification and review

Fifteen actual-font tests cover ligatures, combining marks, RTL and absolute
clusters across CRLF, exact asset priority, fallback, missing glyphs, line wrapping,
scaling, blank/trailing lines, retained results and malformed input. Regression
controls include a named but unloadable face, italic-only/bold-first/condensed
faces, a malformed fallback, and identical geometry after removing font names.
CI-profile tests, all-target Clippy, wasm32 compilation, runtime dependency guard,
formatting and generated conventions pass. See [check evidence](evidence/native-game-text.json).

Independent Sol 6.1 extra-high review found and reproduced three font-ingress
issues before publication. All are fixed: actual face preloading rejects malformed
fonts, fixed matching metadata preserves asset order, and direct bounded metadata
construction avoids decoded-name amplification. The same 646,352-byte malicious
name fixture fell from 143,507,456 to 9,256,960 bytes peak process RSS (regular
fixed baseline 9,093,120 bytes) in a private macOS debug probe. This is a diagnostic
before/after observation, not a shipping-profile or phone performance result.
The original and final [review receipts](evidence/native-game-text-review-final.json)
retain exact source, fixture and command identities.

## Remaining Phase 1 work

This is a shaping foundation, not a complete game UI or text renderer. Cooked font
assets, atlas rasterization/upload, draw integration, flexbox widgets, focus and
gamepad navigation, IME and mobile keyboards remain. CJK, Devanagari and emoji
coverage and Claude-owned rendered golden review are still required. No native
window, rendered appearance, reference-phone, performance or phase gate is passed
by these CPU results. The existing localization runtime remains a separate input
to future text widgets.
