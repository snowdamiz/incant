# Cooked MetalFX boundary programs

Source implementation is prepared; native compilation, actual target-output audit
and default/cooked device parity have not run for this slice.

`cooked-metalfx-boundary` covers the two engine-owned pack and normalize programs.
It requires the complete `cooked-temporal` feature and is included in the public
`cooked-native-shaders` aggregate. It does not enable `native-metalfx`, change the
default spatial preference, or change the optional vendor scaler's equations or
quality policy. The full aggregate expects 25 fixed families / 144 artifacts.

The host cooker pins the exact unchanged `common + alpha + prepare/normalize`
source, private program kind and exact producer digest. Both programs have a
96-byte frame uniform (six float vec4 fields), two unfilterable float textures and
one depth texture, all fragment-only in group 0. Pack writes HDR color, depth,
two-component motion and reactivity; normalize writes HDR. Contract validation
checks entry/stage use, resource spaces/types, frame names/offsets and every MRT
location/type before emitting six backend/stage artifacts per program. Bundle
identity and complete unique stage membership are checked before output writes.
No project can supply shader bytes, native entries, source paths or features.

The actual cold runtime constructor selects embedded native modules for complete
native cooked builds and WGSL for ordinary/web builds. Partial cooked-output or
cooked-temporal configurations retain the explicit `CookedShaders` refusal;
there is no mixed native WGSL fallback. Host layout/target declarations are
checked against the pinned ABI. Both PSOs receive the renderer-owned optional
pipeline cache, and the cold-preparation completion hook runs only after the
device validation scope closes successfully. This makes no Metal cache-hit claim.
Frame preparation, target/callback ownership and native scaler prewarm are unchanged.

Tests cover deterministic artifacts and native binding maps, source/equation and
valid-IR ABI mutations, nonzero tampered producer, wrong kind, missing/duplicate
stages, host selectors, descriptor corruption and unsupported backends. The public
auditor requires both actual compiler-selected families plus the dedicated feature
and verifies source, producer, manifest and artifact bytes. Default selection stays
off. The existing 17-case numerical boundary fixture keeps its exact oracle for
coverage, nonfinite values, depth and jittered motion, with its optional retained
report identifying the actual cooked route. Its test-only depth oracle remains
WGSL. No vendor appearance, gameplay shader-stall, whole-renderer parser exclusion,
public shipping export or performance gate is established by this source change.
