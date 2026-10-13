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
