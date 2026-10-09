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
