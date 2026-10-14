# Consistent shadow receiver fitting

Claude's 0072 sharpness review traced the offset viewport's large on/off shadow
difference to receiver fitting. Temporal rendering expanded the receiver frustum
before cascade-radius quantization, while its unjittered reference did not.
At 885 by 501 pixels this changed the first two cascades' texel sizes enough for
a thin alpha-tested grille to cast a different shadow. This predated the
sharpness candidate; it is separate from temporal reconstruction.

Shadow preparation now receives the actual render viewport and always applies
the same conservative half-pixel receiver guard to a local camera copy. Radius
quantization, caster depth fitting and existing filter margins are unchanged.
The scene camera is untouched: its guard still selects temporal light clustering
only when appropriate. No extra texture, pass or pipeline is introduced. Default
unjittered shadow fitting changes intentionally. The later producer-scoped
Claude visual comparison and root review are accepted; current-main
qualification is separately pending.

CPU regressions require bit-identical cascade matrices and sampling parameters
with temporal rendering off or in any of its eight phases, for perspective and
orthographic projections, ordinary/offset/fractional viewports and one pixel.
A negative control reproduces the original second-cascade quantization change.
The receiver coverage test now exercises the production viewport entry point.
Invalid viewport values refuse before fitting.

Local qualification passes 129 default and 143 cooked-temporal CPU tests, with
45 GPU tests ignored in each run. Strict cooked all-target Clippy, formatting,
generated conventions and diff checks pass. The first compile attempt missed two
test-helper call sites after the private signature change; that failure is kept
alongside the corrected source and complete rerun in the
[qualification archive](evidence/shadow-receiver-guard.json).
The later scoped Metal review is retained in
[the original report](../../handoffs/0076-shadow-guard-review/result.md) and
[the final root receipt](evidence/shadow-receiver-final-root.json). The original
strict comparison remains failed; a same-binary one-frame variation was
demonstrated with unknown cause/rate. Thin MASK shadow fidelity and temporal
quality remain open, including an unisolated one-code lit ramp pixel change.
The full-frame follow-up retained all sixteen temporal-on frame identities.
No current-main GPU, phone, performance or phase-gate claim follows.
