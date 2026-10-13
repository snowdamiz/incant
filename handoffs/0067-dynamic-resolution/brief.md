# 0067 — retained dynamic-resolution execution and appearance

Use Claude Opus 5.5 through ACP, with Max thinking explicitly verified on every
new or resumed session. Read CLAUDE.md. Claude owns appearance and rendered-pixel
review; Astra owns engine correctness. Do not substitute a model or effort.
This packet is preparation only. Do not launch, build or acquire the GPU/target
until the root readiness grant below exists. 0065 and 0066 precede this packet.

## Frozen source and scope

Implementation and independently accepted CPU evidence are committed at
`1a1c6c07e62e67310c0f51ec4556344a8d67f4d6`, based on main185
`d5702546b34fcc6452a9fe2b22a0e2b23802991a`. The root grant will name the exact
integrated source used for this packet. Read `docs/spikes/dynamic-resolution.md`,
`docs/spikes/evidence/dynamic-resolution.json` and the adjacent source-review
receipt. Fourteen source/build identities, 103 lossless artifacts and five CPU
test binaries were reviewed: 73 tests pass, 89 GPU tests remain ignored. This is
not prior GPU or appearance acceptance. WGSL and dependency resolutions were
unchanged by the implementation.

The feature is explicitly opt-in and default behavior must remain unchanged.
Do not enable it by default, add game/project settings, change shader math,
filters, art, UI styling, controller policy or tolerances to manufacture success.
The existing nearest-texel spatial expansion is deliberate fallback behavior;
assess and report its real quality, without calling it temporal upscaling or
MetalFX. Source fixes requiring such decisions return to Astra first.

## Actual routes and numerical contracts

Use the existing retained renderer and composed GPU host, not a separate renderer.
`GpuRenderSettings::with_dynamic_resolution(ResolutionConfig)` wraps the existing
trusted target provider. `RetainedScene::set_dynamic_resolution` and its opaque
`ResolutionFrame` receipts expose the lower-level owner contract. Host observers
are `resolution_frame()` and `discarded_resolution_samples()`.

For fixed controls use identical policy fields with minimum, initial and maximum
all equal to `ResolutionScale::new(8)`, `new(12)` or `FULL`: 50%, 75%, 100%.
Compare disabled baseline and fixed 100% exactly where the existing tests promise
exact bytes; 50% and 75% intentionally alter scene sampling and are not expected
to equal the full-resolution world pixels. Record actual receipt scale and
attachment size, not just the requested config. Test odd output extents (for
example 959×539), offset/fractional camera viewports, orthographic and perspective
projection, scene clipping/scissor boundaries and unchanged physical output size.
Internal normal target dimensions are ceil(output×scale), while all scene viewport
coordinates use the same exact binary scale. Never derive camera aspect from the
independently rounded target dimensions.

Include screen-space text and interactive UI through the canonical composed host.
Validate layout boxes, focus/hit coordinates, clipping and output dimensions remain
physical-size. Use independent numeric UI state/geometry checks, with opaque UI
regions for exact pixel controls where appropriate; transparent UI composites may
vary with the scaled world beneath them. Include an authored pixel-perfect camera:
its existing logical target, integer mapping and letterboxing must bypass adaptive
scaling and must not drive feedback. Verify unchanged frames reuse the same target
size and existing preparation catalog; do not claim all GPU/driver allocations or
complete frame encoding are allocation-free.

Exercise actual native queue completion through the public composed route. The
clock starts before render preparation and ends when
`Queue::on_submitted_work_done` invokes its callback. Record it as completion
latency including CPU preparation, queue backlog, presentation blocking and
callback-dispatch delay. It is not isolated GPU time, refresh time, whole-game
frame time or a benchmark. Do not use simulated numbers as actual callback proof,
add a production wait/query feature, or tune the workload until a performance
claim appears. In a separate bounded adaptive run, retain the real delivered
measurements and prove decisions follow the configured deadband/streak/cooldown
and slew limits; a stable scale is a valid result if those measurements warrant it.

Verify these lifecycle cases using actual submissions/callback dispatch plus
explicitly identified failure injection at the host boundary:

- Normal delivery is consumed once, and at most one callback or completed result
  is retained while subsequent accepted frames proceed.
- Delayed delivery across viewport resize or scene/world replacement cannot update
  a different owner or the new extent epoch. Old receipts are discarded.
- Failed staging/encoding produces no new accepted receipt. Acquisition or
  presentation refusal remains terminal under the existing host contract; a later
  callback cannot resurrect it. Distinguish submitted-but-failed-presentation work
  from an accepted presented frame.
- Dropping a host or replacing its telemetry owner with work outstanding cannot
  publish into another host. Recreating a host starts fresh. The completed-ticket
  destructor must not erase a subsequently reserved slot.
- Foreign, duplicate, stale and invalid-current telemetry tests preserve the
  accepted scale/owner according to the typed contract. CPU injection tests are
  separate evidence from native callback execution.

Do not infer delivery from a sleep alone. Retain actual sequence/epoch/scale
observations and dispatch/consumption outcomes. Reuse existing test-only failure
providers and GPU test infrastructure. If a missing observer requires a new
engine hook, report the precise cfg(test) seam to Astra; no production bypass or
unbounded trace. Browser automatic timing remains explicitly unsupported.

## Checks and bounded appearance review

Build once from the grant's frozen source using locked dependencies, ci profile
and at most two jobs. Preserve its agreed feature set for every comparison;
fixed-scale versus disabled controls are runtime policy differences, not different
shader builds. Retain source, Cargo compiler-artifact features and copied binary
hashes before execution. List the current renderer library/model_gpu and GPU-host
game/UI tests. Execute all applicable ignored GPU cases serially; the reviewed
base has 89. Explain any integrated-source count change and list actual pass,
fail, skip and refusal outcomes. Preserve failures instead of replacing evidence.

Add focused actual GPU numerical tests for the contracts above. Observe a few
complete matched frame sets at fixed 50%, 75% and 100%, including an offset
viewport with UI and the pixel-camera bypass. Prefer 3–6 comparisons. Inspect
unmasked complete frames for aspect, clipping, chrome/UI integrity and the real
nearest-sampling quality. Do not turn intentional lower-resolution differences
into a false exact-parity assertion. Keep numerical and appearance verdicts
separate. No desktop capture or unapproved scene/style changes.

## Ownership, grant and delivery

Only root Astra can create
`/Users/andreyyurlov/.codex/incant-evidence/0067/review-ready.json`, naming this
packet, exact source, exclusive GPU grant and target release after 0065/0066.
If absent, preserve preparation and stop; do not poll or acquire a lease.
The grant must confirm the target explicitly. Expected target, only after release,
is `/Users/andreyyurlov/.codex/incant-core-presentation-target`; never use Astra's
`/tmp/incant-dynamic-resolution-target` or root's active compiler targets.
Use a root-approved isolated child checkout. Evidence belongs under
`/Users/andreyyurlov/.codex/incant-evidence/0067`.

Hold the exclusive GPU lock. Run one bounded process group at a time: five minutes
per GPU suite, two minutes per additional capture/control process. In addition
to existing regression-suite frames, allow at most 48 new numerical feedback/
lifecycle submissions and 24 additional appearance frames, each at most 960×540.
Freeze case names and budgets before execution; report a genuine missing bound
rather than repeatedly extending a stalled run. Offscreen engine frames only,
no native window, desktop/browser capture, audible playback, credentials or
accounts. Root can separately authorize a native-window follow-up if necessary.
Recheck source/binary identities for each child. Terminate and reap timed-out
process groups, then release the GPU and target with all owned PIDs confirmed
absent. Do not leave a background callback test or capture process running.

Return result.md, reproducible packet-only helpers/tests, raw numerical receipts,
source/binary identities and a small unedited appearance selection. Record actual
OS/adapter/backend and verified model/effort. Commit with Built-by: claude; no
pushes, PRs, main changes or CI edits. New test code is reviewed by Astra before
integration. This packet does not approve reference-phone frame-time/memory gates,
Vulkan/DX12 execution, temporal history, MetalFX, browser timing or default enablement.

## Root execution clarification, 2026-10-13

0066 completed its bounded Metal review and explicitly released the GPU and warm
target. Its extra raw cluster-buffer observer remains separate Astra follow-up;
this does not prevent the independently accepted DRS source review. Read the
new grant and independently verify its release and target-lease hashes.

Never persist the inherited environment, credentials or ACP transport variables.
Child receipts may record only an explicit nonsecret allowlist of build/renderer
settings. Reuse the environment-safe 0066 or 0064 logger, never the old0065 logger.
The 24 appearance-frame budget includes every new render in the appearance
helper, including repeated dry runs and controls, not just saved PNGs. The48
numerical submissions are a separate hard budget; keep the existing regression
suite executions separately counted. Prepare the exact case ledger before use.

If any required host observation cannot be expressed through existing public
contracts, leave a precise request file and continue independent preparation.
Do not silently relax numeric or lifecycle acceptance. No extra GPU execution
after the release latch; return missing evidence for a later bounded packet.
