# Third-party notices

## Localization dependencies

Incant uses unmodified ICU4X 2.3 locale, decimal, date/calendar and plural
libraries and compiled Unicode/CLDR data. Direct crate versions are 2.3.0;
Cargo.lock records every transitive library/data version and registry checksum.
These packages, `fixed_decimal`, and `writeable`
use [Unicode License V3](licenses/Unicode-3.0.txt). The calendar dependency
`calendrical_calculations` uses [Apache License 2.0](licenses/Apache-2.0.txt).
Corresponding sources are
available in their versioned crates.io packages or `cargo vendor --locked`.
Incant's bounded MessageFormat parser and pseudo-localization are original code;
they are not a complete ICU MessageFormat implementation.

Translator exchange uses unmodified [quick-xml 0.42.0](https://crates.io/crates/quick-xml/0.42.0),
copyright (c) 2016 Johann Tuffe, under the [MIT license](licenses/quick-xml-MIT.txt).
The pinned source and checksum are recorded in Cargo.lock.

## Audio dependencies

Incant uses unmodified [Kira 0.12.5](https://crates.io/crates/kira/0.12.5),
licensed under MIT OR Apache-2.0, and
[Symphonia 0.6.0](https://crates.io/crates/symphonia/0.6.0) with its locked
0.6.1 decoding/format components, licensed under
[Mozilla Public License 2.0](licenses/MPL-2.0.txt). Their exact versions and
registry checksums are retained in Cargo.lock. Corresponding unmodified sources
are available from those versioned crates.io packages (including each
`symphonia-*` package listed in the lockfile), or through `cargo vendor --locked`.
The synthetic WAV/Vorbis test tones are original mathematical signals; no music
recording was incorporated. The complete distribution dependency inventory
remains a release requirement; this notice covers the new direct audio libraries.

## Khronos PBR Neutral tone mapper (derived preview curve)

`crates/incant_render/src/tone_map.wgsl` derives from the Khronos Group's
[PBR Neutral reference](https://github.com/KhronosGroup/ToneMapping/blob/180b1a7bddec33f73fe41712a2963cc3ad8e5547/PBR_Neutral/pbrNeutral.glsl).
Copyright 2024 The Khronos Group, Inc. Licensed under
[Apache License 2.0](licenses/Apache-2.0.txt).

The pinned revision's `.reuse/dep5` assigns Apache-2.0 to that code. Incant
modified it: ported from GLSL to WGSL, removed the 0.04 Fresnel toe offset,
moved the compression threshold from 0.76 to 0.8, added fixed preview exposure,
and rewrote the comments. The shoulder formula and desaturation constant are
kept. The result is an Incant derived preview curve, not a conforming Khronos
PBR Neutral implementation. Retain this notice and the license when
distributing the shader or a binary that contains it. No upstream NOTICE file
exists at this revision. This notice covers the incorporated tone mapper; it is
not a complete inventory of dependency licenses.
