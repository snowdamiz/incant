# Incant patch to cosmic-text 0.14.2

Original: crates.io `cosmic-text = 0.14.2`, archive SHA-256
`da46a9d5a8905cc538a4a5bceb6a4510de7a51049c5588c0114efce102bcbbe8`.
The published VCS metadata identifies upstream commit
`9e7a56f083db15f67510df4396351464df2e64bd`.
The published source, tests, fixtures, licenses and manifest are retained; only
Cargo's registry cache markers `.cargo-ok` and `.cargo-checksum.json` are omitted.
MIT OR Apache-2.0 licensing is unchanged. This keeps the plan's pinned rustybuzz
shaper; no system font discovery is enabled in Incant.

Incant changes:

- `.gitattributes`: remove upstream Git LFS filters. Published font/image bytes
  stay in Git directly, so configured LFS clients need no external objects.
  Root attributes preserve published CRLF and fixture/license whitespace.

- `src/shape.rs`: a wrapped line must commit when it contains an earlier bidi
  span, even if the current span has accumulated no width. Otherwise the next
  word exceeds the line width or an interior space is skipped. The reverse
  glyph fallback must start at the current word's exclusive upper boundary;
  starting at its lower boundary loses a glyph. Before final placement, trim
  only overflowing end blanks, including those committed in earlier spans.
  Fitting and interior spaces, no-wrap overflow, oversized unbroken words in
  Word mode, and oversized indivisible glyphs remain supported.
- `src/attrs.rs`, `src/buffer.rs`, `src/edit/editor.rs`: spell inferred lifetimes
  explicitly and feature-gate Swash-only imports for the current Rust toolchain.
  These are warning-only changes, with no layout behavior change.

The workspace selects these exact sources using `[patch.crates-io]` and excludes
this dependency from its member list. Do not edit Cargo's shared registry cache.
`crates/incant_text/tests/bidi_wrap.rs` exercises observable multilingual wrapping,
cluster preservation, missing-glyph fallback, whitespace and both paragraph
orientations using explicitly packaged fonts. The source delta from the published
crate is retained in `INCANT_PATCH.diff` for review.

Upstream context: [mixed-direction overflow issue #252](https://github.com/pop-os/cosmic-text/issues/252).
This is an Incant correction on the pinned release, not an upstream fix claim.
