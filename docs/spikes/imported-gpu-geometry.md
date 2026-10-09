# Imported indexed GPU geometry

Phase 1 increment, 2026-10-09. The shared renderer now resolves validated
MeshRenderer bindings against the current immutable cooked asset store. A model
uses its default glTF scene (or first scene), composes entity and node transforms,
and uploads indexed vertices plus instance transforms. Unreferenced library nodes
do not become scene instances. Repeated primitives share a draw call and buffers.

RenderScene owns exact GPU/CPU versions through Arc references. GPU buffer lookup
uses cooked fingerprints and weak entries; failed scene preparation cannot alter
an earlier published scene. The editor prepares only on document revision changes
or failed-load retries, outside the command-bus lock. Reimport and Undo/Redo
therefore select the appropriate immutable version. The headless screenshot
command and agent perception use the same asset binding and drawing path.

The current appearance remains monochrome diagnostic shading. It does **not**
implement GPU materials/textures, lighting, shadows, PBR or the Phase 1 render
graph. Material overrides fail explicitly. Missing/stale bound models fail rather
than being replaced with cubes. Unbound Transform entities retain Phase 0 cubes
for compatibility. Singular/out-of-range transforms and more than 100,000 model
primitive instances fail before uploading scene state. Camera controls, active
scene selection and simulation-driven render snapshots remain open.

## Verification

- CPU behavior checks exercise default-scene/node hierarchy, grouped instances,
  and missing/stale model rejection.
- Two explicitly invoked native-GPU tests pass on Apple M5 Pro: actual indexed
  pixels survive source removal, reimport changes output, retained scenes keep
  original buffers, and the editor controller recovers from failed replacements.
- The public CLI probe imports through incant_cmd, binds two model entities in
  one command transaction, renders one instanced draw, removes the sources and
  reproduces identical PNG bytes, then reimports changed vertices and observes
  changed output. Parent hierarchy/flattened diagnostic equivalence still passes.
- macOS source checks and Windows/Linux desktop workflows explicitly invoke the
  GPU tests. Desktop jobs also execute the public CLI probe. Hosted results are
  pending for this increment; local tests do not count as hosted evidence.
- Claude handoff 0011 owns rendered-pixel/native composition review and accurate
  viewport error wording. Native review is pending.

The last valid GPU scene stays allocated on an asset/render-preparation failure;
the existing error overlay currently covers it. Retry is once per second or on
the next revision. Asset decoding/upload happens on the render worker and can
delay frames while loading; the native event thread and document lock remain
available. No frame-time or production-memory-budget gate is claimed here.
