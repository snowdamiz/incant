# Root-motion primitive on main150

The reviewed rigid local-track sampler is integrated with actual main150
`58cca708545eb494868e18469fd4de98f47eb2da`. Its seven accepted source/test inputs
remain byte-identical. The tested head was `af412bc72df7842401e23c3a8d0564361b0c6edc`;
merging actual main150 produced the same tree. All frozen build inputs remain
unchanged after checks and the documentation-only Claude0058 packet retention.

All 128 assets/runtime CPU cases across 19 test groups pass, with no failures or
ignored tests. All-target/all-feature Clippy, wasm32 visual-reader compilation,
the authoring-free dependency guard, formatting, conventions and diff checks pass.
The seven terminal commands and compressed/raw log identities are retained in
[the integration inventory](evidence/native-root-motion-main150.json). The earlier
[producer contract](native-root-motion-core.md) and its independent numerical and
allocation controls remain unchanged.

This primitive does not select an authored root, own a gameplay clock, strip a
pose, move a body, integrate graph blends or add save/SDK control. No rendering,
shipping performance, device or phase gate is claimed. The separate accepted
[Claude0058 packet](native-text-0058-acceptance.md) is retained as historical native
input evidence; it changes no engine source.

Checkpoint12 is separately frozen at main150, before this sampler. Its
[manifest](evidence/release-checkpoint12/manifest.json) and
[eight initial push workflow identities](evidence/release-checkpoint12/push-runs.json)
record pending full validation, not a release promotion.
