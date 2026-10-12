# 0051 — Cooked output and geometry appearance acceptance

Preparation only. Do not start ACP, native processes, GPU tests or PNG captures
until root grants this packet exclusive graphics ownership after 0050. Both
cooked features stay off by default. Use Claude Opus 5.5 through ACP, explicitly
confirmed Max thinking on every new or resumed session. Do not substitute a model.

## Frozen scope

Read `docs/spikes/cooked-output-shaders.md`, `cooked-geometry-shaders.md` and
this packet's `prepared-inputs.json`. Astra owns the contract/implementation and
numerical assessment; Claude owns all PNG creation, pixels and appearance review.
No shader, layout, math, production or dependency edits are authorized here.

The historical output-only pair comes from source
`550b5635ac04d47ea9eee2f8baacc25ae88b4652`, implementation `a8dd12e9`.
It already passed its two existing HDR tests on Apple M5 Pro/Metal: all 16
normalized RGBA cases were exactly equal (four formats; four swatch and twelve
resize cases). Receipt SHA256
`af64a4dfb81e46f62e82f83313d6bf07de6a4cafb7caaf7b26257152efc59a49`.
That was numerical only; no images or Claude appearance review were produced.
Do not describe it as geometry acceptance or Vulkan/DX12 coverage.

The new default/cooked-geometry pair is built from the exact same source
`85e1c5f` (full hash in `prepared-inputs.json`), production `c5268ae0`.
Only the cooked build enables `cooked-geometry` (and its `cooked-output`
dependency). The manifests record complete source/build/feature identities.
Pinned Naga/wgpu are 29.0.4. Geometry adds diagnostic geometry and all eight
shadow variants; the material, sprite and light-grid families still use WGSL.
No pipeline-cache/per-level recording/async fallback/device-hitch completion is
claimed. MSL driver compilation is still expected. HLSL and SPIR-V emission is
CPU evidence; actual driver tests in this packet cover Metal only.

## Launch prerequisites

Astra copies the listed immutable executables/manifests into this worktree's
ignored `artifacts/tools/0051/`. Verify SHA256 against `prepared-inputs.json`
and each pairing manifest before every child. Never run from a live Cargo target.
No Cargo/builds are needed or permitted during the graphics interval.

Before each GPU child require `artifacts/tools/0051/gpu-review-ready.json`
containing the exact prepared-inputs SHA256 and an explicit current root grant.
The gate is deliberately absent during preparation. Use one exclusive lock around
checking the grant and executing/reaping a child. Do not create or restore your
own grant. Stop immediately if it is withdrawn; a released marker permanently
closes this interval. Root/Astra must authorize any subsequent capture interval.

The new test-only `diagnostic_output` fixture now draws actual diagnostic cubes
through known typed SceneInput and an explicit camera. It compares empty/base/
raised poses in all four formats, requires substantial bounded nonbackground
pixels for both reflected placements and verifies upward projected motion. Its
CPU compile/Clippy passed; actual GPU assertions and paired equivalence remain
pending. The earlier v1 geometry binaries do not contain this test; use v2 only.

## Existing numerical controls, after the separate grant

Run serialized with no native windows or provider/account actions. Use a fresh
output directory for every command; remove inherited `INCANT_*_EVIDENCE` and
`INCANT_SHADOW_FIXTURE_COPY` values before supplying the explicitly listed ones.
Never overwrite earlier receipts. Timeout each full suite at 180 seconds and each
focused capture process at 60 seconds; terminate/reap on timeout and retain failure.
Do not retry a failed check or adjust tolerances to make it pass.

For each new geometry binary pair, run the full existing ignored renderer suites
once, with the unmodified assertions and test lists in the pairing manifest:

```text
<default-incant_render> --ignored --test-threads=1 --nocapture
<cooked-geometry-incant_render> --ignored --test-threads=1 --nocapture
<default-model_gpu> --ignored --test-threads=1 --nocapture
<cooked-geometry-model_gpu> --ignored --test-threads=1 --nocapture
```

Set `INCANT_OUTPUT_NUMERICAL_EVIDENCE=<fresh per-mode output directory>` and
`INCANT_GEOMETRY_NUMERICAL_EVIDENCE=<fresh per-mode diagnostic directory>` for
each lib suite. Compare the 16 current default/cooked cases exactly, including raw RGBA,
case/format/dimensions/source and adapter identity. Also compare all twelve diagnostic cases (empty/base/raised times four formats)
exactly, with matching adapter/source/dimensions and nonbackground assertions.
Existing analytic assertions remain unchanged. Full model suites include all eight shadow variant preparation,
mask holes/reflection/culling, retained range/offscreen lights, cascades,
queued atlas resize, geometric normal offset, and pipeline warmup over all four
output formats. They also retain the existing unrelated material/light/sprite/
retained/source-deletion controls; do not reduce those suites to the changed paths.
Report actual tests executed and backend, rather than assuming counts from a prior
branch. No PNG export variables are set for these numerical-only suite runs.

## Bounded actual image review

After successful numerical controls, reuse existing capture hooks. No new scene
or fixture catalog is needed. Preserve both sides and inspect actual pixels:

- Old output pair: filter `hdr_output::hdr_values_survive_until_tone_mapping_and_transfer_runs_once`
  with `--exact --ignored --test-threads=1 --nocapture`, setting
  `INCANT_HDR_EVIDENCE=<fresh per-mode output directory>`. One 384x128 swatch PNG
  per side. The four-format claim comes from raw numerical receipts, not this PNG.
- New geometry pair: filter
  `shadows::lookdev::directional_shadow_lookdev_contact_slopes_and_curves` with
  the same exact/ignored flags, setting `INCANT_SHADOW_EVIDENCE` to a fresh
  per-mode directory. Four 640x480 captures each: three sun elevations and
  the unshadowed control.
- New geometry pair: filter
  `shadows::alpha_masked_and_reflected_casters_preserve_holes_and_blend_casters_fail_explicitly`
  with the same flags and fresh shadow evidence directory. Five 640x480 images
  each: holes, reflected mask, single/double-sided backs and blended noncaster.

- New geometry pair: filter
  `diagnostic_output::diagnostic_vertices_draw_reflected_geometry_and_preserve_display_formats`
  with the same flags and a fresh `INCANT_HDR_EVIDENCE` directory. Two 160x96
  captures each, base and raised; the empty control and all four formats are
  separately covered by numerical records.

This is 24 exported PNGs total and eight focused subprocesses, in addition to the
four numerical suites. Do not generate further images without coordination.
Review tone/color, silhouette, shadow contacts/slopes/curves, mask holes,
reflection and culling. Exact decoded bytes are the first comparison, but actual
Claude pixel inspection is required. Diagnose differences; do not rebaseline,
change tolerances, or label a shared limitation as a new regression. Retain a
compact unedited selection; full PNGs can remain ignored with a hash manifest.

The fixed test fixtures are existing trusted renderer tests. This packet does
not authorize a new project mutation route; any later real project fixture must
use the public command bus. No browser/native UI/system screenshots/gamepads,
audio devices, source mutation, shell access for the engine agent, or provider
turns are requested.

## Release and deliverable

Record exact child commands, binary/source hashes, PID, start/end, exit, logs,
actual OS/adapter/backend, case/dimensions and capture hashes. After the last
child, withdraw the grant under the same lock, mark RELEASED, verify every child
is reaped/absent, then report graphics release before writing the final report.
Do not make later renderer calls while assembling documentation.

Write `handoffs/0051-cooked-shader-acceptance/result.md` with verified Opus 5.5/Max,
scoped output, diagnostic and shadow appearance verdicts, numerical evidence references,
defects/repro, and explicit pending unexecuted backend/device gates including Vulkan/DX12,
phone/performance/default-enablement gates. Only write this packet and ignored
`artifacts/0051-cooked-shader-acceptance/`. Commit with `Built-by: claude`.
No push, PR, merge, account access or production changes. If ACP cannot verify
both settings, preserve this packet for manual Claude execution without fallback.
