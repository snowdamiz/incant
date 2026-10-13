# Default-off SSAO on current main

This curated integration takes the complete diffuse SSAO, plane-aware filtering
and C2 sampling implementation from `02c71fdb`, independently accepted CPU and
Metal appearance evidence. It does not merge the producer's private ancestry.
The optional feature remains off by default. Native `cooked-materials` retains
its typed refusal; no cooked SSAO family or non-Metal execution is claimed by
the producer evidence.

## Current composition

The AO, filter and prepass WGSL are byte-exact producer source. Main's existing
material, motion, temporal, bloom, cluster and other shader files are unchanged.
The current normal dependency graph, lock and cooker code are unchanged; the
only Cargo addition is the default-off `ssao-stage-readback` diagnostic feature.

View-space reconstruction uses the same jittered camera projection as opaque
geometry. The existing geometry matrix computation and its operation order are
unchanged. The new projection helper does not reconstruct projection through a
translated view matrix; a numerical test projects through actual geometry and
reconstructs in perspective/orthographic views with fractional viewport offsets.

The four SSAO passes execute once before the opaque or unsplit model route. All
four pass descriptors participate in the existing Models timing envelope.
Transparent geometry keeps the current stable camera/depth path, uses the ordinary
material pipeline, and receives no AO. Existing temporal history, overlay,
integer owner, bloom and Display scheduling retain their respective ownership.
An additional ignored four-frame joined test exercises SSAO, fixed history,
transparent geometry, bloom, 50% DRS and actual supported/unsupported timestamp
branches, or precise cooked refusal with an ordinary completed frame. It is not
an executed GPU result until separately granted and retained.

The SSAO cache and settings retain their producer limits: 4,194,304 pixels,
14 logical bytes/pixel, bounded optional stage readback, and no bound on
caller-owned queued physical resources. Changes to settings prepare a candidate
under validation scope before replacement. Every original parameter, viewport,
budget, stage-oracle and revised quality control remains present.

## Evidence boundaries

The accepted producer report is Claude0070 `c73d276a`, reviewed by root receipt
`22ac4dc7`. It reports 43 numeric gates and all 14 N8 rows passing; Claude reviewed
16 PNGs and 11 pairs. Authored N6 remains failed at seven codes. The revised N6T
and N8 margins remain 0.79 and 0.97 codes respectively. Original execution001 and
002 are separate historical evidence; this composition does not retroactively
pass their failures or claim a new appearance judgment.

Current source review is independently accepted (`97d2f158`). Thirteen CPU and
compile checks passed at `c01bed48`: 168 default and 200 overlapping cooked
observer cases, strict Clippy, native-window/browser consumers, dependency rules
and exact identity of all 33 previous native shader payloads. The earlier user
pause and exact checkout recovery are retained. After resume, Claude Opus 5.5
with ACP-verified Max executed the frozen runner once on Apple M5 Pro Metal: two
listings and two tests, eight 65x33 frames total, all children successful and
reaped. The default route admitted SSAO and completed four positive instrumented
frames; the cooked route explicitly refused SSAO and completed four ordinary
fixed-temporal frames. No images were captured or reviewed for this composition.
All 33 prior native shader payloads remain byte-exact. Final independent integration review is accepted (`c3e4b994`); publication
awaits the required fast check.

[Current manifest](evidence/ssao-main229.json), [original pause](evidence/ssao-main229/pause.json)
and [original001/002 retention](evidence/ssao-main229/producer-retention.json)
preserve historical and current identities separately. No default enablement, non-Metal appearance, phone, performance,
whole-frame GPU cost or phase-completion claim is made.
