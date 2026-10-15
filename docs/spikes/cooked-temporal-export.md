# Public fixed motion/history cooking

Source implementation is ready for qualification; native compilation, actual
compiler-output audit and matched Metal execution have not run for this slice.

The public `--shaders cooked` aggregate now includes the existing complete
`cooked-temporal` feature. Five engine-owned programs cover motion, integer owner
identity, overlay depth, fixed history resolve and presentation copy. The guarded
jittered cluster compute program is included too. Together these add six families
and 33 backend artifacts to the qualified SSAO catalog: 23 families and 132
artifacts expected from a complete compiler-selected target output.

The shader equations, source pins, native emitters, host ABI, default renderer
policy and explicit cold preparation are unchanged. Enabling the Cargo feature
does not enable motion/history in a scene. Ordinary and web paths keep WGSL;
native partial cooked configurations without the complete temporal feature keep
their typed refusal. The existing finite pipeline/cache preparation, submitted
owner/sequence checks, retained 1x route and MSAA restrictions remain in force.

Temporal bundles now retain a private program kind, so shared WGSL cannot relabel
motion as overlay depth. Before any bundle files are written, the writer checks
kind, exact source hash, nonzero producer identity and one complete unique set of
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
2 MiB cap, checked shapes and an explicit incomplete/complete summary. No extra
GPU work or native window is introduced by the recorder.

`tools/temporal_parity.py` compares separately compiled default and cooked results.
Cross-mode raw equality starts at zero tolerance. Owner identity, alpha/coverage,
motion/depth and resolved history must agree; nonzero moved/deformed motion and
the existing 0.001 analytical history bounds must also pass. Equal no-op output
is insufficient. Data-only controls reject incomplete/wrong routes, changed
membership/shape/hash, raw differences, lost owner coverage, zero motion and
history fallback. Actual device acceptance remains pending, not inferred from
these synthetic parser/refusal controls.

The explicitly separate `native-metalfx` preparation/normalization shims still
assemble WGSL at runtime. Ordinary/web paths and test-only depth/filter oracles
also retain their WGSL consumers. This is fixed-catalog export coverage, not
whole-renderer parser exclusion, appearance approval, a cache-hit result or the
Phase 1 shader-stall/performance gate.
