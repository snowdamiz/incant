# Exact clustered light membership

Phase 1 performance increment, 2026-10-09. The 64-light list overflow path in
[authored lighting](clustered-lighting.md) evaluated every local light for a
fragment. Dense scenes therefore lost the benefit of spatial selection.

Each cluster now stores a bit per supported local light, in 32-bit words. The
compute pass ORs all intersections into a bounded 128-word workgroup array and
writes every used word, including zeros. No intersection is discarded. Fragment
shading visits ascending words and their least set bits, producing deterministic
summation without sorting or an overflow fallback. Directional lights stay
separate; geometric bounds, attenuation, BRDF and display transform are unchanged.
The bit scan uses WGSL's [firstTrailingBit](https://www.w3.org/TR/WGSL/#firstTrailingBit-builtin)
only on nonzero words.

Buffers are cached by tile dimensions and mask word count. Replaced resources
remain retained by encoded commands. Allocations are bounded to 131,072 clusters
and 128 words (64 MiB plus counts), and checked against actual device limits.
At 1920×1080 with 4096 lights, masks occupy 6,266,880 bytes plus 48,960 count
bytes. Smaller light counts use fewer words. Directional-only and explicit
reference frames skip cluster compute and use small unused binding buffers.

`RenderScene::with_local_light_selection(LocalLightSelection::All)` is a read-only
diagnostic that evaluates all local lights without constructing or reading the
grid. It does not mutate the project or alter the command bus. Oracle tests now
use this explicit path; adding zero-energy lights no longer forces a fallback.
The oracle isolates culling correctness and shares the existing shading equations.
Separate physical light tests validate inverse-square, cone and range behavior.

## Verification

135 Rust behavior tests, 27 real GPU checks, 283 UI tests, workspace Clippy,
generated-file checks, SDK typechecking and five Python tool tests pass.
GPU readback verifies all 24 depth slices, every mask bit through light 4096,
31/32/33, 63/64/65, 127/128/129 and 4095/4096 boundaries, tail clearing, word-count
changes, empty clusters, viewport offsets and queued resize lifetimes. Actual
4096 mixed point/spot-light frames match the All path byte for byte at odd and
even sizes. Existing spatial, range, spot, material and retained-scene checks pass.

Thirty-seven of thirty-nine prior named captures are byte-identical. The two
96-light captures differ by one channel level in one channel of one pixel;
the new clustered/reference pair remains identical. Those reconstructed fixtures
create fresh ULIDs, so cross-run summation order can differ. Claude's independent
pixel review in `dab3a96` approved appearance within this scope. A follow-up
now gives the final local bit its own visible-surface capture: 4095 off-frustum
lights precede the only visible local light, with a directional prefix. The
clustered, All and single-local-light reference images match exactly. Its
focused pixel review passed in Claude commit `1249e4b`. All 53 current PNGs
are recorded by hash in the evidence ledger. The custom-protocol native release
build also passes.

## Paired frame timing

The committed read-only `frame_benchmark` example ran three paired trials against
baseline `8b94eff` and the mask implementation, alternating variant order. Every
pair read the same cooked project and its document hash stayed unchanged. Each
invocation measured 30 frames after five warmups at 1920×1080. No concurrent local
test/build workload ran during these trials; the native editor was closed.

The fixture is a 3,968-triangle sphere, black environment and 0.7-meter-spaced
point-light lattice prefix with 2-meter ranges. Values below are the median of
three trial medians on Apple M5 Pro, in milliseconds:

| Local lights | Previous lists | Exact masks |
| --- | ---: | ---: |
| 0 | 1.396 | 1.408 |
| 64 | 1.397 | 1.407 |
| 256 | 1.406 | 1.394 |
| 1024 | 5.198 | 2.708 |
| 4096 | 34.226 | 6.438 |

The dense case improves about 5.3× in this paired run. Its previous medians varied
27.63–34.80 ms; the mask medians were 6.42–6.47 ms. Smaller cases remain near the
measurement floor. These wall times include CPU encoding, queue submission and
GPU completion waits; they exclude loading, readback, PNG, presentation and
simulation. They are not GPU timestamps or a Core Sample performance result.
The raw samples, project hashes and per-trial p95 values are in the
[evidence ledger](evidence/light-masks-2026-10-09.json).

Cluster construction still tests every local light against every cluster.
Hierarchical binning, production render graph, shadows, mobile tiers and named
performance gates remain open. No new native UI review is claimed for this
renderer-only change; the preceding authored-light Inspector review still applies.
