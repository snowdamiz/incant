# Sampled Core locale rendering on Metal

Four newly packaged shipping players from source
`50b5408b1d6be66dbb1dcbca501d4380add9d656` completed five CPU routes and the same
five routes with 37 selected offscreen Metal frames on an Apple M5 Pro. Entire
gameplay/HUD reports and PCM match each corresponding CPU baseline. Independent
review of 13 original PNGs accepts the sampled locale layouts, including the
previously missing final wins and narrow pseudo result/prompt wrapping.

The [one-session package proof](native-player-capture-packages.md) records the
fresh shared-unit origins, independent scene-bound players and deletion of
authoring/build storage before standalone startup. This documentation adds no
native source changes or new build claim. The producing tree remains
`f0c5fcf5abe7d53c533629c0a7d1f1be92aa5147`; later integration is distinct.

| Route | Host steps / samples | Logical / physical extent | Reviewed originals |
| --- | --- | --- | --- |
| Core English, 10,000 script entities | 2,156 / 5 | 640×480 / 1280×960 | [Winning HUD](evidence/core-locale-sampled-captures/images/Core-en/frames/frame-2156.png) |
| Simplified Chinese | 3,056 / 10 | 640×480 / 1280×960 | [Loss](evidence/core-locale-sampled-captures/images/zh-Hans/frames/frame-0900.png), [restart](evidence/core-locale-sampled-captures/images/zh-Hans/frames/frame-0901.png), [win](evidence/core-locale-sampled-captures/images/zh-Hans/frames/frame-3056.png) |
| Corrected Arabic | 3,056 / 10 | 640×480 / 1280×960 | [Damaged health](evidence/core-locale-sampled-captures/images/ar/frames/frame-0550.png), [loss](evidence/core-locale-sampled-captures/images/ar/frames/frame-0900.png), [restart](evidence/core-locale-sampled-captures/images/ar/frames/frame-0901.png), [win](evidence/core-locale-sampled-captures/images/ar/frames/frame-3056.png) |
| Expanded pseudo-English | 3,056 / 10 | 640×480 / 1280×960 | [Loss](evidence/core-locale-sampled-captures/images/en-XA/frames/frame-0900.png), [restart](evidence/core-locale-sampled-captures/images/en-XA/frames/frame-0901.png), [win](evidence/core-locale-sampled-captures/images/en-XA/frames/frame-3056.png) |
| Narrow pseudo-English, same player | 3,056 / 2 | 320×480 / 640×960 | [Wrapped loss/prompt](evidence/core-locale-sampled-captures/images/en-XA-narrow/frames/frame-0900.png), [wrapped win/prompt](evidence/core-locale-sampled-captures/images/en-XA-narrow/frames/frame-3056.png) |

The localized route captures host step 900 in generation 1 at simulation tick
900, before restart. Step 901 is generation 2, simulation tick 1; the final
step 3,056 is generation 2, simulation tick 2,156. Core English has no restart.
Every sample retains its generation, world epoch, typed render/UI input extent,
PCM-prefix hash, encoded PNG hash and decoded RGBA hash. The 13 checked-in PNGs
are byte-identical originals, totaling 9,630,676 bytes.

The reviewed Arabic damage frame shows the empty pip on the left and filled
pips toward the right. Joined loss/restart/win text stays contained. CJK glyphs
are complete and fit; expanded pseudo text preserves its brackets and accents.
Narrow pseudo loss/win text and prompts wrap without observed clipping or HUD
overlap. Restart removes the prior result and resets sample count/health. These
are the independent review's sampled observations, not conclusions inferred
from text-source strings or numerical widget counts.

## Numerical and lifecycle evidence

Five serial managed children completed successfully in 44.829 seconds. Their
full reports equal the five CPU baselines after removing only `frame_capture`;
the full PCM and every selected prefix also match. PNG CRC, dimensions,
decompression and decoded RGBA hashes pass for all 37 originals. Capture
manifests are complete and correspond to individual frame receipts. The
wrapper and all five owned process groups were independently absent afterward.
All packages retain their exact executable, scene and script bindings from
the preceding package proof.

The frozen envelope was 120 seconds per child, 650 seconds overall, 512 MiB
combined CPU/capture output, 1 MiB per log stream, 16 MiB aggregate logs and
a 6 GiB free-space floor, with no retries. Actual combined output was
213,606,230 bytes and logs 56,087 bytes. Cooperative checks are not kernel
quotas. The ordinary renderer used WGSL, 1× offscreen output and the actual
packaged CPU spatial world/UI; no substitute scene or second simulation was
used to produce a selected frame.

The compact packet retains the exact proposal, supervisor, comparator/negative
controls, raw five reports, all frame receipts/manifests, closeout and independent
pixel review as data. Full packages, PCM and the other 24 original PNGs remain
external and hash-bound. It depends on the preceding package packet for full
source/build provenance and CPU baseline reports.

```sh
python3 docs/spikes/evidence/core-locale-sampled-captures/verify.py
python3 docs/spikes/evidence/core-locale-sampled-captures/verify.py \
  --originals --binaries --git-source 50b5408b
```

Default replay checks all saved report/receipt relationships and decodes the
13 checked-in PNGs. `--originals` additionally checks all 37 retained original
images and ten complete PCM files. The other flags replay the preceding
package packet's executable/data and Git-source identities. Archived programs
are retained as evidence bytes and are never executed by this replay.

## Scope and earlier attempts

This adds actual sampled winning/RTL/CJK/pseudo/narrow pixels to the combined
Core sample evidence. It does not turn earlier [Arabic](core-arabic-corrected-window.md),
[CJK/pseudo](core-locale-native-window.md) focus-interrupted/locked-window
attempts into complete native-window appearance proofs. Their original outcomes
and the package lane's build/setup refusals remain retained.

Thirteen images were reviewed; all 37 were numerically audited. This establishes
neither continuous motion nor native-window presentation/pacing, temporal
history/reconstruction quality, audible quality, translation quality, 60 FPS,
shader-stall timing or phase approval. Existing polygon/edge aliasing remains
outside the locale acceptance. No desktop window, CUA or screen-recording helper
was launched by this sampled capture sequence.
