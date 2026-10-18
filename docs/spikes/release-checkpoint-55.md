# Release checkpoint55

[PR408](https://github.com/snowdamiz/incant/pull/408) promoted selected main407
`c504668e036712bfc07f765140d772129f034cdf` after all 23 required contexts and all
eight workflows reported actual success on frozen candidate
`931226f957cba663be5718fef5831995220f2112`.

Release merge `2d96f46130dfa01e3a359541e7bbbfad84440304` has tree
`cca20800d0502ef6a935a7da768632a80c58d4c4`, exactly equal to selected main and the
tested candidate. Its merge body includes `Built-by: astra`, and published release
history is preserved. [Exact checks, integrated review and promotion](evidence/release-checkpoint-55.json).

This advances the checked baseline through the default-off presentation batching
and pose interpolation diagnostics, authored native-Wasm localization export
proof, HUD font-engine reuse, and retained compiler-profile/scheduler results.
The Windows desktop job completed in 81 minutes 44 seconds; this is one hosted
run, not a matched performance comparison. Failed renderer/performance experiments
remain failed. The nonrequired website deployment job was skipped; its build and
workflow succeeded. No required check was bypassed.

Main409 touch handling and main410 sprite-alpha collider authoring remain outside
this tested tree. No distribution was published, experimental renderer/performance
change adopted, or phase gate approved by this promotion.
