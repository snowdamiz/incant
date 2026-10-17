# Native hierarchy interaction

The actual Mac editor passed the ordinary hierarchy shortcut, context-menu,
history, keyboard-focus and saved-project workflow on October 16, 2026. This
completes the native interaction follow-up to [subtree duplication](editor-subtree-duplicate.md)
and [focus recovery](editor-hierarchy-focus-recovery.md), within this fixture.
It does not establish the broader Phase 2 editor acceptance gate.

The producer was `212aa7ec2a9b300ba81698e2fee9a6693ba5e94c`, with tree
`93b1dc1c6aabc6177389feacea3d44a768eece42` (the main380 tree). The ordinary
`ci`-profile editor was the same binary used by the
[native material workflow](live-material-binding-refresh.md#actual-native-window-material-workflow):
SHA-256 `cad1bacb2b473cf352a680d413b132234570c308f61e111ff5bee09adf830ce6`.
This was functional testing, not a shipping-profile performance measurement.

## Native actions and retained outcome

A headless command transaction created a parent, its child and an unaffected
sibling. The dedicated native app opened that saved project on the unlocked Mac.
Root operated its actual accessibility controls and keyboard input:

1. Cmd+D duplicated the parent and child, selecting the new parent. Undo removed
   the copy; Redo restored the same IDs and expanding the parent exposed its child.
2. The context menu's Duplicate action made a second subtree copy. Undo removed
   it, and the next Up key selected a surviving child through recovered focus.
3. Cmd+Backspace deleted that child. The next Up key selected its parent, and
   Undo restored the deleted child.
4. An immediate Save while Undo was still busy correctly refused with
   `engine.request`. Save after completion reported revision 8.
5. Cmd+Q exited normally. A fresh headless process reopened the saved project
   with its journal and returned revision 8. It contained the unchanged original
   three entities and exactly the first copied parent/child, with their original
   duplicate IDs, remapped parent relation and shared transaction provenance.

The native window ran for 102 seconds and exited zero. All seven owned process
groups were absent at closeout. The retained saved file equals the fresh-process
project response. There were no runtime, script, rendering or UI-style changes
for this evidence collection.

## Evidence and limits

[Portable records](evidence/native-hierarchy-window/manifest.json) contain exact
build, launcher, setup, state and root-observation bytes. Run:

```sh
python3 docs/spikes/evidence/native-hierarchy-window/verify.py
```

This only checks retained records; it never opens an app, executes an engine or
replays input. Intermediate selection, menu, focus and busy-save behavior is
supported by root's contemporaneous accessibility observation notes, not a raw
accessibility transcript or screenshot. The verifier checks those notes' identity
and scope; it cannot independently recreate the intermediate interactions.

The user-authorized October 16 disk cleanup removed the original executables,
raw journal and large temporary artifacts. The recovered build receipts,
launcher, RPC output, before/after state and saved file remain preserved here;
the raw journal itself cannot now be replayed from this packet. No fresh native
execution is claimed for the October 17 recovery. The earlier locked-desktop
attempt remains historical. Appearance, large-scene timing, complete editor
workflows and phase approval remain separate.
