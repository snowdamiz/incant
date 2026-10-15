# Public fixed motion/history cooking

The public cooked aggregate is CPU-qualified and its existing fixed motion/history
programs ran on Metal with exact default/cooked raw parity. The original GPU
wrapper failed an extra stationary-precision assertion; that failure is retained.
A separately reviewed Python-only correction passed against the same saved bytes.

The public `--shaders cooked` aggregate now includes the existing complete
`cooked-temporal` feature. Five engine-owned programs cover motion, integer owner
identity, overlay depth, fixed history resolve and presentation copy. The guarded
jittered cluster compute program is included too. Together these add six families
and 33 backend artifacts to the qualified SSAO catalog: 23 families and 132
artifacts in the actual compiler-selected target output (573,004 artifact bytes).

The shader equations, source pins, native emitters, host ABI, default renderer
policy and explicit cold preparation are unchanged. Enabling the Cargo feature
does not enable motion/history in a scene. Ordinary and web paths keep WGSL;
native partial cooked configurations without the complete temporal feature keep
their typed refusal. The existing finite pipeline/cache preparation, submitted
owner/sequence checks, retained 1x route and MSAA restrictions remain in force.

Temporal bundles now retain a private program kind, so shared WGSL cannot relabel
motion as overlay depth. Before any bundle files are written, the writer checks
kind, exact source/producer hashes and one complete unique set of
backend/stage artifacts. Deterministic build receipts bind the ABI, source,
producer, embedded manifest and artifact bytes. The public exporter checks the
actual target features and all six new receipts, including the exact guarded
cluster source assembly. Project content cannot supply native shader bytes,
entries, paths or Cargo features.

The existing numerical GPU controls retain their analytical assertions: owner
visibility for opaque/MASK/skinned geometry, accepted/refused frame chronology,
on/off attachment equivalence, and fourteen history modes for disocclusion,
reactivity, coverage and alpha exclusion. An optional test-only trusted output
path retains raw arrays before assertions, with create-only files, a 64-record /
2 MiB cap, checked shapes and an explicit incomplete/complete summary. Owner and
direct-resolve controls retain their prior submission counts. Joined capture adds
four bounded readback submissions per mode; it opens no native window.

`tools/temporal_parity.py` requires exact cross-mode raw equality, with zero
tolerance. Owner identity, motion/depth, color/alpha and resolved history agree
in all 60 record pairs (259,600 raw bytes per mode). Both changed owners have
valid nonzero motion samples different from the preceding field. The existing
0.001 analytical history bounds pass. Equal no-op output is insufficient.

A separate four-frame retained scene adds one real point light, preserving the
three historical joined workloads. It submits jittered clustered lighting and
fixed history at full/half resolution, retaining the final pixels; timestamp-
capable adapters must report the actual light-assignment pass. Cross-mode output
must be exact and contain visible opaque geometry. Thus the temporal cluster
variant is exercised by dispatch, not only cold pipeline admission.

The producing native source is `5f10b67ae8b21ab11a5aee9f515e92138a868277`,
joined with main349. Rust 1.99.0, arm64 CI, locked/offline CPU qualification passed:

| Check | Actual result |
| --- | --- |
| Cooker | 48 tests |
| Default renderer | 186 passed, 57 device tests ignored |
| Full cooked aggregate + pipeline cache | 218 passed, 57 ignored |
| Partial cooked-output / cooked-materials | One refusal control each |
| Default and joint strict all-target Clippy | Passed |
| Native-window/profile/cooked fast-physics player | Compiled; not run |
| Actual target-output audit | 23 families, 132 artifacts |

Six separately bounded Metal children on Apple M5 Pro passed: owner, fourteen-mode
resolve and real-light joined history, once each in default and cooked modes.
The joined controls retain four actual light-assignment query intervals per mode;
these prove dispatch occurred and are not performance measurements. All owned
process groups were cleaned up and independently absent.

The enclosing original `gpu-001` result remains failed. Its external comparator
required stationary UV motion below `1e-6`, which the native owner test does not
specify. The shader subtracts raster-position-derived current UV from interpolated
previous clip coordinates. Saved stationary maxima were 0.00125122 and 0.00140572
internal pixels at full/half resolution, identical across modes. Source and bytes
do not establish the hardware cause or a portable precision bound. No renderer
equations, native assertions or CPU closed-form `1e-6` checks changed. The reviewed
Python successor reports residuals without asserting stationary zero, retains
finite/valid history and depth checks, and rejects missing history, nonfinite
outputs, frozen fields and either changed owner lacking a motion witness. Its
saved-byte replay passed; no native child was repeated. Final pure controls total
37 (26 catalog audit, 11 parity/refusal). Earlier protocol drafts were unexecuted.

[Portable evidence](evidence/cooked-temporal-export/manifest.json) retains the
CPU commands/logs, compiler features/generated artifacts, original GPU failure,
all raw arrays, source-backed correction/review and successful offline replay.
Run `python3 docs/spikes/evidence/cooked-temporal-export/verify.py`; optional
`--originals --binaries --git-source 84f52203 --source-root .` also checks retained
local originals/binaries and unchanged final code inputs. This is a bounded input
inventory, not a full transitive compiler snapshot. Native source/generated shader
bytes remain those of CPU003; the successor changes only the Python comparator
and its data-only controls before this documentation packet.

The explicitly separate `native-metalfx` preparation/normalization shims still
assemble WGSL at runtime. Ordinary/web paths and test-only depth/filter oracles
also retain their WGSL consumers. This is fixed-catalog export coverage, not
whole-renderer parser exclusion, appearance approval, a cache-hit result or the
Phase 1 shader-stall/performance gate.
