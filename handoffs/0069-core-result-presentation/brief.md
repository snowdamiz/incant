# Priority revision: execute the accepted generation-5 GPU-only review

This revision supersedes the earlier preparation-only restrictions for this invocation.
Use the existing worktree. ACP must verify Claude Opus 5.5 and thinking Max before
prompting; confirm the verified header before running. Root reviewed the GPU-only
wrapper at 0351d93e and its seven CPU guard tests and exact-input preflight passed.
0073 released the GPU at 08:23:36Z with every child reaped. You now own the sole
GPU interval for this packet. Do not build, recook, edit source, or use a Cargo target.

Read gpu-proposal.md and cpu-result.md. Root authority is
/Users/andreyyurlov/.codex/incant-evidence/0069-gpu-review-ready.json
with SHA256 6b8064c83fefc0b0b51e752809603bc012f7f84cfe9680082dab3648b82f987e. Wrapper gpu_only.py SHA256
e3530e5d51c4a09d6f4b3f803b1c5aaddfd93c2d32f35f5e489c0a6bb464ae71. Root review is
/Users/andreyyurlov/.codex/incant-reviews/core0069-gpu-wrapper-root-20261013.json.

Run once, in the foreground (no background process or automatic retry):

    python3 -B handoffs/0069-core-result-presentation/gpu_only.py run --evidence /Users/andreyyurlov/.codex/incant-evidence/0069-gpu-001 --authority /Users/andreyyurlov/.codex/incant-evidence/0069-gpu-review-ready.json

This runs the eight exact GPU regressions (39 tiny frames), then the ten exact
capture processes (22 appearance frames). The wrapper releases on every outcome;
verify its release record and no owned processes before returning. A failure stops
this attempt; retain it and report, without changing gates, sources or authority.
After a successful run inspect every retained image yourself, including narrow,
RTL, CJK, pseudolocalized, win/loss/restart states and text contrast. Appearance
is your judgment. Give a clear scoped verdict; do not substitute numerical passes.
Write gpu-result.md and a compact provenance manifest linking immutable external
reports and the CPU packet, and commit only your new packet evidence/result with
Built-by: claude. Preserve all prior failures and original CPU files. No push/PR.
If repairs are needed, describe them and stop after release; root coordinates a
fresh source/build interval. No desktop capture, native window, device audio,
persistent memory, other checkouts, or unrelated edits. No phone/performance or
phase-gate claim. Report source, frames, regressions, images, release, and limitations.

---

# Priority revision: finalize prepared halo source for CPU review

Root has read the prepared halo implementation and renderer/paint integration;
no source-correctness blocker was found. Finalize the following small items and
commit your owned source/helpers/evidence with Built-by: claude. No Cargo/GPU
or target grant exists yet; do not wait for one. Preserve current source scope.

Answers: use non-waiting global GPU lock acquisition (a busy lease refuses),
not600seconds. Root schedules GPU ownership explicitly. The new2-frame halo
regression may use the separate128-frame regression ceiling, but identify it
as newly added coverage, not an existing historical test; totals must include
it. Keep current padding for the first bounded visual evaluation and use your
judgment after inspecting the captures; root does not choose appearance.
Do not claim identical prepared bytes on every platform without cross-platform
execution evidence. Document the bounded f32 algorithm and existing local
oracle tolerance instead; actual cross-platform equality remains unverified.

For the forthcoming CPU-only grant, parameterize/assign this packet TARGET to
/tmp/incant-agent-font-target (currently idle and reserved by root for0069).
Do not access/build it yet. 0072 owns incant-core-presentation-target. Re-run
only affected synthetic lifecycle/configuration checks, no broad redesign.
Produce the exact final source/helper commit and focused changed-path hashes.
Root will supervise long CPU builds directly to avoid ACP background jobs
being lost at the end of an ACP turn. Preserve current prepared result and
append precise revisions rather than altering historical failures.
Model Claude Opus5.5 with thinking explicitly Max through ACP remains required.

# Priority revision: complete performance and runner corrections before any grant

Root read the updated source/helpers. The prior five findings are closed by source inspection, but the new review at `~/.codex/incant-reviews/core0069-prep-second-20261013.json` requires correction. Read it now. The halo is25 texture reads PER minification tap, up to400 per fragment, not25perpixel: avoid this global label cost. Keep your visual direction but choose a bounded lower-cost approach, preferably cached/prepared glyph halo coverage outside the rendering hot path. Normal glyph coverage must retain its current contract; all source/layout/cooked implications need explicit tests and documentation. Do not silently swap algorithms or make mobile claims from CPU checks. You may change the necessary glyph-atlas/runtime preparation source for this scope; prepare the concrete approach and implement it, CPU source review only before grant. Also add the global native GPU lock, close release-versus-launch race, and make released ledger hashes stable. Preserve existing result as previous-preparation-result.md, write new prep-result.md; do not overwrite historical evidence. Existing UI GPU regressions can have a separate bounded <=128tinyframe debit (derive actual count from source), in addition to24new appearance frames. Still NO Cargo/shared target/GPU until fresh root grant; CPU parser/lifecycle synthetic controls permitted. Current target belongs to0068. Model Opus5.5 Max remains required.

# Additional review received — 2026-10-13

The full independent helper review is now in `root-helper-review.json` next to
this brief (root-owned). Read and close all five findings before asking for an
execution grant: interrupt cleanup, unstripped porcelain -z source identity,
review frame provenance, build-to-capture frozen identity, atomic frame debit.
This source review does not decide appearance. Scope remains preparation only.

# Priority revision — 2026-10-13, follow-up preparation

Read this before all earlier instructions. Your preparation result is received.
Continue on the existing handoff checkout, Opus5.5 with explicitly verified Max.
This invocation is still PREPARATION ONLY: no Cargo/build/target/GPU/captures.
No grant exists. Do not wait or poll for it. No personal/auto/persistent memory.

1. Choose the design yourself using the director's full creative authorization.
Do not settle for moving a contrast failure from the result title to the stats.
The source inspection demonstrates a missing legibility capability. You now own
visual implementation within `crates/incant_render/src/ui/theme.rs`, UI geometry
and the associated UI draw shader if necessary, plus existing visual tests.
Prepare a coherent fixed-theme legibility treatment which works against both
bright and dark game backgrounds, preserves useful text and fits narrow/RTL/CJK
layouts. Avoid decorative glow. Astra will review bounds, clipping, resource and
shader correctness before builds. A new authored style field or schema/runtime
mutation path still needs Astra implementation; if that is necessary, specify
its exact semantics. Do not change gameplay, camera, physics, audio or test oracles.

2. Astra fixed the optional prompt fixture in commit f5eba1d7. All20 existing
binding/playtest tests now pass with your proposed documents. Preserve that commit.
`verify_bindings.py prepare/ship` is the intended public route for the authored
prompt. The prepared geometry remains your draft and can be improved as needed.

3. Read-only access is authorized to the hash-pinned font downloads and original
art cache under `/Users/andreyyurlov/.codex/incant-evidence/0060`; preserve original
assets/provenance. Choose appropriate Chinese wording, not a missing-glyph dodge.
Later granted execution may regenerate the subset reproducibly and verify glyphs.

4. Independent correctness review found two blockers in draft helpers: `lease.run`
only reaps on timeout, while KeyboardInterrupt/SystemExit can leave the session
child alive; all outcomes need finally-owned process-group kill and wait before
unlock. Also `capture.reuse` trusts any same-freeze review receipt and suppresses
names without proving matching variant/route/viewport/state frames actually ran.
Prefer removing reuse complexity and rendering the full22-frame plan within24,
or bind reuse to exact successful report/frame/plan/binary hashes. No fabricated
review acceptance. A complete independent receipt will follow; source may be
repaired now but no real commands may bypass the grant.

5. Use a constructed minimal environment for compiler/child execution. Never log
inherited environment or ACP/session values. Include bounded synthetic lifecycle
controls for normal/nonzero/timeout/interruption and prove no surviving PID/group;
these may use temporary files strictly inside this packet's tools/test-scratch
and must clean them up. They must not build, use GPU, touch the target or grant.
No live engine/test invocation until reviewed. Keep frame accounting conservative:
all rendered attempts count, including failures and unretained frames.

6. The source+binary review must confirm one actual accept per capture and no
GPU device on layout-only routes. Fresh rendering is suitable for the current
non-temporal Core sample, but do not claim equivalence to an accumulated temporal
history. Snapshot/routing must consume actual public render/UI inputs. Keep any
history-sensitive cases explicitly separate.

Return updated source, helpers/case list and a short delta result with outstanding
correctness review. Do not ask the director routine design/permission questions.
The grant will use `packet` and `target` plus exact reviewed source/helper and
predecessor-release identities; root writes it only after review.

---

## Priority: prepare on the reviewed main196 integration

The latest source now includes main196 (`3da8b0ebeab43f715b0531603e30a864e801be11`),
including merged Core result/restart support. Sample source is byte-identical to
the prior reviewed Core producer. This invocation is preparation only: read the
packet and existing limited appearance evidence, prepare the concrete visual
proposal and bounded safe helper/case plan, and return for root's correctness
review. No builds, Cargo target use, GPU execution or captures are authorized yet;
0068 owns the GPU and shared target. Do not poll or wait for its grant.

Use Opus 5.5 with explicitly verified Max effort. Stay in this handoff worktree.
Do not create or update personal/persistent memory, Claude auto-memory, external
notes or other checkouts. Final visual asset edits may be prepared here, but do
not claim visual acceptance before the later granted execution. Preserve source,
fixture and historical evidence; no engine or gameplay correctness edits.

# 0069 — finish the Core Sample result and narrow HUD presentation

Use Claude Opus 5.5 through ACP with thinking explicitly verified as Max before
every new or resumed session. Read CLAUDE.md. Claude owns composition, copy,
styling, fonts and rendered-pixel judgment; Astra owns engine correctness.

This packet is queued. No builds, GPU work, screenshots or shared target access
until root writes `/Users/andreyyurlov/.codex/incant-evidence/0069/review-ready.json`.
Do not poll for it or bypass ownership through a different target. If started
without a grant, prepare a concise plan/helper proposal and return; root resumes
the same session when ready. No desktop capture, native window or device audio.

## Exact source and remaining problem

The reviewed base is `16656913b3ea6ee6aa422875af4dcf1f0c6aca56`, including the
sanitized 0065 presentation and Astra's result-binding correction. Read
`docs/spikes/core-result-hud.md`, `samples/core/PRESENTATION.md`,
`handoffs/0065-core-live-hud/result.md`, and the sample's binding/oracle helpers.
Root may qualify a newer integration before granting execution. Never silently
reuse a binary whose complete relevant source/tool inputs differ.

0065 found weak title contrast over the horizon, weak narrow-width contrast,
a keyboard-only legend, a missing result-state prompt and stale win objective.
The last two now have reviewed engine/script support: the optional `hud.prompt`
label is inactive while playing, appears with either result, and returns to its
initial state after restart; the objective clears on a win. Its localized message
key is `hud.prompt`. The ordinary sample does not yet author that optional label.
The existing 13-widget correctness fixture is deliberately not an approved layout.

Implement the finished presentation using the supported binding. Give the player
an immediately understandable result and restart action, including the working
gamepad Start action. Improve readable contrast at normal and narrow sizes and
in Arabic, Simplified Chinese and pseudo-localization. Keep the professional,
restrained direction: clear hierarchy, measured spacing, little decorative
chrome, no gratuitous glow or ornamental borders. Choose the visual solution;
do not merely preserve a compromised position to avoid changing the layout.

## Boundaries

Primary editable paths are `samples/core/ui/`, sample font assets/provenance and
`samples/core/PRESENTATION.md`, plus packet-only helpers. Preserve stable IDs,
explicit role bindings, gameplay, physics, camera behavior, sound and timings.
All actual project construction and edits go through the existing atomic
`incant_cmd` API. Do not create a second project mutation path.

Use existing supported UI/background/layout facilities. If a required renderer
capability is absent, return the exact capability and minimal reproduction for
Astra instead of inventing unsupported fields or changing renderer code. Do not
edit the controller, binding/oracle logic or relax tests. Font changes must retain
their licenses and reproducible subset provenance and cover every authored glyph.
Never alter historical screenshots, source freezes or acceptance receipts.

Read `samples/core/verify_result_hud.py` and `verify_bindings.py` to understand the
public authoring/native/ordinary-export contract. Root already verified the code
with both 12- and 13-widget layouts, including fresh standalone R/Start/held-input
restarts and exact state/PCM comparisons. That proof does not automatically cover
your changed authored assets. Run the relevant CPU authoring, binding, locale,
font and source-free export checks for your final source. Distinguish fresh runs
from immutable relevant-source reuse; preserve failures and all original evidence.

## Review and resource budget

Freeze source, helper, binaries, feature graph, OS and GPU/backend before running.
Use the granted `/Users/andreyyurlov/.codex/incant-core-presentation-target`, locked
dependencies, the ci profile and at most two compilation jobs. One managed
process group at a time, with a two-minute GPU/capture limit and a five-minute
existing-regression limit; terminate and reap children on every failure path.

Before rendering, submit a fixed case list that covers normal and narrow playing,
won/lost and fresh restart states, plus Arabic, CJK and pseudo-locale readability.
At most 24 newly rendered appearance frames, each at most 960x540. Every baseline,
comparison, dry-run or discarded frame counts, not just saved PNGs. Prefer reusing
reviewed CPU routes and rendering only selected states. If existing helpers submit
more frames, adapt the packet helper and request correctness review before a run.
No repeated oversized exploratory capture batches. Keep the numerical route,
restart, HUD state and PCM checks separate from your visual verdict.

Record only explicit nonsecret environment overrides from a small documented
allowlist, such as CARGO_TARGET_DIR and INCANT_NATIVE_PROJECT. Never serialize an
inherited environment, ACP transport value, credential or account metadata. Use
the sanitized logger pattern; do not copy the original 0065 environment logger.

Return result.md with the verified model/effort, precise changed paths, source and
binary identities, complete numerical outcomes, honest contrast/readability
judgment and a compact unedited screenshot selection. Release the GPU and target
explicitly with no owned processes remaining. Commit with `Built-by: claude`;
do not push, open PRs or change main/release. This packet does not approve a
reference-device, performance, complete-game or phase gate.
