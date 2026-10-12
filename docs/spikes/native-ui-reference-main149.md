# UI numerical reference on main149

Claude0057's renderer correction is integrated with current main, preserving its
commits and source reviews. Zero-alpha fragments are discarded so uncovered
stored pixels remain unchanged. The independent reference keeps geometry, clips,
order and opacity exact while accounting for its documented filtering/sRGB
precision contract. A read-only accessor returns the same caret geometry used by
the draw path. The original Metal and mutation receipts remain source-bound.

Installed Mesa packages and CPU emulation do not identify the failed Linux CI
adapter. The source comment, spike and handoff result/JSON now qualify that
candidate diagnosis. Existing GPU tests log the actual selected wgpu adapter
under their existing --nocapture execution. The reference requires at least 8
sub-texel bits; Vulkan's permitted minimum 4 remains outside its contract.

Integrated CPU checks pass 135 cases across 15 result groups, with 89 GPU tests
intentionally ignored. All-target Clippy, wasm32 renderer and dependency guards
pass. Rustfmt initially requested wrapping only the new test diagnostic; that
failure is retained. After the mechanical correction, all 9 focused UI CPU cases
pass (5 GPU ignored), and all 7 final commands pass, including repeated Clippy,
wasm32/guards and formatting/conventions. Test scopes overlap. Actual-main149
integration changes only documentation relative to those compiled inputs.

No GPU, native-window, pixel, provider or device execution occurred here. Actual
Linux execution and adapter evidence remain pending the next frozen checkpoint;
all 23 contexts must succeed before release promotion. Original native-input
findings and their separate main149 fixes retain their own evidence, with the
Claude0058 visible follow-up reviewed separately. No complete IME/Game UI or phase
gate is claimed.

[Source identities, commands, original formatting failure and final checks](evidence/native-ui-reference-main149.json).

[Independent final integration review](evidence/native-ui-reference-main149/root-final-review.json) accepted the exact source and CPU checks. Actual Linux/full-checkpoint acceptance remains pending.
