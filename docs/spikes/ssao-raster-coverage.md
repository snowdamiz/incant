# Fractional SSAO raster coverage repair

The exact-main239 Windows diagnostic reproduced checkpoint26's failure on
Microsoft Basic Render Driver (Dx12, driver 10.0.26100.33438). With a 17/255 clear,
the failing pixel remained 17, proving that it had not been rasterized. Both
perspective and orthographic cases had 29 horizontal and 16 vertical mismatches
inside the shader rectangle. Full-target rasterization eliminated every mismatch
in both stages without changing depth, uniforms, shaders or scalar intervals.

Production SSAO sampling and both filters now use the full target viewport and
an outward-rounded integer scissor over the original fractional rectangle. The
shader continues to enforce that original rectangle. This covers accepted pixel
centers consistently without shading the whole attachment. Empty rounded regions
retain the white clear. Geometry's depth/normal viewport and camera projection
are unchanged, as are shader bytes, attachment sizes and default-off behavior.

The existing strict filter test now invokes the same raster helper as production
and keeps every original depth and R8 interval assertion. The observation-only
diagnostic retains its original three modes so the previous behavior remains
reproducible. The fixed Windows workflow runs the exact strict test first and
requires both projections and an actual Dx12 adapter. Its observation step cannot
turn a failed strict test into a passing job.

Independent source review found no issues. Local qualification passes 22 CPU
cases (three GPU cases ignored), strict library/test Clippy, WebAssembly library
compilation, formatting, conventions and whitespace. The exact workflow passed
actionlint and twelve policy cases before composition. Raw diagnostic observations,
source review and local checks are retained in the [evidence record](evidence/ssao-raster-coverage.json).
The repaired strict Dx12 test is still pending; observation success alone does
not pass the failed release check or approve any phase or performance gate.

The final independent audit rehashed all 32 raw/archive records and 23 lossless
payloads, all five source/workflow identities and the completed check logs. Its
[acceptance receipt](evidence/ssao-raster-coverage/final-review.json) retains the
strict Dx12 execution limitation. This composition starts at actual main240;
no additional Rust source join or duplicate compiler run was needed.
