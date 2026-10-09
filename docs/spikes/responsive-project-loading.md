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
and duplicate journal ownership. Four bridge regressions cover completion during
initial IPC, obsolete responses, stale-state removal and independent account
metadata, concurrent edit/status responses, and explicit failure when native event subscription is unavailable.
Renderer failures now also notify the bridge immediately. The integrated 240 editor/bridge tests pass. Claude Opus 5.5 supplied the loading/error
presentation and keyboard-dialog focus ring through handoff 0009. Strict TypeScript,
production UI build and the custom-protocol native build pass. The integrated UI
is 93.55 kB gzip JavaScript and 8.38 kB gzip CSS. Browser pixel review and behavioral
checks are recorded in the handoff; a CSS-source mirroring assertion was removed
at integration, retaining behavioral and rendered-style evidence.

A follow-up CUA attempt still found macOS locked. Native verification of the new
focus ring, loading/error states and schema field order remains open. Browser
focus-visible suppression is explicitly a simulation, not native WebKit proof.
No account action or filesystem permission change was taken.

PR #5 merged as b2f7f18 after all twelve hosted checks passed on 2580b09. This
includes source checks, both Windows/Linux native editor builds/tests/readback,
three credential stores and all six platform probes. Exact jobs and merge metadata
are in [responsive-loading-2026-10-09.json](evidence/responsive-loading-2026-10-09.json).
Native Mac checks above remain open; CI results do not erase that limitation.
