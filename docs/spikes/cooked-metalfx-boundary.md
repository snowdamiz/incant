# Cooked MetalFX boundary programs

The two fixed boundary programs now use the trusted backend-cooked selector.
CPU qualification and actual numerical Metal parity passed at producing source
`7aa8b9cedaea2dd9c13e6c6e11e82f091ab199b0`, composed with main351. This tests
pack/normalize behavior; it does not execute or qualify the vendor scaler.

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
WGSL.

## Recorded qualification

54 cooker, 191 default/native-MetalFX renderer and 224 full cooked/cache renderer
tests pass. Each renderer run leaves 62 manual GPU tests ignored. Three additional
controls exercise partial-output refusal, partial-temporal refusal and dedicated
complete-family admission. Both final feature guards and strict all-target lints
pass, alongside 39 Python controls and three harmless lifecycle controls. The
actual fast-physics public-player compiler output passes the 25-family /
144-artifact audit: 632,455 artifact bytes, 194 generated files. The player was
compiled only; its actual feature graph does not enable `native-metalfx`.

Two retained test executables ran the same existing boundary control on Apple M5
Pro / Metal: ordinary WGSL and the full cooked aggregate with explicit
`native-metalfx`. Each child passed all 17 cases with one submission and one map.
All five planes (packed color, depth, motion, reactive mask and normalized color)
match exactly across all 87,040 bytes per child. A separate saved-byte comparator
repeats the unchanged native sample oracle; four synthetic controls reject altered
bytes, oracle failures, changed route/workload/schema and mismatched source/size.
No tolerance was added. Raw outputs were retained before native assertions; both
child process groups were independently confirmed absent after managed cleanup.

The initial CPU attempt failed because `Fallback` was omitted from a native-feature
import. Restoring that import allowed the broader renderer tests to pass; strict
lint then rejected a test assertion on a constant. The final successor expresses
the same feature implication in an inline const. Both original failed receipts
and logs remain retained. Shader equations, ABI and fixture assertions did not
change during these repairs. Cumulative target growth, including the preceding
temporal work and all boundary attempts, was 649,192,884 bytes under the 4 GiB cap.

The [portable packet](evidence/cooked-metalfx-boundary/manifest.json) retains exact
sources, commands, CPU failures, generated artifacts, numerical raw buffers and
root reviews. Binaries remain outside Git with pinned hashes. Replay without a
compiler or device:

```sh
python3 docs/spikes/evidence/cooked-metalfx-boundary/verify.py
```

Optional `--originals --binaries --git-source 7aa8b9cedaea2dd9c13e6c6e11e82f091ab199b0
--source-root .` also checks local retained originals, external binaries and the
unchanged executable/tool inputs. Seven frozen markdown files are historical
context, including the pre-result version of this document and STATUS; later
documentation changes are not relabeled as compiled source.

No vendor scaler, player, window or capture ran for the device comparison. Native
vendor appearance failures remain unresolved. This is not a public shipping
export, Vulkan/Dx12 device, cache-hit, shader-stall, whole-renderer parser exclusion
or performance gate result. Ordinary/web WGSL and test-only oracles remain;
neither default quality policy nor vendor preference changed.
