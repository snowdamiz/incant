# Mixed-direction text wrapping

Claude's native text review exposed a real layout failure in the pinned
cosmic-text 0.14.2. With the packaged Latin/Arabic/CJK chain, the mixed string
`Wrapped ffi سلام 中文 ש and spaces` could exceed its declared line width and
omit an interior space. A public TextEngine regression reproduces 241.248 logical
pixels in a 180-pixel Word-wrapped line on the original source.

The correction retains cosmic-text 0.14.2 and rustybuzz 0.14.1 through the same
vendored-source pattern already used by the navigation dependency. Only four
upstream source files differ. The algorithm delta commits a line containing an
earlier bidi span, preserves the exclusive range boundary when splitting a
reverse-direction word into glyphs, and removes only overflowing trailing blanks
before alignment. Remaining changes spell lifetimes and feature-gate unused
imports on the pinned Rust toolchain. The published licenses, source, tests and
fixtures remain, with a reviewable [patch](../../third_party/cosmic-text/INCANT_PATCH.diff)
and [upstream identity](../../third_party/cosmic-text/INCANT_PATCH.md).

The regressions compare every non-whitespace cluster, font, glyph and bidi level
against unwrapped shaping across both paragraph directions, multiple widths,
long tokens, CRLF, ligatures, Arabic, CJK and observable missing Hebrew glyphs.
Interior spaces must survive. Word mode still allows an indivisible oversized
word; WordOrGlyph still allows a single glyph wider than the line. No-wrap retains
its previous behavior. All three causal mutations are caught by the actual tests,
and the exact reviewed source bytes were restored before final checks.

The affected text, native render-input and composed-host suites pass 63
CPU tests, with no failures or ignored cases. All-target Clippy, wasm32 text
compilation, the authoring-free runtime dependency check, formatting and generated
conventions also pass. Independent source review accepted the correction without
findings; the final checks used those unchanged frozen inputs.
[Machine-readable evidence](evidence/text-bidi-wrap.json).

Claude0055 will verify the corrected layout through the canonical native text
draw route. That rendered followup is pending, and this evidence does not claim
complete Unicode conformance, full game UI/IME, device performance or a phase gate.

A final repository portability correction retains the exact published CRLF
license/fixture bytes and removes upstream Git LFS filters for the already
included fonts/images. Narrow whitespace attributes preserve upstream inputs
and the raw patch. This changes no Rust source or font bytes; the original
source review and all numerical checks remain applicable. The vendor manifest
records the additional attributes delta.
The final committed-blob and CPU-log review is retained in
[evidence/text-bidi-wrap-final-review.json](evidence/text-bidi-wrap-final-review.json).
A fresh isolated checkout with CRLF conversion enabled and deliberately failing
required LFS filters preserved all 91 committed source/vendor files exactly and
had a clean diff. The [portability receipt](evidence/text-bidi-wrap-portability.json)
and [independent review](evidence/text-bidi-wrap-portability-review.json) cover
that metadata-only follow-up; 88 original frozen inputs remain byte-identical,
and the three differences are attributes and patch documentation.
