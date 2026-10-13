# Corrected temporal and native shader composition

This integration composes the independently reviewed cooked-temporal producer
fa28645c, native MetalFX producer 4bc810fe and temporal edge correction b0b4dcf0
(with its evidence 5bd27bb7). Source review, CPU tests and offline compilation
pass. Actual combined GPU execution and appearance acceptance remain pending.

The trusted native shader families explicitly include the shared motion vertex
and no-color depth fragment, fixed 64-byte copy uniform at binding 1, corrected
history resolve and overlay composite, and both MetalFX boundary programs. Pack
has four color targets; normalize has two. Source digests are updated for the
reviewed source changes, with independent IR and actual host-layout validation.
Native MetalFX remains an explicit opt-in and cooked selection never retries a
failed trusted program as runtime WGSL. Default and browser paths use the same
engine source; neither project data nor runtime shader source is newly admitted.

The MetalFX postbuffer now rerasterizes unjittered opaque/MASK depth only when
current overlays exist, after native normalization and before transparent draws.
It uses the same conditional prepared motion packet as the portable graph and
counts that actual pass in Models timestamps. The original jittered motion and
history depth remain unchanged. MetalFX packing/normalization math, conservative
current-coverage edge policy and native jitter sign remain unchanged.

Apple's [WWDC22 MetalFX explanation](https://developer.apple.com/videos/play/wwdc2022/10103/)
uses sample position(0.625,0.78) with API jitter(-0.125,-0.28), consistent with the
engine's image-shift convention (sample at pixel minus shift). This source
comparison is not visual evidence. The delegated stationary registration review
still checks the actual native result before any quality claim.

Portable temporal, native MetalFX and cooked execution require separately bound
same-source GPU tests and Claude visual review. Original rejected0072 quality,
source failures and prior producer evidence remain historical evidence, not
acceptance of this combined tree. Phone/performance and complete phase gates
remain open.

## Verification

The [retained manifest](evidence/temporal-metalfx-composition/manifest.json) binds
2,450 tested source hashes, the independent source review, 314 lossless records,
119 unique payloads and fourteen copied test binaries. The original source review
found that an unconditional motion-frame rule incorrectly required the depth-only
fragment stage to read the frame uniform. Generation 2 limits that rule to the
vertex stage; the fragment retains only material, texture and sampler resources. Independent review
accepted the resulting contracts and actual host binding layouts.

Twenty-nine cooker tests, 145 complete-feature CPU tests, 130 default CPU tests
and the explicit incomplete-cooking admission test pass. Complete-feature Clippy,
wasm32 and iOS compilation, runtime dependency checks, formatting and generated
conventions pass. GPU cases remain ignored: 99 in the complete configuration and
98 in the default configuration, including the separate persistent-cache probe.
Seventeen generated shader stages pass Metal compilation, metallib linking and
SPIR-V validation (54 compiler/version commands). HLSL is retained; DXIL is
unverified because DXC is unavailable. Compilation does not establish execution
on Apple phones, Vulkan or DX12.

All supervised child groups are reaped. Original failures remain: the first
cooker contract attempt, the first offline invocation with an unresolved
interpreter path, and two zero-match test filters. The final admission run
executes one real case. No failure or zero-match run counts as coverage.
