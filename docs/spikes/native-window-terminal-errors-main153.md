# Native window error preservation on main153

The reviewed shutdown policy is integrated with actual main153 `71559622a13826f7aa4454277fb19b4c73534ff0`.
Both source files remain byte-identical to `d24f7c63`; conflicts affected only
STATUS. All 1728 frozen build inputs remain exact after the documentation-only
main ancestry merge. All 57 player tests in three groups pass, with no failures or
ignored tests; native-window all-target Clippy, the authoring-free dependency
guard, formatting, conventions and diff checks pass.

The first typed game failure now survives later loop/cleanup failures. A terminal
error, explicit close or stop-after prevents later callbacks from accessing the
game, while surface and IME cleanup still run. Tests cover typed cause identity,
late callbacks, cleanup delivery and error precedence. The
[producer contract](native-window-terminal-errors.md) and its retained stale-target
build failure/remediation remain unchanged. [Integrated receipts](evidence/native-window-terminal-errors-main153.json)
pin the two sources, six commands, raw/compressed logs and full frozen inventory.

No window, GPU, capture, provider or device execution occurred in this integration.
This does not diagnose the historical excluded heap-instrumented failure, change
audio policy or establish new live IME/appearance acceptance.
