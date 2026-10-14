# R4 two-attachment numerical reference correction

The first R4 Metal attempt remains [rejected](reconstruction-r4-numeric.md).
This change affects only test code. It neither changes reconstruction shaders
nor relaxes a visual criterion. Attachment 0 retains its original independent
f64 comparison and per-pass allowance. Attachment 1 now models the binary16
intermediate that the composite actually loads.

Let `B(v)` be the existing `half_bound(v)`: one binary16 spacing at `v`, plus
`abs(v) * 2^-18` for arithmetic, with `B(0) = 0`. This correction does not change
`B`. It is a numerical test contract, not a claim of a universal Metal rounding
mode or a proof of all shader arithmetic error.

For independently reconstructed history `H`, the first attachment input set is
the representable binary16 values inside `[H-B(H), H+B(H)]`, clamped to the
sanitized channel domain (RGB 0–65504, alpha 0–1). Endpoints round **inward** to
representable values; round-to-nearest-even only locates those endpoints before
the necessary adjacent-value adjustment. No observed GPU value participates.
Numeric `-0` and `+0` have one canonical ordered representation.

For the fixture's independently sampled/sanitized overlay `O`, composition is
`O + (1-O.a) * under`. Because `O.a` lies in 0–1, this is nondecreasing in the
under value. Propagate the first-attachment endpoints through this expression
and clamp the result. The final allowed set is the representable half values
inside `[composite_lower-B(composite_upper), composite_upper+B(composite_upper)]`,
again clipped to the channel domain. `B` is nondecreasing over this nonnegative
domain, so its value at the upper endpoint bounds every intermediate point,
including binade crossings.

If admitted history alpha can be zero, RGB's lower endpoint includes the exact
clear value produced by `clean()`. With an all-zero overlay, an already stored
half value is preserved exactly (or its RGB clears at zero alpha); there is no
invented second error term. A fully opaque overlay is independent of history.
The helper models this fixture's uniform or affine bilinear overlay fields; it
does not claim arbitrary rasterization equivalence.

Five CPU controls exercise inward half endpoints and ties, subnormals, mixed
channels through two attachments, adjacent-outside refusal, sanitation and alpha
limits, monotone binade crossings, and signed-zero compatibility. The first
four passed in source002's full renderer runs: 166 default and 181 cooked/native
CPU tests, with 43/44 GPU tests ignored. Strict all-target Clippy also passed.
Root then identified signed zero in the helper; source003 adds the fifth control
and canonicalization. Source002 and its executed binary copies remain intact.
Source003 passed all five focused controls in both default and cooked/native
configurations and strict all-target Clippy. Root accepted the source003 signed-zero
correction before this refresh. The earlier full suites apply to the unchanged
fixture and interval math; the final focused runs cover the only later helper delta.
Fresh full test executables were compiled and copied for a separately authorized
numerical device run. Both CPU runners reaped every child and released the target.
The original failed device attempt remains unchanged and no retry has run.

All production sources and all 21 WGSL files remain equal to `72b1ba8e`.
The existing 19-case ordinary fixture changes only its composite comparison;
attachment0, viewport-clear, depth and evidence checks are unchanged. The owner
fixture and every submission/map count are unchanged. No new GPU execution is
authorized by this correction or its CPU checks.
