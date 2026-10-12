# Text-field pointer focus

Clicking and releasing a native game text field now focuses it without emitting
`Submitted`. Only a captured button click activates from pointer release.
Explicit submit/activation still submits a focused field; an active composition
continues to own Enter without submitting. Capture, clipping and stale-pointer
checks are unchanged.

The regression fails on the original implementation because the field click
submits. The fixed all-feature/all-target `incant_ui` suite passes 47 tests with
no failures or ignored tests. Clippy with warnings denied, formatting and diff
checks pass. [Retained evidence](evidence/ui-pointer-focus.json). This is CPU
input verification; actual native interaction and appearance are not claimed.

The macOS managed text-client lifetime repair is separate. Logical focus
restoration and horizontal text-field scrolling remain open.
