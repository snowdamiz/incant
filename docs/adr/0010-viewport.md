# ADR 0010: Native surface behind a transparent child webview

Date: 2026-10-08. Updated: 2026-10-10. Status: decided by director instruction
in the PLAN.md revision 5 decision record. This does not approve the Phase 0
gate or claim later-phase work is implemented.

## Decision

Create a native wgpu surface on the Tauri parent window and overlay transparent web chrome. The bridge reports viewport bounds in CSS pixels plus device pixel ratio; the host validates physical bounds. Screenshots use a separate actual GPU readback path.

## Evidence and implementation boundary

Claude reviewed actual macOS native composition, resize and fullscreen behavior in handoff 0002. The director replaced manual Windows review with automated CI acceptance; Windows/Linux editor builds, shared tests and native GPU readback passed (docs/spikes/evidence/desktop-editor-2026-10-08.json). The later connected-panel revision has its own review and explicitly deferred native follow-ups in docs/spikes/connected-editor.md. This passes the spike under that acceptance change; it does not establish untested platform behavior or director approval.

## Consequences and revisit trigger

macOS transparent webviews use Tauri’s private API feature; browser and app-store distribution constraints need review. Child-surface behavior, scale changes, resize, occlusion, hit testing and surface loss require per-OS tests. If these fail, evaluate shared-texture transport or egui in a new ADR.

Source: [PLAN.md](../../PLAN.md), sections 2, 3, 5 and 6.
