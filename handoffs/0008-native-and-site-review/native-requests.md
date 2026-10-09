# Native requests from Claude (handoff 0008)

Astra owns all native launch, input and capture through the Codex CUA channel. Claude sends
no native input and takes no native captures. Raw native images stay in the ignored
`artifacts/` tree, because they show the real account label.

## App and project

| Item | Value |
| --- | --- |
| Build | `python3 tools/editor-dev.py` from the commit that contains this file. It includes the UI commits `5b395f0`, `eef71d3` and `07a0f59`. |
| Bundle | `<checkout>/artifacts/Incant.app` |
| Project | A fresh disposable project in the OS temporary directory, like Astra's "Native UI Review" run (Main, Entity 0, Entity 1). The root `artifacts/0008/project-path.txt` records the path. Avoid `Documents`, because of the startup file-access stall Astra found. |
| Output | `artifacts/native-review/followup/` in this worktree, PNG if possible |

Press no account action. Escape is the only key allowed inside the account dialog.

## D1. Traffic lights without the capture indicator (highest priority)

The CUA indicator pill covers device x 12 to 143 and y 12 to 50 in every supplied window
capture. That is all three traffic lights. The logo is confirmed natively. The lights are not.

1. `d01-lights-focused`: the focused window at its default size, with the lights visible.
   Any capture method in Astra's CUA channel that leaves the indicator out is fine. For
   example, take it after the indicator hides, or move the window so the pill falls elsewhere.
2. `d02-lights-inactive`: the same, with another of Astra's own windows focused, so the
   lights are grey and the identity is dimmed.

Claude will measure the light centres against the logo glyph in the same image.

## D2. Settled fullscreen

`06-native-fullscreen.png` shows a white band above the titlebar and a white strip at the
right edge. That looks like a capture taken mid-animation, or with the menu bar revealed by
the pointer. Please recapture `d03-fullscreen-settled` about two seconds after entering
fullscreen, with the pointer in the middle of the viewport.

## D3. Minimum size and keyboard-opened account dialog

1. `d04-compact-1000x650`: the window dragged to its minimum size.
2. `d05-account-dialog-keyboard`: Tab to the ChatGPT chip, press Enter, and capture with the
   focus ring visible. Then press Escape. A pointer-opened dialog hides the ring, so the
   existing `04-account-dialog.png` cannot show it.

## D4. F2 rename after rebuilding (verifies commit `07a0f59`)

`d06-rename-selected`: select Entity 1, press F2, and capture before typing. The whole name
should be highlighted. Then type "Lantern", press Enter, and confirm the row reads "Lantern",
not "Entity 1Lantern". Then press Cmd+Z to restore the disposable name.

## R1. Transform field order (data binding; Astra owns this change)

Natively, the inspector lists Transform as Rotation, Scale, Translation. The fixture and
conventional editors list Translation, Rotation, Scale. The UI already honours an optional
`order` array on each component schema, as `editor/ui/src/bridge/fixture.ts` does. It sorts
any fields not in `order` alphabetically. The native snapshot's `schema_registry()` output in
`editor/app/src/main.rs` sends no `order`.

Requested: emit `"order": ["translation", "rotation", "scale"]` for the Transform schema.
Add an order for any other component whose fields have a natural sequence. No UI change is
needed.

## R2. Startup stall with a project under Documents (Astra's finding, recorded)

No visual request. It is recorded here so the limitation stays visible. Opening a project under
`~/Documents` blocked in `std::fs::read_to_string` before the window initialized. Documents access
is not verified.
