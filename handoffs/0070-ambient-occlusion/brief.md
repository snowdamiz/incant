# Root revision3: CPU build only, root supervised

The corrected helper lifecycle/lock/latch protocol is accepted for CPU build.
Decisions: keep precise-retry latch strictness; reconcile may take only packet
lock because its recorded orphan can hold the inherited global lock; CPU builds
need only packet lock; keep45-minute wall-clock ceiling and count all four
listings within16 children. Do not widen scope or add another framework.
Root will run runner.py build directly through a supervised process using the
new build-grant.json under ~/.codex/incant-evidence/0070-effects-build. Target
/tmp/incant-dynamic-resolution-target is exclusively assigned to this build.
Do not launch any build, GPU child or alter helpers/source while root builds.
No GPU grant exists. After compilation root will review exact binaries and
complete fixture checks before sending the execution instruction. Preserve
Opus5.5 explicitly Max for the next resumed Claude execution and pixel review.

# Root review revision2 — preparation corrections only

No GPU or Cargo target grant yet. Opus5.5 explicitly Max. Preserve previous result
as preparation-result-v1.md; fix these focused runner defects, rerun CPU controls,
and return final identities. No new broad source survey.

1. `Ledger.locked()` only holds packet runner.lock. Add the shared
~/.codex/incant-evidence/native-gpu.lock, acquired nonblocking BEFORE packet lock
and held through preflight/reservation/launch/kill+wait/completion/release. Never
allow separate packets to render concurrently. Add a two-packet control proving
one refuses before reservation while the other owns the shared lock.
2. A failed completion currently leaves no unresolved reservation, so gpu_checks
allows unrelated next GPU children. Latch production/test/validation failures and
refuse further device work except a separately root-authorized precise retry.
Do not classify missing tests/verdict/invalid frame receipt as success.
3. Reconcile must not SIGKILL an arbitrary recycled PGID just because every member
started AFTER the old reservation. Require retained exact process identity for
signaling; if ownership cannot be proven, refuse and report for root handling.
A positive timestamp comparison alone does not establish ownership. Ensure failed
reconciliation/group survivors cannot unblock subsequent children.
4. Root expands EXISTING regression budget to8192 frames (combined8320), preserving
new128/PNGs32/serial16children/45minutes. This admits the actually counted6264-frame
wide test plus1828 others, so include all94 compiled cases exactly once and remove
the artificial deferral. Derive totals again; do not shrink the test. No new GPU
or target permission follows from this budget correction.

Root accepts half-resolution as canonical SSAO treatment; full-res quality can be
separate later. Oversize refusal can remain CPU-covered. Cargo config key/hash
inspection is appropriate. Existing prior128 new-frame cap and stop-on-production-
failure remain. Root reviews the numerical Rust helper separately before grant.

---

# Priority continuation — 2026-10-13

The first preparation session was interrupted only to apply the director's
already-authorized bypassPermissions mode. Its five owned processes are gone;
no build/target/GPU grant was issued and no capture was made. Resume source-only
preparation in this same checkout. Explicit Opus5.5 Max remains mandatory.
Do not restart the source survey from scratch: continue the combined fixture and
bounded runner/case plan you were preparing. No Cargo, GPU, target or capture yet.
No memory or outside notes. Model/effort verification is performed before this prompt.

For context, the exact engine source now has completed union CPU evidence on the
parent at94f604fa (docs-only after your b21f91c1). All11checks passed:25shadercook,
98default renderer/host,125cooked with overlap,94GPUignored per variant,Clippy,
web/native compilation and dependency checks. You may read the parent's
`docs/spikes/render-effects-composition.md` and its evidence manifest read-only.
Engine hashes are unchanged from reviewed b633f92d.

Known runner mistakes found in another packet must not recur: use git porcelain
-z without stripping leading status columns; freeze every relevant actual dirty
source and helper byte plus the compiler binary inputs; put frame reservation
under the same lock as launches; always finally kill/wait the owned process group
on interruption/error/timeout before releasing the lock. If reusing previous
frames, bind exact source/binary/variant/route/viewport/state and actual files;
simpler fresh bounded captures are preferable. Construct an explicit nonsecret
environment and record only approved overrides.

Return the prepared helpers promptly once ready for source review. Do not wait
for a grant and do not repeat extensive evidence reads already completed.

---

# 0070 — opt-in diffuse SSAO: backend and appearance acceptance

Preparation only. **No GPU/native process/target lease is granted by this packet.**
Root schedules this after the already queued work. Do not launch until root grants
an exclusive target and graphics lease. Require Claude Opus 5.5 through ACP,
thinking explicitly Max, verify both before every new or resumed session.

## Ownership and base

Astra owns the numerical SSAO implementation and CPU correctness. Claude owns
actual rendered review, images and appearance. Source base is main192
`7fc413d23cbb65e244ca515ca5d684aec21cfb90` plus this packet's containing commit.
Profiler branch `bfe8ede2` and DRS branch `83852138` are preserved separately.
Root/publisher will provide the final union: pending DRS `1a1c6c07` and GPU
 timestamps `99778642` must not be discarded when composing renderer/lib hooks.
No remote publication. Never edit the producer's worktree; root chooses an idle
isolated checkout and target. CPU builds at most two jobs, GPU tests serially.

Read `docs/spikes/screen-space-ambient-occlusion.md` and its evidence manifest.
All existing WGSL remains byte-identical. New SSAO programs are portable WGSL.
The native `cooked-materials` combination **explicitly refuses** enabling SSAO;
do not remove that guard or silently substitute a runtime material shader for a
cooked artifact. Compare matched baseline/candidate feature sets without that
feature. Other cooked shader families may be enabled only in both sides equally.
No default enablement is authorized. This is not temporal upscaling or MetalFX.

## Exact connected interface

Initialize a renderer, then before handing it to the host call:

```rust
pollster::block_on(renderer.enable_ambient_occlusion(
    incant_render::AmbientOcclusion { radius: 2.0, bias: 0.01, strength: 1.0 }
))?;
```

Those values are numerical fixture inputs in world/view units, not approved art
presets. Default constructor leaves the option absent. Disable with
`renderer.disable_ambient_occlusion()`. Do not put a per-frame call around the
setup function; it prepares 17 finite pipeline variants.

ModelDraw performs: opaque/MASK depth+surface normals → 16x4 AO sampling → forward
models. The same retained vertex/instance/index data, normal transforms, alpha
cutoff, winding and culling feed both geometry passes. AO only multiplies diffuse
IBL. Direct/shadowed light, emissive, specular IBL, transparent BLEND, sprites,
world text, screen UI and output/chrome are not AO receivers. Transparent and
fallback diagnostic geometry are not occluders. No temporal history or filter.

The AO cache uses actual internal model target dimensions/viewport. Max 4,194,304
pixels, 52 MiB logical attachment payload; driver/in-flight/other resources are
additional. Do not silently increase this bound. Record typed refusal of larger
SSAO targets separately from valid default rendering. Output/UI dimensions and
viewport/corner masks must remain physical. No phone/performance claim.

## Bounded numerical and rendered acceptance

First freeze all source, executable and input hashes, adapter/backend/driver,
features, exact commands and process IDs. Preserve every failure, correct only
its actual cause, and distinguish superseded results.

1. Run the existing ignored numerical test, which checks zero-strength parity,
   queue/resource lifetime across odd extents and offset viewports, invalid
   reconfiguration rollback and an empty scene. It does **not** prove AO works:

   `tools/cargo test --locked -p incant_render --lib ambient_occlusion::device_tests -- --ignored --test-threads=1`

2. Add a small source-generated numerical geometry fixture through the existing
   typed/cooked glTF route, not game art: visible flat receiver at view z=-4,
   finite nearby foreground slab or corner at z=-3 with a side boundary. With a
   positive diffuse environment, assert nonzero bounded accessibility near the
   corner versus 1 on an isolated flat control. Use both perspective and
   orthographic projection. Retain actual AO/readback numeric values or raw
   attachment data and source. Do not settle for a blank frame/unchanged image.
   Require finite [0,1] AO, background 1, normal-sign/near/far/radius controls and
   out-of-screen edge behavior consistent with the documented single-layer
   approximation. No golden aesthetic threshold should replace these invariants.
3. In the same real material path, isolate direct-only (environment intensity0),
   emissive-only, metallic/specular-only (diffuse factor0), BLEND-only, and mixed
   opaque then BLEND draws. AO toggle must preserve the excluded contributions;
   diffuse-only must change for the occluding geometry. Preserve raw numeric
   outputs. Check the actual bind-group/pipeline switch between opaque/BLEND.
4. MASK cutout and below/equal cutoff coverage; double-sided backfaces and
   mirrored/nonuniform instance transforms. Include actual retained animation/
   deformation where feasible so the prepass cannot use stale static vertices.
   Use a flat analytic control to detect self-occlusion from bad normal space.
5. Resize repeated/alternating dimensions, queued frames submitted out of encoding
   order, dropped source assets/scene, invalid viewport/config followed by valid
   frame. Include odd extents and a fractional offset viewport. Verify original
   physical UI/chrome and text dimensions are unchanged. If DRS is integrated,
   test internal50/75/100% with the same camera/output dimensions; its telemetry
   lifetime remains the separate DRS contract.
6. Few complete frame pairs (bounded three or four views) allow Claude to assess
   artifacts, edge halos, normal precision and readability. Discuss known
   offscreen/hidden-geometry and fixed-kernel limitations honestly. No aesthetic
   tuning by Astra, no unsupported quality preset, and no default enablement.

List/rerun the full affected renderer GPU suite on the final exact source union,
serially. The producer CPU source currently has 83 ignored GPU tests; a later
union may add tests, so record the actual list/count instead of repeating that
number. No screenshot cap workaround or unrelated sample/art changes.

## Deliverable and cleanup

Return `result.md`, focused diff, exact command/exit logs and source/binary/input
hashes, bounded numeric readbacks, selected frame pairs with Claude's explicit
appearance judgment, and all PIDs/leases reaped/released. State backend(s) actually
executed. Source/CPU acceptance does not equal GPU or phone acceptance. Keep
SSAO cooked-artifact support, native default exposure, profiling subpass entries,
TAA/upscaling, device budgets and complete Phase1 rendering explicitly pending.
Root reviews correctness before publisher intake; no automatic PR or launch.

## Launch budgets and environment (required before root grant)

Prepare one bounded runner with a process-wide timeout and a render counter before
execution. New SSAO controls: at most 64 complete frames and eight independently
reviewed baseline/candidate frame pairs (16 of those64), dimensions at most
640x480. Existing affected GPU regression: one serial run, at most 2048 encoded
render frames, maximum30minutes; combined ceiling2112frames. Count actual frames
and stop/refuse when a ceiling would be exceeded rather than silently extending
a run. If existing tests cannot fit/count this budget, report the exact test and
ask root to revise the packet before that test runs. These are execution ceilings,
not device performance measurements. No screenshot/video loop or scene soak.

One test executable child at a time, no browser/editor/game window unless root
explicitly adds it. Maximum three test executable launches total (existing
suite, new numerical controls, one focused repair retry), no more than two
simultaneous compiler jobs, all process groups owned and reaped. Preserve failed
first output. Any additional retry requires a new source/reason receipt and root
coordination. Release the GPU/target leases immediately after the bounded run.

Use an explicit environment allowlist. Start with PATH pointing to the pinned
Rust/compiler and system tools, the root-granted absolute CARGO_TARGET_DIR, and
TMPDIR only if a private existing directory is required. HOME, CARGO_HOME and
RUSTUP_HOME may be forwarded unchanged only when needed to find the actual
installed tools; never fabricate/replace the user's HOME. Set the expected
backend and evidence directory explicitly for this run. Do not inherit ambient
INCANT_*, WGPU_*, TRACY_*, RUSTFLAGS/CARGO_ENCODED_RUSTFLAGS, CARGO_FEATURE_*,
compiler wrappers or arbitrary account/provider environment. Reject unapproved
build overrides before launching. Record allowed key names and nonsecret tool/
evidence paths; never dump the parent environment or credentials. Target/backend
selection, shader features and source identities must match the frozen commands.

## Root priority revision — combined effects preparation, 2026-10-13

This section supersedes earlier source, separate-packet and launch-count details.
Prepare one combined SSAO+bloom numerical and visual review on this checkout's
reviewed renderer union `b633f92d9d20cd8ed4c328b0b670ab7935797801`:
cluster/DRS integration `9b1df9db`, accepted SSAO `0ae7cc02`, and bloom `10befc30`.
Read both this packet and `handoffs/0071-bloom/brief.md`; 0071 will not run separately.
The combined source already maps bloom sampling to internal coordinates and has
an ignored reduced-resolution emitter-position test. Source-only review found no
integration issues. Root's 11 CPU/compile/lint/tooling checks passed on source003;
that is not efficacy or appearance acceptance. GPU timestamps are NOT part of
this union; root is correcting the separately discovered Metal timestamp defect.

THIS SESSION IS PREPARATION ONLY: no Cargo build, target access, new GPU process,
window, render, screen capture or execution grant. You may inspect existing
retained images and source and build CPU-only planning/analysis helpers. Prepare
and commit a bounded case plan and self-contained execution helpers, then return
for root's correctness review. Do not create or update persistent, personal or
auto-memory notes. Write only within your isolated handoff worktree. A future
external evidence directory and target will be explicitly assigned by root.

Plan one matched default-WGSL union with controls off, SSAO-only, bloom-only and
both enabled. Exercise real diffuse occlusion (not just strength-zero parity),
excluded contributions, MASK/BLEND/retained deformations, perspective/orthographic,
odd dimensions and offset viewports, and DRS50/75/100. Preserve exact tone,
material, chrome and UI behavior. Include the new reduced-resolution bloom emitter
position test exactly once. Read original source and evidence limitations before
choosing fixtures; report missing observation capabilities rather than guessing.

A single retained process can handle several bounded cases; separate actual
renderer regression binaries must still run individually. Propose at most16 serial
GPU children, at most128 new frames TOTAL across both effects including all new
ignored tests and failed attempts, at most16 retained matched pairs/32 PNGs,
maximum960x540. Existing regression ceiling2048 rendered frames, combined2176,
45-minute execution ceiling excluding compilation. CPU build concurrency2 only.
Charge actual new test frames to new-frame budget even when invoked in the suite.
Freeze exact listing delta, per-case counts/source/binary/feature hashes and
construction environment before launch. There is no license to rerun failing
cases unboundedly. Original failures and all cleanup outcomes must be retained.

Helpers must re-verify source/grant/binaries, reserve budget before every child,
and terminate AND REAP owned groups in finally even for interrupted waits or
postprocessing exceptions. An uncatchable SIGKILL limitation is explicit. Build
and GPU leases are separate; root may grant one without the other. No account or
ambient build/provider variables; construct a narrow environment as above. Do
not copy unreviewed old runner failure handling. Include helper Clippy and actual
CPU process-lifecycle controls in the proposed check list. On completion return
result.md describing readiness and any blockers, not claiming visual success.


## Root priority revision — repair reviewed acceptance defects, 2026-10-13 07:30 UTC

PREPARATION ONLY. Original CPU attempt is now terminal and its build lease released;
no GPU was launched. Preserve the original evidence at
`/Users/andreyyurlov/.codex/incant-evidence/0070-effects-build`, including the helper
compile failure and `build-released.json`. Do not run Cargo or acquire target/GPU
leases in this session. Keep the current production renderer unchanged.

Read independent source receipt
`/Users/andreyyurlov/.codex/incant-reviews/effects-0070-fixtures-source-20261013.json`
(SHA256 `64774dd5978c48069f2c33f5ca72295602a6ead6546b890d31c560137a7733c6`).
Fix both analyzer acceptance groups with meaningful CPU-only synthetic controls:

1. `infer_k` must reject nonfinite or empty RGB interval intersections. SSAO
   agreement must require usable covered comparisons and actual occlusion
   comparisons, never pass when every pixel was skipped/uninferable. Preserve
   diagnostic counts. Choose explicit justified minimum coverage from the fixed
   fixture geometry, not a post-hoc result threshold. Make furnace basis uniformity
   an enforced precondition anywhere shared off/specular bases are reused.
2. Require every declared numeric case and gate, not whichever case directories
   happen to exist. Missing evidence fails with useful retained diagnostics.
   A numeric `pass=false` must write its report and exit nonzero. Preserve distinct
   numerical invariants versus Claude's later appearance judgment.

Also fix the exact Rust compile error in `tools/effects/src/composed.rs:106`:
annotate the existing eye/at coordinates with explicit `[f64; 3]` types so sqrt,
atan2 and asin resolve. Do not alter camera/layout values. Source engine test
binaries compiled successfully; helper failed on these three ambiguous calls.

Root already made two runner corrections in commits6b06c1c8 and bfecb8d6: per-child
TMPDIR and resolved Cargo dependency-identity comparison for a pruned helper lock.
Preserve them. The generated helper Cargo.lock is untracked evidence; do not
replace root Cargo.lock or change dependency resolutions. Do not modify lifecycle.py;
its accepted shared implementation is pinned by another completed GPU packet.

Add negative controls for empty inference, conflicting RGB intervals, missing
cases/gates, inadequate coverage, failed furnace precondition and nonzero CLI
failure (while report survives). Run the focused synthetic checks, parse/format
checks and diff review. Do not add a new generic execution framework. Commit only
your scoped source/test/result changes with Built-by: claude and summarize exact
changes, evidence and remaining checks. Root will supervise a fresh CPU build
once this source is reviewed. Finish with no background work or subprocesses.


## Root priority execution — 2026-10-13, latest instruction

The analyzer repairs are independently accepted (`effects-0070-analyzer-repair-20261013.json`,
SHAfc33bbd271949e7d78963867b958ea82a999f832548bc823f6a71bb4f354b8bb).
The supervised fresh CPU build and helper Clippy passed; all three owned children
are reaped and build lease released. Root independently hashed five frozen
binaries, nineteen helper files and matched renderer/host features. This session
may now execute the bounded plan using ONLY the frozen binaries and runner.
No Cargo/target permission. ModelOpus5.5 and thinkingMax remain required.

Grant: `/Users/andreyyurlov/.codex/incant-evidence/0070-effects-build-002/gpu-grant.json`.
Evidence directory is that same parent; build record SHA
96e878bff1780f346e04d3fd9b8b084eeb279bb0b9590e9930cea23d6c22f123.
SourceHEAD stays729e206af5f1f10c380794b5f66bf0beb33081ab during execution.
This root-owned brief is intentionally uncommitted in the child so its source
HEAD and compiled input identity remain exact. Do not commit or change any source,
helper or plan until after GPU release. Previous0074 released07:24, no GPUowner.

Use runner.py with --grant above: list all four suites; run all four suites once;
run the five numeric cases; run analyze.py on the evidence directory; if the
numeric analysis passes, run the appearance plan once. Existing regression
budget8192 (8092planned), new128 including20new suiteframes+73numeric+16appearance,
combined8320;16children including4listings,32PNG/16pairs,maximum960x540,45minutes.
These are the previously reviewed plan limits, not a new benchmark. Run serially
in foreground, supervise/wait every child, never finish with background work.
The two reserve retry slots are NOT approval to rerun: any failure ends device
work pending root diagnosis. Preserve every output/refusal/latch; never weaken a
comparison or increase a bound to make a run pass.

Always release through runner.py release, even after failure, and verify no owned
process survives. After rendering, personally inspect every retained paired PNG
and give a candid appearance judgment with concrete issues. Functional success
is not visual success. Root handles nonvisual defects. Return exact source/binary/
backend/counts, raw failures, explicit pending gates and a concise result. Do not
write personal memory or push. Commit only scoped report/evidence after release,
with Built-by: claude; root will review before publisher intake.
