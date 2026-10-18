# Current radiance for visible samples

This is an unqualified, test-only continuation of the GPU visibility reconstruction
experiment, bounded to 128×128 output pixels. Ordinary rendering and the original
negative reconstruction arm are unchanged. It is not an adoption or performance
result.

The retained e4757e85 experiment supplied current coverage, but frames 26 and 28
each lacked radiance at 48 visible pixels (alpha-area error 12, RGB error 36).
Corrected data-only replay completed under the original geometry tolerance;
the failed exact-calibration checker and those deficits remain retained. The
replay receipt is external `gpu-visibility-reconstruction-data-001-c9orcba3/closeout.json`
(SHA-256 `31b8803738196d5fc3d46e7df375c10632dfc9104b687ea08104beeb99673bef`).

The new arm adds a GPU R32Uint mask with one bit per output sample. A bit is set
only for admitted current visibility with neither a current nor a history
radiance witness. It rerasterizes the current opaque geometry through the actual
ordinary PBR material and lighting source. World, normal, tangent and UV use
sample interpolation, with an explicit sample index. All five implicit material
texture reads precede the mask/owner discard; remaining IBL and shadow reads
use explicit-level operations. This preserves derivative uniformity without
claiming that sample-frequency shading has the same footprint as the ordinary
centroid route. The relevant WGSL contracts are [sample interpolation](https://www.w3.org/TR/2026/CRD-WGSL-20260921/#interpolation)
and [textureSample uniformity](https://www.w3.org/TR/2026/CRD-WGSL-20260921/#texturesample).

A separate cleared Depth32Float 4× attachment uses Less/write and the same draw
order, culling and clip-coordinate expression as visibility. Equal-depth
triangles therefore retain the first winner, including different UVs inside one
owner. Final resolve admits fallback color only for a requested sample with
finite RGBA, alpha one and exact agreement with the original visibility depth.
It never creates coverage. The old witness logic, history limits and tolerances
are unchanged. Opaque admission is checked again for each prepared draw.

Lighting receives an unjittered output-sized rectangle. Output cluster assignment
runs after low-resolution shading consumes its clusters; it can reuse storage
without overwriting a still-needed grid. The reference shades every currently
visible sample and evaluates every local light, independently of witness
availability and cluster selection. It shares the ordinary PBR evaluation,
not an emission-only substitute.

The pending native control retains the original 29 geometry poses, 49 analytic
strip coordinates, halo checks, raw ownership/depth and completion chronology.
It requires zero sample coverage/radiance deficits for the known emission fixture,
including the two former disocclusions. Additional controls compare requested
textured PBR samples with the unconditional current reference under two material
variants, a moving light and occlusion/removal; check coplanar first-winner UV
ordering; and preserve accepted output after refused preparation, cut and resize.
A 128×64 case crosses the x=64 cluster boundary with a short-range local light,
comparing the candidate with all-lights and light-off references. Raw records are
written before assertions so a failed native result remains diagnosable.

The added mask (4 B), HDR 4× target (32 B) and private depth (16 B) total another
52 B/output pixel: **236 B/output pixel** including the existing diagnostic
visibility/history planes. That is about 466.7 MiB at 1080p or 1.82 GiB at 4K,
before other renderer/driver allocations. This representation is unfit for
adoption. It adds two render passes and an output-light-grid dispatch when local
lights are clustered; all geometry and material texture fetches still execute
before selective discard. No cost saving is asserted. Compact visibility,
storage reduction, textured temporal quality, live material replacement,
full-motion appearance and shipping/device cost remain separate work.
