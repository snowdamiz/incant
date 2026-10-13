# Restart integration pause point

The unchanged `7f6e41c4` integration source completed local checks and two fresh
ordinary shipping exports before the requested pause. Core and Planar each pass
12 copied-player processes, including whole-session restart, gamepad restart,
loss-to-win reset and held-input suppression. Each restart ends in the same state
as a fresh session and produces exact concatenated PCM. This is local producer
evidence awaiting independent final integration review.

The window/player library, shared runtimes and mobile library suites pass 355
CPU cases in 30 groups, with no failed or ignored cases. Six-package Clippy,
compiler-free/static compilation, strict TypeScript and dependency checks pass.
The initial static compile lacked its mandatory trusted bundle and exited101;
the unchanged-source retry passes after preparing the current guest with pinned
WABT. Both attempts remain retained. No static module execution is claimed.

The final Planar export was already active when pause was requested, so that
existing bounded chain was allowed to drain. No publication or new checkpoint
started. Actual main179 is `02203670`; its final worker documentation/ancestry
has not yet been merged into this tested private branch.

[Pause manifest](evidence/native-restart-main178-pause/pause.json) retains all
check logs, frozen inputs, proof receipts and inventories for the durable external
packages. On resumption, reconcile actual main, finish independent review and
only then publish the scoped restart PR. Native-window/device/GPU evidence,
physics snapshots and unsupported Wasm/headless Play restart routes remain open.
