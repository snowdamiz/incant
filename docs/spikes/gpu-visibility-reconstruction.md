# Visibility-admitted reconstruction experiment

This is an opt-in, test-only correctness prototype under `experimental-taau`.
The ordinary renderer and the earlier color-TAAU candidate remain unchanged.
The new reconstruction source has **not been compiled or run**. No appearance,
frame-time, memory-budget or shipping adoption gate has passed.

## Observed owner correction

Producer `b2f6c96ee654a0c6ef468b1c060266d2bb9dadc1`, copied executable
SHA-256 `a9d0534c2abd8f6581de17d132b5299db61863c1230f0db331e48a7e6052b5b3`,
ran one headless device-buffer experiment. Twelve CPU controls and strict renderer
lint passed before the experiment. No surface, native window, OS screen access,
computer-use tool, screen recording or GPU debugger participated.

Both arms used the same four RGBA16Uint attachments, depth state, geometry,
forward draw order, upload bytes and original geometric oracle. The baseline
reproduced exactly 48 wrong owner samples: x=6…53, y=44, sample 3. The second arm
kept owner low/high halves in XY and wrote the exact existing sample-position-X
bits in ZW. It had zero owner/depth/correspondence or packing mismatches. Motion,
depth and raw motion lanes remained byte-identical; exactly those 48 owner values
changed. All 4,193 covered companion values matched the independent position
plane; uncovered RGBA stayed zero. Actual HAL-generated MSL retains this single
expression change and unchanged output locations. This establishes a useful
local discriminator; it does **not** identify a driver/compiler cause or establish
general correctness of the companion method.

The baseline remains a failed visibility result. Capture and data-process exit
success are recorded separately from each arm's acceptance. Native session 84608
and data session 51706 exited zero; all four observed wrapper/child identities and
groups were absent at closeout. Root independently replayed all 89 capture records
and all 16,384 samples. The immutable native packet remains
`gpu-color-visibility-dependency-gpu-001-lr7_lcg8` under local resume evidence.

The portable [manifest](evidence/visibility-samplex/manifest.json) binds 114 inert
records, including the original negative baseline, numeric buffers, actual MSL,
upload receipts, commands and terminal records. The 256,143-byte gzip contains no
native executable. Replay reads data and repository verifier code only:

```sh
python3 -B docs/spikes/evidence/visibility-samplex/replay.py
python3 -B docs/spikes/evidence/visibility-samplex/test_data.py
```

External executable/source-freeze identities remain hash-bound records; portable
replay does not recreate the device run or independently inspect that executable.
Upload receipts describe the CPU bytes passed to `queue.write_buffer`, not a GPU
readback of the instance buffer. The companion changes the shader; observer effects
and applicability outside this fixture remain open.

## Next concrete mechanism

The new arm submits low-resolution color/owner/depth, output-resolution 4x
visibility, then reconstruction and display in one GPU command buffer. The
visibility pass uses the passing companion representation without XOR packing.
No mapped buffer, CPU image classification or pixel admission occurs between
these passes. Existing retained preparation/acceptance owns history publication.

Current visible samples determine coverage. Clear samples cannot revive a
departed surface. A visible sample obtains current radiance only from matching
low-resolution owner/depth taps. Missing radiance may use previous color only
with a matching stable owner, valid object/camera correspondence, compatible
expected previous depth, finite values and the existing eight-phase age bound.
Visibility motion is already unjittered; the new resolve does not remove jitter
again. Current low-resolution lookup still applies current input jitter once.
The changed bit is retained in visibility, while matching uses the stable owner
within the accepted motion owner/epoch; moving objects are not blanket-refused.

Reusable output history is tagged only when all covered samples have radiance
and belong to one owner. Mixed-owner pixels may compose current color but cannot
later identify that composite as one surface. Never-observed color remains
missing rather than being invented from coverage. Existing analytic strip
slivers missed by all four samples remain reported. This is a conservative
prototype, not a complete solution for arbitrary moving radiance or self-occlusion.

The bounded fixture exercises constant white/red radiance, full jitter warmup,
object translation and reversal, camera movement, foreground occlusion/depth,
offscreen motion, cut, removal, resize and failed-preparation nonpublication.
Checks use independent projected geometry, exact raw owner/correspondence,
sample and analytic alpha-area error, RGB error, and fresh-owner pixel equality.
Raw outputs are saved before acceptance assertions so a failure remains inspectable.

## Cost and validation boundary

The deliberately unreduced visibility pass uses 144 bytes/output pixel: four
RGBA16Uint 4x attachments plus Depth32Float 4x. Existing color/support ping-pong
history uses 32 bytes/output pixel; two R32Uint owner histories add 8. Total is
184 bytes/output pixel, plus 4 bytes/input pixel for low-resolution owner.
That is about 364 MiB at 1080p or 1.42 GiB at 4K, excluding existing low-resolution
planes, driver allocations and transient overlap. The new route refuses extents
above 128×128; a 64×64 instance accounts for 753,664 bytes plus input owner.
Position/readback diagnostics must not silently become an adopted storage design.
Precision and admission are qualified first; storage reduction and measured cost
are separate future work. Ordinary rendering does not allocate these targets.

Next native work requires a separate bounded CPU grant after review: incremental
renderer test-artifact build, the focused new controls and strict lint. One later
offscreen numerical invocation will test the new route. Saved owned-buffer images
can support subsequent pixel review; OS capture and native-window presentation
remain prohibited. No follow-up native execution is implied by this source change.
