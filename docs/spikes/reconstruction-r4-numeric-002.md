# R4 attachment correction and bounded Metal numerics

The four authorized numerical fixtures passed on Apple M5 Pro Metal. This is
kernel and cooked-backend evidence, not an appearance, retained-scene, MetalFX
scaler, performance or release acceptance. Historical thin geometry, texture,
silhouette, DRS transition and C7/C9/P1 quality failures remain open.

Production remains exactly `72b1ba8ec351e86d66a90c401e0a75a530633616`.
The test-only correction is `cae2bffcdef061d3e1505b7f78a2d9888b582e61`:
four cfg-test paths, with all 21 WGSL files unchanged. The independent f64
first-attachment assertions and their error bounds are unchanged. The second
attachment propagates the independently admitted binary16 intermediate through
premultiplied composition, as specified in the
[mathematical correction](reconstruction-r4-attachment-reference.md).

The [first attempt](reconstruction-r4-numeric.md), including its mode7 failure,
is preserved. Its source, raw outputs, grant, debit and release were not edited.
An ideal two-pass round-to-nearest-even point model did not explain every
observed channel, and is not claimed as Metal's rounding behavior. The corrected
test accepts only representable values within independently derived intervals;
CPU controls reject adjacent outside values. Root's signed-zero finding is
covered by canonical numeric zero and an actual Rust clamp/half-bit regression.

Source002's complete renderer suites passed 166 default and 181 cooked/native
CPU tests, with 43/44 device tests ignored. The only subsequent source003 delta
canonicalizes zero and adds its control. Five focused controls passed in each
configuration against freshly compiled full test executables, followed by
strict all-target cooked/native Clippy. Both CPU generations and all four copied
executables are retained. Final fmt, diff and exact ignored-test listings pass.

The renewed GPU grant allowed these ordered invocations, with no retry:

| Configuration | Fixture | Queue submissions / maps | Extent |
| --- | --- | ---: | --- |
| Default WGSL | Ordinary modes 0–18 | 19 / 19 | 8×8 |
| Default WGSL | Owner cases 1–12 | 12 / 12 | 16×16 |
| Cooked Metal | Ordinary modes 0–18 | 19 / 19 | 8×8 |
| Cooked Metal | Owner cases 1–12 | 12 / 12 | 16×16 |

All four ran one exact test and passed, with complete ordered output markers and
the same actual adapter. These are submission counts, not render-pass counts;
owner case3 includes the current-only reference and cases7/8 include ordinary
fallback references within their existing encoders. The tests verify their
color/depth/record/evidence assertions, including complete relevant comparison
planes. Printed diagnostic values alone are not presented as full raw-texture
parity evidence.

Mode7 again reported the same intermediate and composite values as the failed
attempt: history RGB approximately `0.35058594`, composite RGB approximately
`[0.27514648, 0.22521973, 0.17529297]`. No shader was changed to produce this
pass. The actual intermediate is not substituted into the expected oracle.

Supervisor40916 completed with exit0. Children43390,43395,43409,43425 were reaped;
the release at `2026-10-14T01:38:54.033524+00:00` records no live owned processes,
groups or images. The global lock was independently reacquired and released.
There were 62 queue submissions, 62 maps, zero images and zero builds during the
GPU run. Concurrent authorized CPU builders make these timings unsuitable for
performance claims.

The [portable manifest](evidence/reconstruction-r4-attachment/manifest.json) binds
69 raw/source records and 58 deduplicated lossless payloads, 2,467 build inputs,
four executable identities, both CPU generations and the complete successful
GPU execution. A collector-development tuple/list comparison error is recorded;
the corrected JSON-shaped count comparison replays successfully. Replaying it
performs no build or device operation:

```sh
python3 docs/spikes/evidence/reconstruction-r4-attachment/collect.py \
  --verify-only --originals --local-binaries
```

The default executable SHA is
`8dc03c77ab409524d47439813965395b6d173900181fa14bc3a3e5000ea437f0`;
the cooked/native executable SHA is
`ca64e977853225f2f9973a96ead6a0fd297c83bdef66830955d8e11a4345a67b`.
Release SHA:
`114f9cf5af67176f8eeeed3250931496af5ed907d3e696e1bbac2b719b4691eb`.
Independent final packet review and any new rendered-quality protocol remain
separate. Claude Opus 5.5 with explicitly verified Max owns pixel review.
