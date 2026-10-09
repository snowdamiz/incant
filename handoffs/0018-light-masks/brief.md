# Cluster membership masks: verify appearance is preserved

Claude Opus 5.5 through ACP owns pixel review. Astra implemented a performance
change to the already reviewed authored-light renderer. Review the actual PNGs
in artifacts/mask-initial, and compare artifacts/mask-prior-comparison.json with
the prior approved images in artifacts/cluster-final. These files will be copied
into this isolated worktree before your review; wait for them if necessary.

The former 64-entry cluster list/sort and all-lights overflow fallback are now
exact per-cluster bit masks for all 4096 supported local lights. The compute
shader's geometric intersection, light parameters, BRDF, range attenuation,
environment and tone-map arithmetic are unchanged. Ascending mask words/bits
preserve stable light summation order. A new explicit diagnostic All mode skips
cluster construction and selection entirely, serving as the independent culling
oracle; it shares shading arithmetic. No authored project or UI behavior changed.

Five GPU unit checks and nineteen model GPU checks pass. They cover 31/32/33,
63/64/65, 127/128/129 and 4095/4096 mask boundaries, all 24 depth slices, stale
clearing, retained queued resize commands, all-light pixel equality and physical
light behavior. Full workspace validation and controlled timing are in progress.
The preliminary 4096-light 1080p median is 6.42 ms versus the previous 28.05 ms;
these fenced CPU+GPU measurements are not a game or mobile performance gate.

Review all newly generated light captures. The full-mask pairs use 4096 spatially
distinct point/spot lights, finite ranges and a directional light at odd/even
sizes; each pair is byte-identical to All mode. Existing spatial, depth, range,
spot and material captures remain included. Of 39 names shared with the previous
run, 37 PNGs are exactly identical. The other two (spatial-overflow-96-640x480 and
its oracle) differ by one 8-bit channel level in one channel of one pixel. Both
new images remain identical to each other. The independently reconstructed test
scene creates fresh ULIDs, so changed accumulation order is a possible source;
this is a numerical comparison result, not a claimed visual verdict. Do not
assume a missing light from the file hash alone.

Inspect for visible missing lights, tile/depth seams, range/cone discontinuities,
color or material changes, or discrepancies between the supplied oracle pairs.
No UI, appearance tuning, shader editing or art direction is requested. Do not
approve the complete lighting system or Phase 1; shadows, authored cameras,
post-processing, mobile tiers and the full render graph remain open. This work
changes light selection only, so no new native UI review is claimed or required
by this packet. Identify any additional evidence needed for a concrete defect.

Use this worktree's own target directory if a diagnostic rebuild is needed;
never share CARGO_TARGET_DIR. Computer use is allowed if needed, but native
capture must use CUA. Do not access credentials or publish images/accounts.
Write result.md with model/transport, exact images reviewed, findings and scoped
verdict. Commit only your report/packet with final trailer Built-by: claude.
