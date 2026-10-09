## PRIORITY 5 — review revision 4 in the actual native app

Your redesign is integrated as b99c520; Astra's nonvisual interaction corrections
are integrated as 3a7f7ff and cherry-picked into this handoff worktree. Review the
new native captures `artifacts/native-review/n*.jpg` now. Read the revision_4
section of docs/spikes/evidence/editor-assets-native-2026-10-09.json for the actual
observed behavior and capture names. All 276 UI tests and the native build pass.

Astra corrections: Import/asset opening reveals a hidden Inspector before focus;
Escape can reveal a hidden left panel; failed-reimport Show restores the correct
asset (and opens the Inspector); folder labels use distinct IDs for punctuation
collisions. No CSS, layout, visual copy or appearance was changed by Astra.

Review wide and minimum native spacing, clipping, grouped folder rows, texture
and model properties, pinned import action, success/error/busy states, focus rings
and titlebar. N1 minimum and N2 minimum are both supplied by n01b-assets-min.
N3 has wide and minimum form captures plus real batch success. N4 has a real
8-image worker and real malformed glTF error, wide and minimum. N5 Enter/Escape
behavior passes and its heading ring is captured. N6 output-only placement passes;
entity-linked Problems reveal only has the automated interaction test because the
native host does not emit those diagnostics yet. Do not fabricate a native pass or
ask for invented errors. Record that exact limitation; it does not block this UI.

Update result.md and native-requests.md with evidence-specific verdicts; commit
Built-by: claude. Fix any genuine visual defect found, and request a targeted new
capture only if needed. Do not redesign again unless evidence warrants it. Keep
raw native captures/account labels ignored and unquoted. ACP transport/model are
unchanged. Astra owns PRs, full checks and merging after review.

## PRIORITY 4 — director rejects the bottom asset UI; rethink the approach

The director's new feedback supersedes all previous visual approvals. Exact feedback:
“I dont like this UI at the bottom. Looks messy, unprofessional, bad spacing. And
I dont even think the assets belong in that bottom sections where the compiler
issues are presumably right next to it. Have it rethink the approach.”

The director's supplied screenshot is available privately at
`artifacts/native-review/director-rejected-bottom-dock.png`. Read it first. It
contains an account label; keep it ignored and never quote that label in reports.

This requires a substantial layout and interaction rethink, not another small
spacing patch. Move asset browsing out of the Problems/Console/History output
dock into a coherent, dedicated place in the editor. You own the design choice
and implementation. Work out where content navigation, the asset list, details
and import/reimport belong together, how they relate to hierarchy and the scene
viewport, and how users enter/leave that workspace without confusion. Keep
Problems, Console and History as output/diagnostic/history tools. Avoid cramming
a table and a second mini-inspector into a short bottom strip. Fix density,
alignment, spacing, visual hierarchy and the persistence/placement of notices.
Preserve the connected neutral charcoal shell and restrained accent palette.

Keep all working backend behavior: real batch imports/reimports, stable IDs,
texture interpretation, atomic Undo/Redo, path validation, retained drafts/errors,
concurrent editing, typed asset.conflict, native text/menu Undo and account safety.
Do not invent thumbnails, file pickers, model placement or unavailable operations.
Use the space thoughtfully at wide and minimum native sizes. Reorganize the shell
and navigation as necessary within editor/ui/**; Astra owns native/backend logic.
No need to ask the director which placement to use. Implement your strongest
approach, review the actual browser pixels, and supply precise native requests.

PR #8 will remain unmerged while this redesign is incorporated. Continue your
existing ACP session on this worktree. Commit completed UI with Built-by: claude,
update result.md and native-requests.md, and retain honest evidence. Avoid
CSS-source-mirroring tests. Run UI behavior tests and production build; keep the
bundle within its established budget. Private captures stay ignored. Astra is
continuing runtime logic separately and will integrate your UI into PR #8.

## Priority revision 3: final native re-capture review

The revision 2 commit is integrated as 08026d8, rebuilt and tested (267 UI tests).
New private CUA captures are in artifacts/native-review/r*.jpg. Review R1–R4 now:
- r01-busy-copy-wide.jpg: actual eight-PNG worker, on the external display.
- r01b-busy-copy-narrow.jpg: actual eight-PNG worker, minimum laptop placement.
- r04-heading-focus-narrow.jpg: also supplies R2 tabs, seven? history count at
  capture is authoritative; normal texture heading focused by Enter.
- r03-reimport-result.jpg: after scrolling texture details down, selecting Linear
  and reimporting; native AX confirms Reimported and history increment.
- r04-heading-focus-wide.jpg: restored wide size, Escape to row then Enter.
Native R5 both cases PASS: select-all then multiline paste replaces Path 1 and
adds Path 2; at end caret it keeps Path 1 and inserts new rows below. Source paths
were synthetic and no invalid paths submitted. Account stayed connected across
this new build too. Finish pixel review, update result.md and native-requests.md
with precise verdicts, commit with Built-by: claude. Only change implementation
if evidence shows a remaining issue. Keep private captures ignored. No need to
repeat all browser tests if only review docs change. Astra will handle PR/merge.

## Priority revision: native integration review, 2026-10-09

Your original UI commit is integrated. Review the native captures now in the ignored
`artifacts/native-review/` directory in this worktree. These were captured by Astra
through Codex CUA on the actual rebuilt Tauri/WebKit app. The Mac is unlocked.
Read `docs/spikes/evidence/editor-assets-native-2026-10-09.json` for observed behavior.
Astra owns behavior assertions; you own pixel review and any visual corrections.

- Fix the busy sentence “Other edits wait until it finishes.” Normal document edits
  now succeed while cooking; a concurrent edit makes the import fail with an explicit
  retry hint. Keep copy short and accurate. Supersedes the old contract below.
- Native errors now preserve `asset.conflict`; UI detects that code with a legacy text
  fallback. Preserve those integration changes.
- Review all native captures, including account-keyboard-focus.jpg from the previous
  rebuilt bundle. Inspect wide/narrow asset layout, focus rings, selected texture
  controls, error readability, titlebar/traffic lights, inspector paths and clipping.
  Some native AX lists expose only the first row though keyboard navigation works;
  inspect whether the pixels actually show all rows correctly.
- Native minimum layout was obtained with Window > Move & Resize > Bottom Right;
  restored wide layout with Return to Previous Size. Report actual image dimensions
  and whether traffic lights are visible or still obscured by the capture indicator.
- Consider the observed multiline-paste interaction: selecting all text in a nonempty
  path and pasting multiple lines appended rows while retaining the old selected path.
  Decide whether replacing the selected current path is more usable and implement
  that behavior with an interaction test if appropriate.
- Do not restore deleted CSS-selector source-mirroring tests. Integrated suite is
  265 passing tests; use behavior tests and pixel evidence.
- The previous report’s “not ACP” claim was incorrect. This session is dispatched
  by tools/handoff/main.py using AcpClient, protocol 1, model claude-opus-5-5.
  The runner controls the outer transport; no routing exception is needed. Retain
  the corrected verified transport section in result.md.
- Keep raw native screenshots/account labels in ignored artifacts only. Never
  copy them into tracked screenshots or quote real account labels in reports.
- If fixes need new native captures, leave precise requests for Astra. Continue
  browser review yourself. Commit fixes with Built-by: claude; no PR/merge.

# Editor asset library and import workflow

Use Claude Opus 5.5 through ACP. The director wants polished, clean, modern visuals
and asked Claude to keep improving the editor while Astra implements logic.
Preserve the connected charcoal panels, restrained accent palette, native custom
titlebar and established spacing. You own the interface, layout, visual states,
icons and rendered-pixel review for this feature. Make it feel designed as part
of the editor. Do not redesign the landing page in this packet.

Computer use/capture is permitted again. Your ACP session has no native CUA.
Browser fixture review is allowed. Request native captures/input from Astra in
native-requests.md; Astra uses Codex CUA only. No screencapture/CGWindow helpers,
AppleScript, synthetic native input, credentials, account actions or external
publishing. Private native captures/account labels must stay out of Git. The Mac
was locked at the last native check; do not claim browser evidence proves native
behavior.

## What now works

Astra has added a real native asset-import command on this branch. Existing saved
projects can import static glTF/GLB and PNG/JPG/JPEG/EXR from their project folder.
Preparation runs in a worker without holding the document lock. An entire batch
commits atomically through shared history, can be undone/redone, and preserves
stable IDs, user names and saved texture usage on reimport. It rejects concurrent
project changes and broken batches with a real error. There is no file picker,
source copying, drag/drop, thumbnail renderer, cancellation or GPU/ECS binding yet.
Do not add controls that claim those exist. The actual available operation is
importing existing project-relative source paths and reimporting listed assets.

The public contract is editor/bridge/contract.ts (re-exported to UI):
- snapshot.assets?: LoadState<readonly ProjectAsset[]>; absence means an older
  host does not expose assets. Loading/error must not claim an empty library.
- ProjectAsset: id, name, path, kind, fingerprint, optional textureUsage.
- snapshot.assetImport?: { available: boolean; reason?: string }. Imports are
  unavailable for the default unsaved startup project; show its actual reason.
- capability 'asset.import'. Document command:
  { type: 'asset.import', sources: [{source: 'models/prop.glb'},
    {source: 'textures/rock.png', textureUsage: 'normal'}] }.
  UI Shell.run dispatches this and returns a boolean, announcing exact failures.
  The bridge returns BridgeResult with the failure message if needed inline.
- 1–64 sources per batch; canonical forward-slash paths relative to the project
  file. No absolute paths, parent traversal, URLs or repeated separators. Files
  must already exist inside the project. No account is required.
- For images, usage is color/linear/normal. Omit textureUsage to preserve a saved
  import setting (new images default to color); never send usage for model files.
- Await completion. The bridge publishes the real new asset list and history.
  Failed imports retain the previous list. A single pending bridge mutation is
  allowed; navigation and reads remain responsive while cooking. No fake progress
  percentages or enabled cancellation. Conflict errors can be retried explicitly.

## Scope and acceptance

Design and implement a usable asset browser and import/reimport flow within the
existing editor shell. Choose placement and interaction details yourself. Users
should find assets, understand name/type/source, import source paths, change image
interpretation when appropriate, and reimport an existing asset without typing
its path again. Avoid walls of instruction text; give enough guidance to make
project-relative paths understandable. Keep errors visible and preserve input on
failure. Do not claim imported models already appear in the viewport.

Handle populated, empty, loading, unavailable, in-progress, successful, failed,
long-path and narrow-window states. Keyboard use and visible focus must work;
retain F2 rename, project/text Undo separation and account-dialog safety. Keep
button labels and explanations precise. Do not expose fingerprints prominently
unless they help the user. Treat filenames as untrusted text.

Edit editor/ui/src/** and associated tests/docs. Astra owns editor/app/**,
editor/bridge/native.ts, native.test.ts and native.generated.js. Read those for
behavior. Do not modify the contract without first recording the needed change
for Astra. The minimal new capability label in UI resolve.ts is already present.
Add only explicit, labeled read-only browser fixtures for pixel review; fixtures
must never masquerade as a working engine or a second project mutation model.
Behavior tests can use a controlled bridge that verifies actual dispatches.

Run npm test --workspace editor/ui and npm run build --workspace editor/ui. Review
browser pixels at 1000x650 and 1440x900, including populated and failure/busy states.
Keep synthetic fixture screenshots, accessibility evidence and the exact model,
commands and measured bundle delta in result.md. No new dependencies, remote
fonts or icon packages; keep the main UI under 110 KiB gzip (currently ~93.57).
Use the pinned local Playwright browser and a separate port from the user's site
at 4176. Existing handoff 0009/0008 browser tools can be reused.

Commit completed UI with Built-by: claude. Return result.md and any specific native
requests. Do not claim complete feature integration or native verification until
Astra reviews/tests the combined implementation. Astra handles PRs and merges.
