# Accepted camera and framing identity

The private temporal owner consumer foundation now exposes a conservative camera
identity against the last submitted frame. It compares the unjittered projection
and view, actual eye and forward values, shader eye, guard and complete source and
output framing. Jitter phases can advance without invalidating this identity.
A dynamic-resolution transition preserves compatible output history but disables
static persistence because the source viewport can change shadow fitting.

The candidate signature becomes accepted only after submission. Abandoned or
failed preparation cannot change it; resets, bypass and plain motion never claim
static temporal history. This flag proves camera/framing equality only: lights,
geometry and renderer effects still require separate checks before any unclipped
history reuse. No shader or consumer has been changed by this step.

Three new CPU controls cover jitter advancement, DRS and abandoned preparation,
actual shading-eye changes with unchanged matrices, cuts, shadow inputs and the
existing plain-motion/temporal topology refusal. The default library suite passes
146 cases (41 GPU cases ignored); cooked temporal plus native MetalFX passes 161
(42 GPU cases ignored). Strict library/test Clippy, formatting and diff checks
pass. Independent review accepted the exact source and execution receipts.
[Portable evidence](evidence/temporal-camera-history.json) retains the logs and
source copies. No GPU, appearance, device or performance claim is made, and the
private upscaler ancestry has not been accepted for current main.
