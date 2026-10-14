# Native project Save and engine-pin review

The editor exposes the existing host-bound Save and engine-pin operations in its
project-title menu, with Command-S and an explicit review-before-apply dialog.
It sends no path or candidate document back to the host: applying uses only the
host-issued review ID. A single in-flight operation, project/bridge scope checks,
revision/history invalidation and revocable announcements keep stale results out
of a newly opened project. Transport uncertainty is explicit: a rejected or
malformed completion cannot claim that a write did not happen, and is never
retried automatically.

The main278 composition is source `d803cc2cc15d399bdf61cbf5743f5db81712f64b`.
Claude Opus 5.5 with Max effort authored the menu/dialog design and initial UI
implementation (`e824206a`). Astra took over unresolved correctness work under
the director's standing authorization: shared bridge outcome metadata
(`a5e862d6`, replay of `806e467f`) and its UI consumption, then project-bound
announcement cancellation (`d803cc2c`). The original Claude attempts, failures,
corrections and reviews remain in [handoff 0077](../../handoffs/0077-project-file-ui/final-intake.md).
The bridge/unknown-outcome change and announcement change have separate
independent source reviews; no author self-approved their own correction.

Final local checks pass 973 UI tests in 47 files, TypeScript, the production web
build and generated bridge consistency. The final browser run covers 30 states,
33 PNGs and five keyboard/pointer flows, with zero reported axe violations,
page errors, external requests or flow errors. The prior and final captures have
identical normalized state/flow reports and 32 identical PNGs; one transient
retired-dialog PNG differs. These are fixture results, not native persistence.

Root also built the actual arm64 editor with Rust 1.99, locked/offline dependencies,
CI profile and custom protocol. On this Mac, native Tauri/WebKit checks establish:

- Rename through the hierarchy, Command-S, exact saved file contents, and the
  saved name/history after quitting and reopening the project.
- An immediate Save overlapping the rename truthfully refused while the engine
  was busy. A later explicit Save succeeded; no automatic retry was used.
- An already-pinned project returns the actual host refusal. Escape restores
  focus to the project menu trigger.
- A byte-exact copy of the legacy empty template produces a read-only review at
  revision 0. Explicit Pin writes exactly the reviewed SHA-256 at revision 1.
  The original bytes remain in the host backup. Undo restores those original
  bytes, Save reports no difference at revision 2, and Redo restores the exact
  reviewed pinned bytes.

Native screenshots and accessibility observations were inspected through the
computer-use tool and remain in the task trace. The
[portable manifest](evidence/project-file-ui-main278/manifest.json) retains
build/asset identities, synthetic before/after files, qualification logs and
explicit native observations. It binds the external native executable
`0be02172b4ce09e66adea8c82df407753b30d909c7cb676857bf577587b01966`.
No account details or tokens are included. The first packet whitespace check
flagged original unified-diff context spaces and two raw log terminal blank
lines; exact-path Git attributes preserve those evidence bytes without relaxing
production source checks. An observation after the first quit
reopened an unintended no-argument instance; that owned process was closed and
the actual project-bound instance was used for the persistence check.

Scope remains Save and exact-current engine pinning of unpinned v1 projects.
Older-format conversion, Save As, native accessibility certification and other
platform UI execution are not established. No phase gate is approved.
