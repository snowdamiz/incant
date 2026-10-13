# Output-space bloom correction

This private correction addresses the numerical DRS failure recorded by Claude
0070 on `fd625299d90fc9b0e359a7bf60baf03795fc9f4f`. The original GPU evidence
remains unchanged: 26/27 numeric gates passed; bloom centroid failed at 75% and
50% scale, with increased far-field radius and energy. No corrected GPU or
appearance result is claimed here.

The pyramid dimensions now derive from the physical output, independently of
internal scene resolution. Each ceil-half grid shares the full output's origin
and center convention. The first extraction maps those footprints into the
exact continuous scene span `output * scale`; the ceil-rounded attachment size
is only a texture bound. Later reductions and reconstruction retain their
original area and center-aligned bilinear rules. Final bloom lookup uses physical
fragment position divided by physical output extent. An offset/fractional
viewport remains an extraction mask, not a new pyramid origin.

The admitted scale is 1/4 through 1, so the first physical ceil-half footprint
spans at most two source texels on each axis. The existing fixed 3x3 overlap loop
therefore remains bounded. At full scale all filter coordinates and weights are
unchanged. Threshold, normalized level gain, premultiplied coverage, tone mapping,
viewport chrome, pixel-camera bypass and zero-intensity bypass are unchanged.

The 8192 dimension, 4-megapixel and 32-MiB pyramid budgets apply to physical output
at every scale. Lowering DRS cannot admit a larger pyramid or hide allocation
cost. The retained cache is keyed by that physical plan; changing only DRS reuses
the same textures, while old encoded commands retain their original handles.

Independent f64 CPU integration uses an analytic covered rectangle with fixed
physical area. It compares energy, centroid and RMS footprint at 100/75/50/25%
against the original varying-grid algorithm, and verifies the full-scale result
is unchanged. Additional controls cover fractional last source texels, odd grids,
offset viewports, fixed resource budgets and WGSL validation. These are numerical
CPU contracts; the original unchanged GPU gates and Claude appearance review must
be rerun on the eventual corrected source before accepting the DRS visual result.
