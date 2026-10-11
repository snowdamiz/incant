# 0036 final isolated native editor acceptance: result

## Status

- **Done.** The plain editor was reviewed natively on the exact prepared binary of 28e5491, in its
  sandbox, at 1440×874 (normal), 1000×650 (minimum) and a 280 pt Inspector (narrow).
- **The director's five requirements hold natively.** Every area the packet lists was observed through
  the native UI, with keyboard focus checked after each transition (table below).
- **The 28e5491 focus fix works natively:**
  - after a successful import, focus lands on the reset Path 1;
  - after a failed import, it stays on Path 1;
  - after a reimport of an atlas, an image, WAV or OGG, it moves to the asset heading.
- **Four new findings for Astra (N1–N4), all in code that predates this source.** The most important
  is N1: a reimport that changes nothing still says "Undo reverts the whole reimport", and Undo then
  reverts an unrelated earlier change. I reproduced this natively.
- **Browser findings 3–10 are unchanged.** Findings 3–8 look the same natively. Finding 9 measures
  about 4.4:1 natively, against 3.35:1 in the browser.
- **No source, style, Cargo or account change.** Nothing was pushed, merged or published. No phase
  gate, game, runtime or performance claim is made.

## Model and effort

- Model: Claude Opus 5.5, model ID `claude-opus-5-5`, as the session's system prompt states.
- Thinking effort: Max, as the packet requires and the runner states. The session does not show me
  its effort setting, so I cannot verify it myself. The ACP runner verifies both settings.
- No substitution.

## Identity

| Item | Value |
| --- | --- |
| Reviewed source | `28e5491ab317019b050bf58b8819e580f796dd33`. Presentation: `65e17177…`. Branch head: `3ff9c1a`, which adds only `STATUS.md`, `docs/spikes/editor-release-union.md`, `docs/spikes/evidence/editor-release-final-2026-10-11.json` and this brief. The `editor` tree (`a1c1f74a…`) and `crates` tree (`e9e2c5fa…`) are identical at 28e5491 and the head |
| Binary | `/private/tmp/incant-native-review-28e5491/Incant Review 28e5491.app/Contents/MacOS/incant_review_28e5491`. SHA-256 `51599f65f3e96458d706a81c81b71bc392e93812923f11549e4ae0bd3024b79e`, which matches the packet and `fixture-provenance.json`. Ad hoc, linker-signed, Info.plist not bound |
| Bundle | `Incant Review 28e5491.app`. Its Info.plist names `dev.incant.review.28e5491` |
| Bundle ID as registered | None. Launched by exec under `sandbox-exec`, LaunchServices registered the exact bundle path, the name "Incant Review 28e5491" and "sandboxed", but no bundle ID. Every input therefore checked the kernel executable path, the registered bundle path and the bundle's on-disk ID |
| Launch | `tools/launch.sh` checks the hash, then runs `/usr/bin/sandbox-exec -f /private/tmp/incant-native-review-28e5491/review.sb <executable> /private/tmp/incant-native-review-28e5491/fixture/review.incant.json`. `open` was not used |
| PID | **64415**, started 21:14:46 EDT. Window 29601, title "Incant" |
| Sandbox profile | SHA-256 `4aa86157…`. It denies network and file access to `~/Library/Application Support/Incant` and to `protected-probe`. The profile was unchanged |
| Isolation, observed | The app reported "provider: Could not create Incant account directory" (`provider.auth`). It never reached the shared store |
| Stop | Before quitting, I re-checked the PID's kernel path and bundle. The normal quit request worked, and no SIGTERM was needed |
| Machine | Mac17,9 (Apple M5 Pro, 48 GB), macOS 26.6 (25G72), WebKit 21624.4.5.11.5, US keyboard layout. Display 1512×982 pt; its visible frame is 1512×874; scale 2 |

## Isolation and input safety

- **Targeting.** `tools/native.swift` is a PID-scoped helper, built with Swift 6.4 into the ignored
  `artifacts/`:
  - keyboard events go to PID 64415 only, through `CGEvent.postToPid`, never the global event stream;
  - accessibility actions use only that app's element;
  - captures use `screencapture -l 29601`: that window alone, never the screen.
- **Frontmost check.** Before every key, character, press, focus, menu item or resize, three sources
  had to name the review app:
  - the NSWorkspace frontmost PID;
  - the owner of the front on-screen window;
  - the app's own AXFrontmost.

  The system-wide accessibility query returned `kAXErrorCannotComplete` in this session, so it was
  not used.
- **Log** (`evidence/actions.jsonl`). It has 219 entries, all for PID 64415:
  - All 128 input entries carry the three-way frontmost check.
  - The only error is the first `identify`, which was refused before any input because of the
    missing registered bundle ID.
- **Never done:**
  - Continue with ChatGPT, adding an account or signing out. Inside the account dialog I sent only
    Tab, Shift+Tab and Escape, after checking focus, and pressed its Close button through
    accessibility.
  - No contact with the other Incant app, and no credential or account-directory read.
- **Interference I did not cause:**
  - Between 21:15:51 and 21:16:32, the review window was zoomed to 1512×874, Camera Perspective was
    selected, the agent panel was collapsed, and another app came to the front.
  - At about 21:21:30–21:21:50, focus in the open account dialog moved from Continue with ChatGPT to
    the dialog container, as a click on dialog content does.

  The log shows no helper input at either time. I don't know the source. I restored 1440×874 and
  re-checked state before continuing. The agent panel stayed collapsed in the later captures.
- **The other Incant app** (`artifacts/0033-native-clean/Incant Studio Review.app`, PID 64681) was
  running when I started and was gone at the end. No helper action targeted it.

## Sizes

| Size | Actual |
| --- | --- |
| Normal | 1440×900 requested; the window is **1440×874**, because the display's visible frame is 874 pt tall. Captures are 2880×1748 |
| Minimum | **1000×650**; a 900×600 request stayed at 1000×650. Captures are 2000×1300 |
| Narrow Inspector | **280 pt**, from 360, set with Home on the focused "Resize inspector and agent" divider at 1000×650 |

## Observed acceptance

### The five requirements

| Requirement | Natively |
| --- | --- |
| 1. No useless collider text; no permanent Entity/ID rows | Pass. <ul><li>The Inspector header is the glyph, name, lock and Copy entity ID only.</li><li>No collider paragraph is visible.</li><li>"Entity, ID …" and "Read-only" remain only as visually hidden header text for screen readers.</li><li>The compound part's own ID is a data field, kept since 0025.</li></ul> |
| 2. Compact, professional account dialog | Pass. <ul><li>420×188 pt at both sizes: title, Close, one status line, the error with its code, and one button.</li><li>See N2 for the status wording.</li></ul> |
| 3. Searchable shortcut table with Mac notation | Pass. <ul><li>Native WebKit shows ⇧F6, ⌘Z, ⇧⌘Z, "? or ⌘/", "⌘⌫ or ⌦", "⇧F10 or Menu", ⌥⌘1 and ⌥⌘2.</li><li>Keys are spoken as words, e.g. "Shift Command Z" and "Option Command 1".</li><li>"hierarchy" gives 10 of 29.</li></ul> |
| 4. Avatar-only titlebar control | Pass. <ul><li>A 30×28 pt person glyph with a red corner dot and no text.</li><li>Its name and tooltip carry the state: "ChatGPT account, sign-in problem: provider: Could not create Incant account directory".</li></ul> |
| 5. Plain controls | Pass. <ul><li>No tiles, glows, accent edges or nested ornament in any state.</li><li>The only elevation is the existing dim scrim behind modal dialogs and the context menu's shadow.</li></ul> |

### Required areas

| Area | Observed |
| --- | --- |
| Account unavailable, Escape and Close focus | <ul><li>Opening focuses Continue with ChatGPT.</li><li>Tab wraps to Close; Shift+Tab wraps back.</li><li>Escape, the Close button, and Escape from the dialog container each close it, with focus on the account button.</li><li>The dialog is the same at 1000×650.</li></ul> |
| Shortcuts | <ul><li>`?` and ⌘/ open it with focus in the filter.</li><li>The first Escape clears the filter; the second closes and returns focus to the opener.</li><li>600×680 pt at normal size; 600×554 pt at minimum, scrolling inside.</li></ul> |
| Cameras | <ul><li>**Perspective:** 60°, 0.1 m and 100 m.</li><li>**Orthographic:** Vertical size 10 m, Field of view tagged Unused.</li><li>**Pixel perfect:** 16 px/m, Resolution px W 320 H 180, Field of view Unused.</li><li>All fit at 1000×650 and at 280 pt with units visible.</li></ul> |
| Compound | Dumbbell: <ul><li>"Parts 2 · 2 sphere", with rows "sphere r 0.3 m".</li><li>Part 1 of 2 (X −0.7); ↓ gives Part 2 of 2 (X 0.7), with a focus ring.</li><li>Fits at 280 pt with the full part ID.</li><li>Tab order: Copy entity ID, Collider, Shape, part row, Primitive, Radius.</li></ul> |
| Sprite | Atlas Sprite reads as the 0035 browser review recorded: <ul><li>Texture "existing.atlas 01M4M7Z8QS5EW…".</li><li>Region shows the unsupported notice with the raw frame.</li><li>Tint is labelled X/Y/Z/W.</li><li>Flips and Sampling read "Not set".</li><li>Rows are alphabetical, and the glyph is the generic entity glyph.</li></ul>This is findings 3–8, not a regression. |
| Atlas, Color only | <ul><li>Listed as "Atlas"; its spoken name is "Texture, Atlas".</li><li>Details show Source `existing.atlas.json`, a plain "Color" value and "Sprite atlases are always color.".</li><li>The button reads only "Reimport"; there is no Linear or Normal map choice.</li></ul> |
| Ordinary image, editable | <ul><li>Color (saved), Linear and Normal map radios.</li><li>→ to Linear shows "Saved as Color; reimport to apply." and "Reimport as Linear".</li><li>That reimport committed: "Texture, Linear", journal seq 5, and the button went back to "Reimport".</li></ul> |
| Import validation | <ul><li>`refused.json`: "Unsupported file type. Use glTF, GLB, PNG, JPEG, EXR, .atlas.json, WAV or OGG."</li><li>`../outside.png`: "Paths cannot leave the project folder (“..”)."</li><li>Both were refused before sending; History was unchanged and focus stayed on Path 1.</li></ul> |
| Real atlas, WAV and OGG import | <ul><li>Shift+Return added rows reading "Color atlas", "WAV audio", "OGG audio", and the PNG's Color/Linear/Normal map choice.</li><li>Return imported: "Imported 4 files. Undo reverts the whole batch."</li><li>Journal seq 4 holds four `upsert_asset` commands, and the list gained `new.atlas` (Atlas), two `new` (Audio) and `ordinary` (Color).</li></ul> |
| Native import failure | `missing.png` passes the UI's checks and fails natively: <ul><li>"Nothing was imported — could not cook missing.png: asset IO: No such file or directory (os error 2). The asset list is unchanged. Your paths are kept below."</li><li>The status bar shows the error, and the Assets tab shows "The last import failed".</li></ul> |
| Completion focus (28e5491) | <ul><li>Import success: the new, empty Path 1.</li><li>Import failure: Path 1, with the path kept.</li><li>Reimport of the atlas, the image (Linear), WAV and OGG: the asset heading, every time.</li></ul> |
| Keyboard and menu transitions | <ul><li>↑/↓ in the tree.</li><li>F6 order: hierarchy, viewport, output, inspector, agent, hierarchy; Shift+F6 reverses.</li><li>Shift+F10 opens Rename (F2), Copy ID and Delete (⌘⌫); ↓ moves; Escape returns to the row with nothing run.</li><li>←/→ on the left-panel tabs; End/Home in the asset list; Return opens details; Escape returns to the row.</li><li>⌥⌘3 hides and shows the Inspector.</li><li>Native **Edit › Select All**, **Edit › Undo** and **Edit › Redo** work. The window after Redo is byte-identical to the one before Undo (`6233dd86…`).</li></ul> |
| Inactive window | <ul><li>Traffic lights turn grey.</li><li>The project name goes from #f0f0f4 to #99999d and "Main" from #b8bcc5 to #75787f on #0b0d12: about 10.2:1 → 4.4:1, approximate.</li><li>The account dot dims from #f39688 to #9b6059.</li></ul> |

## Findings for Astra

Each finding is pre-existing and was not introduced by 28e5491. I changed no source.

1. **N1: a reimport that changes nothing promises an Undo that isn't there.** Behaviour, Astra; then
   copy, Claude.
   - **What.** Reimporting the atlas, WAV or OGG source reports "Reimported. Undo reverts the whole
     reimport." (`AssetInspector.tsx:174`), but the host records no transaction:
     - History stayed at 3, then at 5;
     - the journal has no record.
   - **Repro (native).**
     1. Right after the no-op OGG reimport, choose **Edit › Undo**.
     2. It reverts the 21:28:06 Linear reimport of `ordinary` instead ("4 of 5 applied", `ordinary`
        back to Color).
   - **Proposal.** Have the import result say whether it committed, or compare revisions. A no-op
     reimport would then read "Already up to date. Nothing changed." with no Undo sentence. Add a
     test.
2. **N2: unavailable account storage reads as a failed sign-in.** Host state, Astra; copy, Claude.
   - **What.**
     - The restore failure ("Could not create Incant account directory") arrives as `provider.auth`.
     - The dialog says "Sign-in didn't finish. You can try again now." (`AccountDialog.tsx:291`),
       although no sign-in was attempted.
     - Focus starts on Continue with ChatGPT, which can only fail the same way.
   - **Proposal.**
     - The host gives storage failures their own code, such as `provider.storage`.
     - The dialog then says "Account storage is unavailable on this computer." with the detail, and
       starts focus on Close.
3. **N3: assets that share a name are indistinguishable.** Presentation, Claude, with Astra's
   approval.
   - **What.** `new.wav` and `new.ogg` both show as "new", with the same glyph and the spoken name
     "new Audio". The path is only in the tooltip.
   - **Proposal.** When names collide in a folder, show the extension or a format tag, as "Atlas"
     already is.
4. **N4 (minor): History labels a reimport "Import assets".** That label is the host's commit
   description in `editor/app/src/assets.rs`, so a reimport looks like a batch import. A
   "Reimport <name>" description would read better.

**Earlier findings:**
- 3–8: unchanged, as shown above.
- 9: borderline natively, about 4.4:1 by approximate pixel measurement.
- 10: not exercised.

## Limits

- **⌘-key equivalents.** Synthetic ⌘-key equivalents posted to the PID did not trigger AppKit menu
  items: ⌘A did not select all. Shortcuts that WebKit handles (`?`, ⌘/, ⌥⌘3, F6, Shift+F10) did
  work. So menu actions were tested by pressing the native menu items. The physical ⌘Z → menu →
  webview route is unobserved; only the menu item is.
- **Assistive tech.** No VoiceOver run. Names, roles and focus come from the macOS accessibility API,
  which VoiceOver also reads, but nothing was heard.
- **Accounts.** Only the unavailable or error state was seen. Connected, switching, signing in and
  signing out remain covered by 0035's account-free browser fixtures.
- **Not observed:** large scenes (the 64-part compound, the District grid, paging), Windows chrome
  and wide sizes.
- **Pixel colours** are approximate. They come from antialiased pixels in captures with a display
  profile, converted to sRGB.
- **Not run:** Cargo, unit tests, phones and VoiceOver.

## Fixture and side effects

- **Fixture changes, made through the native UI:**
  - The journal grew from 4 to 8 records:

    | Seq | Record |
    | --- | --- |
    | 4 | Import assets: four upserts |
    | 5 | Import assets: `ordinary` as Linear |
    | 6 | Undo of seq 5 |
    | 7 | Redo of seq 5 |

  - 8 new cooked cache files.
  - `review.incant.json` and every input file are unchanged (`evidence/fixture-*.sha256`). No JSON
    was edited directly.
- **Outside the fixture.** At launch (21:14:47) the app created
  `~/Library/WebKit/dev.incant.review.28e5491` and `~/Library/Caches/dev.incant.review.28e5491`.
  They hold WebKit and cache data for the review bundle ID. I left them in place because they are
  outside my allowed paths.
- **The prepared bundle** in `/private/tmp` is unchanged.

## Evidence

- **Screenshots** (`screenshots/`, account-free, 3.6 MB):
  - Six are unedited window captures.
  - The other 25 are unscaled crops at the accessibility frames recorded during the run, made by
    `tools/curate.py`, which reproduces them exactly.
  - The raw set of 41 captures (13 MB) and the logs are in the ignored
    `artifacts/0036-editor-final-native/`.
- **Evidence JSON.** `evidence/native-review.json` holds identity, sizes, observations, interference,
  fixture changes, and the hash of every raw capture.

| File | Shows |
| --- | --- |
| `normal-camera-perspective-1440x874.png` | Whole window, perspective camera |
| `camera-orthographic-inspector.png`, `camera-pixel-perfect-inspector.png` | Orthographic; pixel perfect |
| `dumbbell-part2-inspector.png` | Compound list, Part 2 focused |
| `atlas-sprite-inspector.png` | Sprite as in findings 3–8 |
| `account-unavailable-dialog.png`, `account-close-focus-dialog.png` | Unavailable account; focus on Continue, then on Close |
| `titlebar-end-account-focus.png`, `titlebar-active.png`, `titlebar-inactive.png` | Avatar-only control with focus after Escape; active and inactive titlebar |
| `shortcuts-mac-dialog.png`, `shortcuts-filter-hierarchy-dialog.png` | Mac labels; "hierarchy" 10 of 29 |
| `assets-list-before-import.png`, `atlas-details-inspector.png`, `atlas-reimported-noop-inspector.png` | Atlas tag; Color-only details; the no-op reimport note (N1) |
| `import-refused-json-inspector.png`, `import-traversal-refused-inspector.png`, `import-typed-rows-inspector.png`, `import-missing-failure-inspector.png` | Refusals, typed rows, native failure |
| `normal-import-success-1440x874.png` | Success, new rows, focus on the empty Path 1 |
| `ordinary-linear-pending-inspector.png`, `ordinary-reimported-linear-inspector.png`, `ogg-details-inspector.png` | Editable interpretation; audio details |
| `history-after-menu-undo-dock.png` | Edit › Undo reverting the Linear reimport (N1) |
| `context-menu-hierarchy.png` | Shift+F10 menu |
| `minimum-*-1000x650.png`, `pixel-camera-narrow-inspector.png`, `dumbbell-narrow-inspector.png` | Minimum window and the 280 pt Inspector |

SHA-256 of each committed file (`evidence/screenshots.sha256`, verified with `shasum -c`):

```
70342793a4ad1bfe2b993e4efcbe17f632753bc4f14ee964c73cd5c9446ff965  screenshots/account-close-focus-dialog.png
64c8e2da402a7488d3b1e3464d80d28cd1e6f94f4ebb7c36fe1d81943f8e7e87  screenshots/account-unavailable-dialog.png
2a4aa6c0db373a21b9c253c60e8502f0f4dfd208ee65c7a411b6381479617e5a  screenshots/assets-list-before-import.png
5912fd1f7e16b477763009d8406a921a476ace7ac6846313dab83fbbf8501626  screenshots/atlas-details-inspector.png
1e6df9fd449a5d6ee3b19eec006448f131a192798dc43bce6a1e458199d72481  screenshots/atlas-reimported-noop-inspector.png
f740e5d6afd2495bbbc641bead29f4abda444fd8e895f35042a516c57591cd43  screenshots/atlas-sprite-inspector.png
c2d2208aefba4cb6c1535d0c2efd65f661fa3bf2665ad63224b6bb308eae2b1d  screenshots/camera-orthographic-inspector.png
e24633b079d620723234601bbc3689b2fd647f6a794ccbb1462f31f0942e8179  screenshots/camera-pixel-perfect-inspector.png
87278ce6a45f108ee70bd2b9e7031f99997e248ad222f0aacb7a9278da2f6921  screenshots/context-menu-hierarchy.png
b37e7b5b6a25fce973b41757024c2942a0bd3e9a884e1108b376167e06b272ec  screenshots/dumbbell-narrow-inspector.png
0ef374af01ef080cfb8a0fbc529ad8f8c008acaa633df8382c8ea1f850e96937  screenshots/dumbbell-part2-inspector.png
2865a5fce7ea6ee19cf13a8c5e0e21ca6811bc7d06f1f08c00e0f403782f99c1  screenshots/history-after-menu-undo-dock.png
7a87c7c080b1940fff213b6264c2bab499809315c96f74e8d7c4a2a82011b26f  screenshots/import-missing-failure-inspector.png
5508c80a953c97e56382c8c8bc06379ae6a8b752a2bcf19ecfa40bce27fe455b  screenshots/import-refused-json-inspector.png
26d989cee1de11006241596e53eb1de4157634ec40979fc0ed322eb051a4664b  screenshots/import-traversal-refused-inspector.png
89b765952d6fee8ffae73f682a2b9c8c838daccf5f06b67ebed72efae4f3124c  screenshots/import-typed-rows-inspector.png
49e7d724916be8545872b3dc0977a5f7dcb578ec08a37e665d42476691dadcb8  screenshots/minimum-account-unavailable-1000x650.png
4e7c5e3ea38a915c5a8d9309e6b725d80026a36a24c600fe5af3ae1ffefda6b9  screenshots/minimum-atlas-narrow-inspector-1000x650.png
6eeff050f81835fe860ff21ecc3b33e4354ddd0ec87ca9f0d74c2deaa7013208  screenshots/minimum-pixel-camera-1000x650.png
c71284fd4cd500d307d1f25db5eb66aea4f8192d5600031c288eeb0d38af0e5e  screenshots/minimum-shortcuts-1000x650.png
087cf86598fff85905eccd783a813fd20af6789efda08c85ed2b5d70ffa4bdf5  screenshots/normal-camera-perspective-1440x874.png
e83a3719901f546f3511dcfe836b0f43e81097707a3473699af12a775aecfe31  screenshots/normal-import-success-1440x874.png
cb274bb7182efb018f2cf5a377b047b9358de79a598ee3da7c9e53500b317517  screenshots/ogg-details-inspector.png
36d00eef10997d99569ac81a530255bb5bd98bef1bc824b3d7c5e0976e2d089d  screenshots/ordinary-linear-pending-inspector.png
11cc5fba46c8b15464d9b6d0f05f82dbece6c152d2b7729dc0e3ebfef50f7d1b  screenshots/ordinary-reimported-linear-inspector.png
93f7c104a7afbc9c39a22f89ee5c85ff2f42be8246ef2e5680c5c06f19cfcffb  screenshots/pixel-camera-narrow-inspector.png
6499fb3ec6bc03fe6a7d97fc4a9ca3d4949eefb21c16dc81ac886376e8a45af8  screenshots/shortcuts-filter-hierarchy-dialog.png
9d4631936351765fa412883ebe62b8dbe301552279795464894b0647cc90ee44  screenshots/shortcuts-mac-dialog.png
5573e0d792347a3dc74c558043f3a7e73e0353b248eb87a4943f2e7a9aec66ce  screenshots/titlebar-active.png
b3f44eced5e93acad50e4623cd3589379407157224aadce2c153206de5d37dc5  screenshots/titlebar-end-account-focus.png
8751fd5759117fbf636bcd66abf58433889a74273869da000d9dcd58ee0c14d4  screenshots/titlebar-inactive.png
```

## Commands and results

```sh
shasum -a 256 "<executable>"                         # 51599f65…, matches the packet
swiftc -O handoffs/0036-editor-final-native/tools/native.swift -o artifacts/0036-editor-final-native/native
artifacts/0036-editor-final-native/native trusted     # accessibility and screen capture granted
handoffs/0036-editor-final-native/tools/launch.sh artifacts/0036-editor-final-native/logs/app-stdout.log   # PID 64415
native identify|raise|key|type|press|focus|menu|resize|capture|texts|focused --pid 64415 --exe "<executable>" ...   # 219 log entries
native quit --pid 64415 --exe "<executable>"          # normal quit; exited
python3 -I handoffs/0036-editor-final-native/tools/curate.py artifacts/0036-editor-final-native/native artifacts/0036-editor-final-native/raw handoffs/0036-editor-final-native   # 31 images; rerun gives an identical manifest
(cd handoffs/0036-editor-final-native && shasum -a 256 -c evidence/screenshots.sha256)   # 35 OK
```

No Cargo, npm, project test suite or project build ran: the packet forbids Cargo, and no source
changed. The only compilation was the review helper, with `swiftc`.

## Changed paths

- `handoffs/0036-editor-final-native/result.md`: this file.
- `handoffs/0036-editor-final-native/tools/`: `native.swift`, `launch.sh` and `curate.py`.
- `handoffs/0036-editor-final-native/screenshots/`: 31 images.
- `handoffs/0036-editor-final-native/evidence/`: `native-review.json`, `actions.jsonl`,
  `fixture-before.sha256`, `fixture-after.sha256` and `screenshots.sha256`.

## Open questions

- N1: should a no-op reimport be reported as "already up to date", or should the host refuse to call
  it a reimport at all?
- N2: should unavailable storage keep "Continue with ChatGPT" visible, disabled with the reason, or
  hide it?
- Should the two `~/Library/*/dev.incant.review.28e5491` directories be removed with the temporary
  bundle?
