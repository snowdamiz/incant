# Shadow receiver fitting on main230

This composition retains the accepted viewport guard from `f636a2d5`. Six of seven changed Rust files are producer-exact; the model-draw entry keeps main230 SSAO routing and changes only its shadow preparation argument to the actual viewport. All previous WGSL, manifests and locks remain unchanged. Source review `e254152d` is accepted.

Thirteen current checks pass: 171 default and 203 overlapping cooked/observer CPU cases, strict Clippy, native-window compile, wasm and dependency checks. Each configuration ignores 107 GPU tests. All 33 existing cooked native payloads are byte-identical. No current GPU execution or appearance result is inferred.

The original source and Claude evidence remain lossless: 13 source-qualification records, 134 handoff files and 85 retained/original visual records. Root review `028e8d4b` accepted that scoped Metal result. The original strict comparison remains failed; same-binary one-frame variation has unknown cause/rate. Thin MASK fidelity, temporal quality and the unisolated one-code lit ramp pixel remain explicit limits. Sixteen temporal-on frames matched in the later full-frame follow-up.

Current integration final review is accepted (`c21d16ea`). Main231 constructor
ancestry is joined with explicit old/new frozen hashes. The imported private
Rapier modules and observer alias are guarded by features absent from both
actual renderer builds. Renderer source, manifests and lock remain unchanged;
this source-relevance qualification is independently accepted (`ec653030`).
Publication follows the required fast check. See [manifest](evidence/shadow-main230.json) and [original report](../../handoffs/0076-shadow-guard-review/result.md). No phone, performance or phase gate is approved.

The later main233 join adds the independently reviewed one-case HierarchyMenu test deadline and its documentation only. The read-only qualification replays the accepted main231 and original assertions, verifies unchanged renderer, physics, tools, manifests and lock, and makes no new build or GPU claim.
