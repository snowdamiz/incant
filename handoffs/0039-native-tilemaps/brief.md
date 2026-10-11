# Retained tilemap appearance and atlas placement review

Use Claude Opus 5.5 through ACP with Max thinking explicitly verified before this
prompt. Astra owns implementation, numerical correctness and integration; you own
rendered-pixel analysis and appearance acceptance. This is a new isolated packet.
Do not alter or overwrite any older handoff evidence.

The source candidate is `b2d0bad` (reviewed implementation `420f866` plus main64
integration; owned renderer source is identical). Read CLAUDE.md,
docs/spikes/native-tilemaps.md and docs/spikes/native-tilemap-rendering.md. The
frozen authoring review executable is supplied under artifacts/tools/incant_headless
with binary.json. Verify its SHA256 before and after commands. No Cargo, Rust,
shader, schema, SDK, editor or production-source edits. Use only this executable;
no substituted installed binary. Do not launch native applications or use native
UI/system-wide input. This review uses bounded headless engine captures only.

## Implemented contract

The existing public Tilemap component uses one cooked color atlas, dimensions,
cell_size and stable-ID layers with cells in bottom-to-top row-major order. Each
occupied cell displays its entire atlas frame across that cell rectangle, nearest
sampling, white tint, no per-cell flips and unlit sprite shading. A frame's atlas
pivot and packing offset do not offset the local grid. Local cell (0,0) begins at
the entity origin; +x right and +y up. Atlas image pixels remain top-down.
Inherited f64 translations, rotations, scales and reflections apply to the map.
There is no new pixel-grid placement or camera-following contract.

Tiles and ordinary sprites share the existing pipeline. Painter order is
(layer, order, entity ID, stable layer ID, y, x), and compatible adjacent entries
batch without changing that order. Empty cells emit no geometry. A map keeps one
native world-transform slot; frame data and geometry are retained across motion.
Configuration/topology changes rebuild atomically. Normal native dependency graphs
remain free of authoring. The authoring screenshot command feeds the same renderer
through its authoring adapter; do not call these captures native game execution.
Astra's actual cooked v6/source-deleted native GPU tests compare eight retained
frames with independently expanded raw-cell ordinary sprites and test failure
rollback. All 45 ordinary and 62 native GPU tests plus Clippy pass. Those numerical
results are not your appearance verdict.

Solid tilemap simulation still refuses the missing Rapier2D consumer. Legacy
Play/headless Play startup/sync also refuses Tilemap; do not use Play for this
packet or add a document simulation path. Static screenshot/RPC authoring is
supported. Separate actual Rapier2D work is underway and outside this review.

## Review scope and public workflow

Create small original diagnostic atlas art or reuse earlier handoff art with
provenance. Build projects only through public init/import/RPC typed commands,
never directly edit project or journal files. Source PNG and .atlas.json files
are legitimate inputs. Reuse public helper patterns from 0034 when useful.

Review a focused set of cases, not a new broad renderer benchmark:
- Distinct asymmetric frames with visible top/bottom/left/right, transparent
  edges and nonzero packed atlas origins; unequal frame sizes stretched into
  equal cells. Confirm bottom-to-top grid order, empty holes, full frame identity
  and no accidental atlas-pivot offset.
- Adjacent cells, multiple overlapping layers, equal layer/order ties using stable
  IDs, and interleaved ordinary sprites. Compare to a deliberately constructed
  ordinary-sprite control with the same intended placements and order.
- Inherited translation/rotation, reflected x/y and nonuniform scale; explicit
  orthographic and pixel-perfect cameras at modest magnification. Look for seams,
  clipped edges, sorting changes or doubled transforms. Do not infer arbitrary
  zoom correctness from this small set.
- Valid cell/order changes and accepted atlas repack through the public command
  and import paths. Verify actual atlas relocation and stable selected frames.
  A rejected missing-frame reimport must preserve the prior preview. Exercise
  Undo/Redo where available. Preserve precise commands and source fingerprints.
- Repeated fresh process captures and source PNG/atlas removal with cooked cache
  retained; source-free captures should match the corresponding cached baseline.

Existing shared renderer limitations remain explicit: nearest exact ties above
128x, some two-axis frame-edge/non-dyadic projection ties, and camera-following
near ties are not fixed by this feature. Read the 0032 round-five and 0034 scoped
conclusions; report any new tile-specific defect separately. Do not solve shared
renderer or camera math here, and do not silently redefine the cell contract.

## Bounds and coordination

Wait for artifacts/tools/gpu-review-ready before issuing GPU screenshot commands;
you may prepare CPU inputs before that file exists. No native foreground is
required. The supplied release is not a shipping-profile/phone performance proof.
At most 128 captures and 256 MiB raw decoded image bytes for this packet, and at
most 16 frames in any helper invocation. Plan/reuse captures and stop with a
precise reproduction if more is necessary. No performance, provider, phone,
interactive player or phase-gate acceptance claim.

Allowed changes are only handoffs/0039-native-tilemaps/{helpers,art,evidence,result.md}
and its artifact manifest. Keep generated projects, cooked caches and private tool
copies in ignored artifacts directories. No production/source modifications,
accounts, credentials, publishing or merging. Original authored art/evidence may
be committed only within this packet. Return a precise reproduction and proposed
correction for an implementation defect rather than editing engine code.

Return result.md with model/effort verification, exact source/binary hashes,
public commands, artifact hashes, capture totals, device/backend, scoped
acceptance or concrete failures, and remaining limitations. Explicitly state
when the last GPU capture process has ended so Astra can release the device slot
while you finish the report. Commit only your packet files with Built-by: claude.
Astra owns the brief and integration. Do not commit binary artifacts.
