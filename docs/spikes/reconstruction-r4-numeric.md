# R4 first numerical GPU attempt: rejected

Root accepted the scoped source and CPU packet, then granted four frozen-binary
numerical tests, at most 62 queue submissions/maps, no images and no retries.
The first ordinary WGSL test failed on Apple M5 Pro / Metal. Execution stopped;
the owner test and both cooked-native tests did not run. This failed result is
preserved and is not an appearance experiment or an integration acceptance.

The immutable source is `72b1ba8e`; the default executable is
`103027db6cba5333af940fba57f29678b339680ea0a757ab39fff140c7155ebc`.
Supervisor handle 23666 terminated with exit 1. Child PID/group 32664 terminated
with exit 101 and was reaped. The release receipt at
`2026-10-14T01:13:30.981549Z` records no surviving owned process/group or images.
One attempt charged the full 19 submissions/maps. Its raw output and frozen loop
establish that eight submissions/maps ran, ending at mode 7's readback. Unused
budget does not authorize a retry.

Modes 0–6 completed their assertions. Mode 7's composite attachment failed:

| RGB | Independent ideal, before either store | Observed composite |
| --- | --- | --- |
| R | 0.27539892856072434 | 0.275146484375 |
| G | 0.22539892781566628 | 0.2252197265625 |
| B | 0.17539892707060822 | 0.17529296875 |

The fixture compared the second render attachment directly against unquantized
f64 history, although the composite reads the first RGBA16Float attachment.
That missing intermediate conversion is a reference-model defect. A simple
round-to-nearest-even replacement does **not** fully explain the observation:
two ideal RNE conversions predict green 0.2254638671875, two final half ULPs above
the retained observation. No universal Metal rounding rule is inferred.

The proposed correction propagates the independently allowed first-pass half
values through the monotone premultiplied composition, then applies the existing
final conversion/arithmetic bound. Neither bound is derived from observed GPU
values. The exact arithmetic, RNE counterexample, resulting endpoint intervals
and adjacent excluded half values are retained in
[quantization-diagnosis.json](evidence/reconstruction-r4-numeric-001/preparation/quantization-diagnosis.json).
This is a test-only proposal requiring source review and new CPU controls; no
production shader, visual threshold or fixture expectation has been changed in
this failure packet.

The [manifest](evidence/reconstruction-r4-numeric-001/manifest.json) binds the
raw stdout/stderr, grant text, fixed plan/runner, unchanged reviewed process
lifecycle helper, ten CPU output controls, debit, ledger and release. Original
files remain in `~/.codex/incant-evidence/reconstruction-r4-numeric-001`.
All historical rendered failures and all R4 device/appearance gates remain open.
