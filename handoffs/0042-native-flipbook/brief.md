# Native flipbook playback appearance review

Use Claude Opus 5.5 through ACP with Max thinking explicitly verified before this
prompt. Astra owns implementation, numerical correctness and integration; you own
rendered-pixel analysis and appearance acceptance. Do not reuse another handoff ID.
Read CLAUDE.md and docs/spikes/native-flipbooks.md.

The immutable candidate source/binary identities will be recorded under
artifacts/tools/binary.json before execution. Use only the supplied
incant_headless, cook_scene and native_frames binaries. Verify SHA256 before and
after commands. No Cargo, shader, engine, schema, SDK or editor edits. No native
application launch or system-wide input. Review uses bounded offscreen captures.
Do not capture until artifacts/tools/gpu-review-ready exists; the GPU slot is
coordinated by Astra and another packet may still own it. Your capture wrapper must check
the gate and frozen executable hash before every GPU invocation, use an explicit
120-second subprocess timeout, and terminate/wait for all children on timeout.
Do not detach capture processes or silently extend the capture budget.

## Actual path and contracts

Create projects only through public init/import/RPC typed commands; never directly
edit project/journal content. Original source PNG/atlas files and native_frames
request JSON are legitimate external inputs. Cook with cook_scene, then execute
native_frames on the cooked scene and retained caches. This helper steps the real
NativeWorld, retains NativeRenderInput and RetainedScene, and selects current
geometry without per-tick rebinding. Its sidecar report records the committed
frame, play/completion state and binding/topology generations. The ordinary
headless screenshot is only a static first-frame authoring preview; it must not
be described as native playback.

SpriteFlipbook uses one SpriteRenderer texture with 1–256 ordered pixel/atlas
regions and exact rational rate {numerator,denominator}. Every region is resolved
at binding, not lazily at selection. Animation replaces the SpriteRenderer's
otherwise unused static region. Frames may differ in source size/atlas placement;
existing destination size, pivot, tint, reflection, sampling, nine-slice and
painter-order semantics remain unchanged. There is no new art/style, shader,
camera or pixel-grid policy. Static sprites and tilemaps share the same pipeline.

At tick0 the first frame is selected. Each successful fixed tick advances the
rational cursor. Pause preserves subframe phase; play resumes it; seek resets
phase/completion but retains play/pause; restart resets to frame0 and plays.
A nonlooping sequence holds its final frame for its full duration then completes;
play on a completed sequence restarts. Helper controls apply after reaching the
specified committed tick and before capture. No QuickJS/Wasm animation SDK, full
input/session/audio/save or on-screen presentation is implemented by this helper.

## Bounded helper protocol

native_frames <cooked-scene> <model-cache> <texture-cache> <camera-hex-id>
              <request-json> <new-output-directory>

IDs are 32 hexadecimal digits of the cooked ULID bytes. Requests, parsed only at
startup, have this exact form (controls optional):

```json
{"width":640,"height":360,"ticks":[0,1,5,10,15,20,25,30],
 "controls":[{"tick":10,"entity":"00000000000000000000000000000001","action":{"kind":"pause"}},
             {"tick":20,"entity":"00000000000000000000000000000001","action":{"kind":"seek","frame":2}},
             {"tick":25,"entity":"00000000000000000000000000000001","action":{"kind":"play"}}]}
```

Use actual fixture IDs, not those example bytes. Captures must be strictly
increasing; controls ordered by tick (same-tick order preserved). Dimensions
16–1024, at most64 captures/256 controls per invocation, ticks0–36000, at most256MiB
decoded output per invocation. This is a review artifact utility, not an atomic
export: earlier PNGs remain if a later simulation/render/output write fails.
Scenes are limited to 4096 entities and 256 flipbooks. Audio scenes are explicitly refused. Native physics/velocity may advance; no
scripts or input host is present. It reads only cooked data/caches during setup.

## Review matrix

Create a small original diagnostic animation with asymmetric, distinguishable
frames, transparent edges, nonzero packed origins and unequal frame dimensions.
Compare native captures to independently authored static-frame controls using the
same intended transforms/painter order. Keep evidence attributable to the real
selected frame; do not edit pixels to simulate animation.

- Verify a short looping sequence at a fractional rational rate, with capture
  ticks on both sides of frame boundaries; check playback report state too.
- Pause, seek while paused, resume, restart, and a once sequence's final hold.
- Nearest and linear sampling; ordinary and nine-slice sprites with valid borders;
  reflected x/y, inherited transforms and modest magnification. Preserve known
  camera/nearest tie limitations rather than silently redefining them.
- Mix animated entities with static sprites and a retained tilemap, including
  overlapping painter order. Static content should remain steady and no stale
  frame geometry, trails, seams, crop shifts or ordering changes should appear.
- Remove source project/PNG/atlas files after cooking (keep private fixture copies
  for reproducibility), rerun native capture from cooked scene/caches, and compare
  hashes. This proves cooked loading, not a phone/performance/shipping gate.

At most96 total captures and256MiB total raw decoded pixels for this packet.
Prefer a small reusable contact-sheet analysis over additional broad captures.
No performance, provider, device, interactive-player or phase-gate claim. Astra's
numerical GPU oracle and fresh shipping proof remain distinct evidence.

Allowed changes: handoffs/0042-native-flipbook/{helpers,art,evidence,result.md} and
artifact manifest only. Keep generated projects, caches, tools and PNG outputs
in ignored artifacts directories unless specific small evidence is needed.
No production/source edits, publishing, accounts or credentials. Return precise
reproductions and proposed corrections instead of modifying the engine.

Return result.md with verified model/effort, source/binary/input/output hashes,
public commands, capture count and adapter/backend, scoped acceptance/failures,
and limitations. State explicitly when all capture processes have ended so Astra
can release the GPU. Commit only packet files with Built-by: claude; Astra owns
this brief and integration.
