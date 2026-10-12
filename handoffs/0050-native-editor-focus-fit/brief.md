# Focused native editor verification after 0048

Use Claude Opus 5.5 through ACP with Max explicitly verified before prompting.
This is a bounded follow-up, not a new app redesign or provider evaluation.
Read AGENTS.md, editor/ui/DESIGN.md and 0048/result.md. Root reviewed and merged
0048's two source fixes, ran all 904 UI tests, strict types, production build,
bridge check, and rebuilt the actual editor against main104.

## Exact grant

Read build.json and verify its immutable manifest and editor hash before launch.
The editor is /tmp/incant-native-review-0050-bin-v1/incant_editor, built from
4589c2ba5edf2d89587fef8b815bdca1cd4af01e. Do not replace or rebuild this binary.
The previous 0048 captures do not verify these new fixes. The numerical shader
comparison has released GPU ownership; you exclusively own native window/GPU
review until cleanup. Do not run unrelated render probes.

Reuse /tmp/incant-native-0048-fixtures/saved/project.json, project ULID
01M4PEJQNPMDQYEDGT0PRWN78X. It has the saved account, 64,000 allowance and 22,044
charged tokens after the one completed turn. **No provider turn is authorized in
this packet.** Do not send a prompt, sign out, change accounts or open account
dialogs. No credential/keychain reads. The numeric usage ledger may be verified
read-only by its exact known path. UI project edits, if necessary, use the normal
command bus; do not edit project/journal files directly.

## Acceptance

1. In the actual WKWebView, select Main / Stationary Camera, focus the scene/camera
   picker, press Command-P, and confirm focus continues at the Play control
   (Pause while running). Stop and repeat from another panel: that existing
   focus must stay there. Also exercise the no-camera refusal and keyboard
   Dismiss once, retaining a usable focus target.
2. Verify the composer shows the entire default gpt-6-astra model string in the
   native field at both 280 and 380 px side columns, including 1000x650 and the
   largest available window up to 1440x900. Check the extra 2 px causes no
   surrounding overflow. If checking a longer model value, do not send it and
   restore the default afterward. Record exact dimensions and native evidence.
3. Confirm the rebuilt executable retains the same signed-in account and usage
   without another login or network request. No account identity in captures or
   published evidence. Sampled socket checks are not proof of zero traffic.

Keep this review short: at most six necessary scoped captures, including a crop
if useful. Reuse and adapt 0048's owned-window helpers to this new binary hash;
all helper source lives in this handoff worktree. Check foreground ownership
before every event, pause when the user changes apps, and restore prior app and
cursor after short bursts. Never capture another app or the desktop.

If a check fails, record the defect and make only its necessary UI correction,
with meaningful interaction checks, strict TypeScript, full UI tests at max4
workers and production build. Root owns backend changes. A new correction needs
a subsequent granted binary; never claim this binary contains that correction.
Do not expand into 0048's nonblocking layout observations or a broad screenshot
catalogue. They remain separately tracked.

Return concise result.md with model/Max, exact build identity, actual outcomes,
reviewed captures, process IDs and cleanup. Quit all owned editor/helper
processes and explicitly release native/GPU ownership. Preserve failures and
limitations. No cross-platform, device-performance, full accessibility or phase
gate claim. Commit with Built-by: claude. Do not publish or merge.
