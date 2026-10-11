# 0038 import completion review: result

## Status

- **Done, with one narrow copy change.** The completion states read plainly and accurately at
  1440×874, 1000×650 and with a 280 px Inspector. They follow the receipt rules in the brief:
  - **Unchanged import and reimport:** "Already up to date. Nothing changed." There is no Undo
    hint and no just-imported mark, and the history is untouched, including its redo tail.
  - **Changed:** "Imported 3 files." or "Reimported." The Undo hint shows only while that exact
    transaction is the applied head. Undo and a newer edit hide it; Redo restores it.
  - **No receipt:** "Import completed." There is no hint and no mark, even when History shows a
    new "Import assets" at Now.
  - **Failure:** unchanged.
- **Changed here (finding 1).** An unchanged result's status-line announcement now names what it
  is about: "Already up to date: crate_albedo. Nothing changed."
  - Not changed: the notice text, receipt conditions, history logic, Rust, the bridge, Cargo, CI
    and styles.
- **Reported, not changed:** five observations (O1–O5), all older than a62d35e.
- No phase gate is approved or claimed. This is browser evidence only. Native acceptance of this
  follow-up stays pending.

## Model and effort

- Exact model: Claude Opus 5.5, model ID `claude-opus-5-5`.
- Thinking effort: Max. The session reports reasoning effort "max" to the model. The ACP runner
  selects and verifies both settings; I cannot inspect that verification from inside the
  session.
- No substitution, and no routing exception.

## Source

| Item | Value |
| --- | --- |
| Base (frozen) | `a62d35ea2f7d6f9bdbf487b7d84d0555689d553f`. All 8 `source_sha256` entries in `docs/spikes/evidence/editor-import-receipts-2026-10-11.json` matched before any change |
| `editor/ui/src/assets/AssetsContext.tsx` | `16aa82d0…` → `d37f1699…`: the two announcement strings |
| `editor/ui/src/components/assets/Assets.test.tsx` | `07721360…` → `85975846…`: the matching assertion |
| `editor/ui/src/components/assets/AssetInspector.tsx` | `84402ea4…`, unchanged, so the notice text is unchanged |
| UI build | <ul><li>Frozen JS: `index-D1eFgbiT.js`. Final JS: `index-DZmcOg3H.js`.</li><li>`index-DBrXeGP-.css` is byte-identical in both (`c25654e9…`).</li><li>Builds are reproducible: rebuilding each source gave the same files.</li></ul> |
| Unchanged | Rust, the bridge, Cargo, CI, styles, schemas and docs |

## Finding 1, fixed here: the status line did not say what an unchanged result was about

- **What happens.**
  - While an import runs, the pending line says "You can keep working."
  - If the user selects another asset before it finishes, the notice is no longer on screen. It
    shows only in the reimported asset's details, or at the top of the import form.
  - The status line is then the only feedback:
    - A changed result names its subject: "Reimported crate_albedo." or "Imported 3 files.".
    - The unchanged result said only "Already up to date. Nothing changed.". Next to
      gull_sheet's details, that reads as a statement about gull_sheet.
  - Observed for a reimport (normal and 280 px) and for a form import with harbor_loop on screen.
- **Cause.** a62d35e added the unchanged and receipt-less messages as one shared string. Before
  that, the reimport announcement always named the asset.
- **Fix.** The status line only:

  | Outcome | Notice (not changed) | Status line before | Status line now |
  | --- | --- | --- | --- |
  | Unchanged reimport | Already up to date. Nothing changed. | Already up to date. Nothing changed. | Already up to date: crate_albedo. Nothing changed. |
  | Unchanged import, 2 paths | Already up to date. Nothing changed. | Already up to date. Nothing changed. | Already up to date: 2 files. Nothing changed. |
  | Changed, no receipt, failure | — | — | Unchanged |

- **Why this wording.**
  - It keeps the brief's sentence and inserts the subject, so the notice and the status line read
    as one message.
  - It leads with the outcome, as "Import failed: …" does.
  - The status line is one line with an ellipsis, so the outcome stays visible even when a long
    name is cut. The longest name in the double (60 characters) fits at 1000 px, and the native
    label "Native engine" leaves more room than the double's.
  - A subject-first draft ("crate_albedo is already up to date. …") came first. I replaced it for
    that reason; its capture sets are not evidence.
- **"Import completed." keeps no subject.**
  - It makes no claim about any asset.
  - No current bridge reaches it: the fixture bridge advertises no capabilities, and the native
    bridge always returns a receipt.
- **Tests.**
  - The updated assertion fails on the previous copy (2 failed, 37 passed in `Assets.test.tsx`)
    and passes now.
  - The suite stays at 30 files and 529 tests: one assertion changed, none were added or removed.
- **Screenshots.** Compare `screenshots/before/` (frozen source) with
  `screenshots/normal/away-noop-reimport-normal-1440x874--done.png` and `screenshots/status-line/`.

## Review of each state (final source, set g1)

Every host result below is MOCKED by the browser double; see "What is real and what is mocked".

| State | Notice | Status line | History shown | Marks | Focus afterwards |
| --- | --- | --- | --- | --- | --- |
| Unchanged reimport (crate_albedo) | Already up to date. Nothing changed. | Already up to date: crate_albedo. Nothing changed. | 5 of 6 applied, Now after "Rename entity to Lantern"; redo tail kept | None | Asset name in the header, ring |
| Unchanged import (2 existing paths) | The same | Already up to date: 2 files. Nothing changed. | Unchanged | None | Reset Path 1, ring |
| Unchanged, with a later source-watch edit in the same read | The same | Already up to date: crate_albedo. Nothing changed. | 6 of 6, Now after "Reimport changed sources" | None | Header |
| Changed reimport | Reimported. Undo reverts the whole reimport. | Reimported crate_albedo. | 6 of 6, "Import assets" at Now; redo tail dropped | crate_albedo | Header |
| … then ⌘Z | Reimported. | — | 5 of 6, "Import assets (undone)" | Kept (O3) | Header |
| … then ⇧⌘Z | Hint back; the 280 px Inspector crop is pixel-identical to the changed one | — | 6 of 6 | Kept | Header |
| … then a newer rename (F2 in the hierarchy) | Reimported. | Renamed to Harbor Sun | 7 of 7, "Rename entity to Harbor Sun" at Now | Kept | Assets tab (clicked) |
| Changed import (3 new files) | Imported 3 files. Undo reverts the whole batch. | Imported 3 files. | 6 of 6 | The 3 new rows | Reset Path 1 |
| … then ⌘Z in that path field | Unchanged | — | Still 6 of 6; nothing sent (O1) | Kept | Path 1 |
| … then the titlebar Undo | Imported 3 files. | — | 5 of 6; the mocked host removes the 3 assets | Gone with the rows | Undo button |
| No receipt (reimport at all sizes, import at normal) | Import completed. | Import completed. | 6 of 6, with "Import assets" at Now and still no hint | None | Header, or Path 1 |
| Failure (form, native cook message) | <ul><li>"Nothing was imported"</li><li>the Rust message</li><li>"The asset list is unchanged. Your paths are kept below."</li><li>Dismiss</li></ul> | Import failed: could not cook textures/dock/rope_coil.png: asset IO: No such file or directory (os error 2) | Unchanged | None | Stays on Path 2; "Try again"; paths kept |
| Moved away (gull_sheet or harbor_loop on screen) | None on screen | Names crate_albedo, or "2 files" (finding 1) | Unchanged | None | Asset row |

- **Sizes.**
  - The Inspector measured 360 px at normal size, and 280 px at minimum size and in the 280 px
    set.
  - The unchanged notice stays on one line even at 280 px; the changed notices wrap to two lines.
  - Nothing is clipped and nothing overflows. After completion the notice is in view, because the
    pane scrolls to the top.
- **Contrast.** Notice text is 10.3:1. The failure's title, message and hint are 13.8:1, 11.1:1
  and 7.3:1.
- **Plain style kept.** One tinted strip with a check icon, and no new elements. The account
  dialog, avatar-only control, shortcuts and Inspector are untouched.

## Accessibility and live feedback

- **axe-core 4.14.0:** 0 violations in all 55 steps of every set.
- **DOM evidence** (MutationObserver records; not heard with a screen reader):
  - **At completion:**
    - The notice is inserted already filled: `role="status"`, or `role="alert"` for a failure.
    - The persistent status line (`role="status"`, polite) changes text.
    - Unchanged results no longer give both the same text; changed results already differed.
  - **Undo** removes only the hint's text node from the notice, and **Redo** adds it back. No
    other live region changes. Under the default `aria-relevant` (additions and text), Undo
    should not prompt a new notice announcement.
  - **A newer edit made elsewhere:** the notice leaves with its pane, and returns without the hint
    when the asset is shown again.
- **No awkward duplication.**
  - The status line is the app's transient message: it clears after 8 s, and now says what the
    result is about.
  - The notice states the result where it happened.
- **Focus.** As in the 0036 fix:
  - after a reimport, focus moves to the asset's name, because the button is replaced while it
    runs;
  - after an import, focus moves to the reset Path 1;
  - after a failure, focus stays on the path.

## Other observations, not changed

**O1. ⌘Z right after a form import does nothing to the project.** Behaviour, for Astra; older
than a62d35e.
- **What.** After a changed import, focus returns to the empty Path 1 field. There, ⌘Z belongs to
  the text field by design: `App.tsx` never falls back to project history from text entry.
  Pressing ⌘Z right after reading "Undo reverts the whole batch." leaves the project at 6 of 6 and
  sends nothing.
- **What does work.** The titlebar and History Undo buttons; ⌘Z after a reimport, because focus
  is then on the asset name.
- **Options.** Keep the standard text-field behaviour, or let ⌘Z in an untouched, empty path field
  fall through to project Undo. That second option is a behaviour change. The hint's wording is
  accurate either way.

**O2. Progress and errors are said in more than one live region.** Accessibility, for Astra;
older than a62d35e.
- **Progress.** While an import runs, three live regions say so: the status line, the asset
  browser's `asset-browser__status` and the Inspector's pending line.
- **Errors.**
  - A failure inserts an alert, which screen readers normally announce on insertion.
  - The status line also says "Import failed: <message>".
  - So the error is likely spoken twice. This was not checked with a screen reader.

**O3. The success notice outlives its transaction.** A presentation question for the director;
by design in the brief.
- **What.**
  - After Undo or a newer edit, the notice still says "Reimported." or "Imported 3 files." with
    its check, while History shows the edit undone.
  - After the batch Undo, the three files are no longer in the list.
  - A reimported asset keeps its just-imported mark after its reimport is undone.
- **Effect.** This is accurate as a record, but it can read as the current state.
- **Why not changed.** Clearing the notice, or saying it was undone, needs history-dependent
  conditions, which this packet excludes.

**O4. Reimport failure copy.** A copy follow-up; older than a62d35e.
- **What.** A failed reimport shows "Nothing was imported" and "The asset list is unchanged. Your
  paths are kept below." in the asset details, which have no path list
  (`screenshots/probe/failure-reimport-normal-1440x874--done-side.png`).
- **Proposed fix.** A text-only change in `AssetInspector.tsx`, for reimports: "Nothing was
  reimported" and "The asset is unchanged.".
- **Why not changed.** The packet keeps failures as they are.

**O5. The error message breaks inside names at 280 px.** Style; older than a62d35e.
- **What.** `.asset-note__message` uses `overflow-wrap: anywhere`, so "rope_coil.png" breaks as
  "rope_coil.p|ng".
- **Possible follow-up.** A style change could prefer breaks after slashes, as paths elsewhere
  do. Styles are out of scope here.

## What is real and what is mocked

- **Real:**
  - the production UI build, at the hashes above;
  - all UI logic, and the requests the UI dispatched, recorded in the evidence;
  - completion text, focus, live-region changes and layout;
  - the initial project and history, converted by the real compiled `snapshotFromEngine`
    (`editor/bridge/native.generated.js`).
- **Mocked** (`tools/receipt-double.mjs`, which reuses the 0035 project):
  - every host result: receipts, the failure, Undo, Redo and the rename;
  - the history and assets published after each one;
  - the concurrent source-watch transaction.

  History descriptions use the Rust host's own strings: "Import assets", "Rename entity to …",
  "Delete entity" and "Reimport changed sources". A committed transaction drops the redo tail, as
  `incant_cmd` does.
- **Not covered here:** native command execution, the journal, and real Undo and Redo. Astra's
  Rust tests cover them (`docs/spikes/editor-import-receipts.md`). They were not rerun, because
  this packet runs no Cargo.

## Commands and results

```sh
npm ci --offline --ignore-scripts --no-audit --no-fund   # local npm cache only; lockfile identical to the main checkout; 211 packages
npm run build --workspace editor/ui                      # frozen source: built
(cd editor/ui && npx vitest run)                         # frozen source: 30 files, 529 passed
(cd editor/ui && npx tsc -b --noEmit)                    # frozen source: 0 errors
# copy change, then:
(cd editor/ui && npx vitest run src/components/assets/Assets.test.tsx)   # 39 passed; with the previous AssetsContext.tsx: 2 failed, 37 passed
(cd editor/ui && npx vitest run && npx tsc -b --noEmit && npm run build) # commit tree: 30 files, 529 passed; 0 errors; built
# evidence sets, all from the committed capture.mjs:
#   previous AssetsContext.tsx restored -> build -> capture b1; final file restored (cmp verified) -> build -> capture g1, g2
node handoffs/0038-import-completion-review/tools/capture.mjs b1   # 28 captures, frozen source
node handoffs/0038-import-completion-review/tools/capture.mjs g1   # 28 captures, final source
node handoffs/0038-import-completion-review/tools/capture.mjs g2   # 28 captures, final source again
node handoffs/0035-editor-union-review/tools/pixel-diff.mjs artifacts/0038-import-completion-review/b1 artifacts/0038-import-completion-review/g1 artifacts/0038-import-completion-review/pixel-b1-g1 --json=artifacts/0038-import-completion-review/pixel-b1-g1.json   # 186 of 220 identical
node handoffs/0035-editor-union-review/tools/pixel-diff.mjs artifacts/0038-import-completion-review/g1 artifacts/0038-import-completion-review/g2 artifacts/0038-import-completion-review/pixel-g1-g2 --json=artifacts/0038-import-completion-review/pixel-g1-g2.json   # 209 of 220 identical (noise)
node handoffs/0038-import-completion-review/tools/summarize.mjs g1
node handoffs/0038-import-completion-review/tools/curate.mjs       # 31 screenshots; 34 differences, 24 of them status line only
(cd handoffs/0038-import-completion-review && shasum -a 256 -c evidence/screenshots.sha256)   # 31 OK
```

- **Each set (b1, g1, g2):** 28 captures and 55 steps, with 0 axe violations, page errors, remote
  requests, step errors, window overflow or clipped status lines.
- **Frozen to final:** 34 of 220 images differ.
  - 24 are the status-line box only: 12 unchanged-outcome captures, each with its status-line
    crop.
  - 9 also differ between the two final runs.
  - 1 is a spinner, caret or hover frame before completion.
- **Notices, Inspector crops and dock crops:** identical between the source revisions, apart from
  that noise.

## Screenshots

Account-free, headless Chrome, deviceScaleFactor 2. Hashes are in `evidence/screenshots.sha256`
(31 entries).

| Folder | Files | Shows |
| --- | --- | --- |
| `screenshots/normal/` (1440×874, Inspector 360) | `noop-reimport`, `changed-reimport` `--changed`, `--undone` and `--newer`, `changed-import`, `unknown-reimport`, `failure-import`, `away-noop-reimport`, `noop-concurrent` | Each state with its status line and History |
| `screenshots/minimum/` (1000×650, Inspector 280) | `noop-reimport`, `noop-import`, `changed-reimport` `--changed` and `--undone`, `unknown-reimport`, `failure-import` | The same at the minimum window |
| `screenshots/inspector-280/` (1440×874, Inspector 280; side-column crops) | `noop-reimport`, `noop-import`, `changed-reimport` `--changed`, `--undone`, `--redone` and `--newer`, `changed-import` `--changed` and `--undone`, `unknown-reimport`, `failure-import` | The notices at the narrowest Inspector. `--redone` has the same hash as `--changed` |
| `screenshots/status-line/` | `away-noop-reimport`, `away-noop-import`, `long-noop-reimport` (1000×650) | Finding 1, final wording; the 60-character name fits |
| `screenshots/before/` (frozen a62d35e) | `away-noop-reimport` window and status line | Finding 1 before the fix |
| `screenshots/probe/` | `failure-reimport` side column | O4 |

## Evidence and logs

- **`evidence/completion-review.json`:**
  - source and build hashes;
  - browser and tool versions;
  - every observed step: notice, status line, History shown and held by the host, marks, focus,
    axe results, the requests sent and the mocked results;
  - live-region changes;
  - the classified pixel comparisons;
  - log hashes.
- **`evidence/screenshots.sha256`:** 31 entries.
- **Logs** (ignored, under `artifacts/0038-import-completion-review/`):

  | Log | SHA-256 prefix |
  | --- | --- |
  | `vitest-commit.log` | `feba98c7…` |
  | `tsc-commit.log` | `e3b0c442…` (empty) |
  | `build-commit.log` | `94e6a5c6…` |
  | `vitest-final-on-old-copy.log` | `f9036cbc…` |
  | `vitest-baseline.log` | `789680f9…` |
  | `b1.log` | `ed40e387…` |
  | `g1.log` | `72385310…` |
  | `g2.log` | `6574818a…` |
  | `pixel-b1-g1.log` | `c941c9f2…` |
  | `pixel-g1-g2.log` | `f5abaecb…` |

## Not run and limitations

- **Not run:**
  - any native app or window (another session owns one here);
  - WebKit;
  - VoiceOver or any other screen reader;
  - phones;
  - Cargo and GPU benchmarks;
  - hosted checks.
- **Environment:** headless Chrome 155.0.8059.40, playwright-core 1.64.0, Node v22.23.3, UTC.
  Dependencies came only from the local npm cache.
- **Live regions:** the conclusions come from DOM mutation records and ARIA rules, not from heard
  announcements.
- **Spike doc:** `docs/spikes/editor-import-receipts.md` still says an unchanged import reports
  "Already up to date. Nothing changed.". That holds for the notice; the status line now adds the
  subject. I did not edit docs, which are outside this packet's paths.
- **Astra's evidence hashes:** Astra's evidence JSON lists hashes for the two changed UI files
  from the frozen source. The new hashes are in `evidence/completion-review.json`.
- **Superseded runs:** earlier exploratory sets (r1, r2, r3, long-d, f1, f2 and p1) are replaced
  by b1, g1 and g2, which all come from the committed tool version.

## Changed paths

- `editor/ui/src/assets/AssetsContext.tsx`
- `editor/ui/src/components/assets/Assets.test.tsx`
- `handoffs/0038-import-completion-review/result.md`
- `handoffs/0038-import-completion-review/tools/`: `receipt-double.mjs`, `capture.mjs`,
  `summarize.mjs` and `curate.mjs`
- `handoffs/0038-import-completion-review/screenshots/`: 31 PNG files
- `handoffs/0038-import-completion-review/evidence/`: `completion-review.json` and
  `screenshots.sha256`

## Open questions

1. **For the director (O3):** should a success notice clear, or say it was undone, once its
   transaction is no longer applied?
2. **For Astra (O1):** should ⌘Z in an untouched, empty path field right after an import fall
   through to project Undo?
3. **For Astra (O2):** should the pending and failure messages use one live region?
4. **O4 and O5:** may a follow-up apply the reimport-failure copy and the error-message wrapping?
5. **For Astra:** when integrating, should the spike doc's sentence mention that the status line
   names the subject?
