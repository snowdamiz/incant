# Third-party notices

## Khronos PBR Neutral tone mapper

`crates/incant_render/src/tone_map.wgsl` derives from the Khronos Group's
[PBR Neutral reference](https://github.com/KhronosGroup/ToneMapping/blob/180b1a7bddec33f73fe41712a2963cc3ad8e5547/PBR_Neutral/pbrNeutral.glsl).
Copyright 2024 The Khronos Group, Inc. Licensed under
[Apache License 2.0](licenses/Apache-2.0.txt).

The pinned revision's `.reuse/dep5` assigns Apache-2.0 to that code. Incant
ports it from GLSL to WGSL, adds fixed preview exposure, and rewrites comments.
The curve constants are unchanged. Retain this notice and the license when
distributing the shader or a binary that contains it. No upstream NOTICE file
exists at this revision. This notice covers the incorporated tone mapper; it is
not a complete inventory of dependency licenses.
