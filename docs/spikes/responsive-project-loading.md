# Responsive native project loading

A macOS native review found a project file open under Documents stalled before
any window initialized. The sampled stack was inside the OS file-open call;
its exact permission cause is not established. Temporary-directory projects
opened normally. No OS permissions were changed.

Project read, validation and persistent command-journal recovery now run on a
worker after the native window and webview have been created. The UI receives a
loading state immediately and a terminal state via an event, with a fresh read
after the event listener is installed to avoid missing completion during setup.
Reads that return out of order cannot replace a newer state. Failed reads clear
stale document state and disable document commands while retaining saved account
metadata and native window controls. All successful edits and history remain on
the same command bus.

A 15-second deadline reports a typed project timeout. It cannot cancel an OS
file read portably; at most one background read remains per process. Its late
result is discarded and any acquired journal lock released. Reopening is the
current retry path. This does not prove protected-folder access works.

Local checks: three Rust behavior tests verify a blocked loader leaves state
readable and commands unavailable, timeout rejects a late result and releases its
journal, and restart restores shared command history while rejecting invalid files
and duplicate journal ownership. Three bridge regressions cover completion during
initial IPC, obsolete responses, stale-state removal and independent account
metadata, and explicit failure when native event subscription is unavailable.
Renderer failures now also notify the bridge immediately. The 226 editor/bridge tests pass. Claude owns the loading/error
presentation and native keyboard-dialog focus ring in handoff 0009. Those UI and
native checks are still pending.
