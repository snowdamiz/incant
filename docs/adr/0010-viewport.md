# ADR 0010: Native surface behind a transparent child webview

Date: 2026-10-08. Status: proposed for director review.

## Decision

Create a native wgpu surface on the Tauri parent window and overlay transparent web chrome. The bridge reports viewport bounds in CSS pixels plus device pixel ratio; the host validates physical bounds. Screenshots use a separate actual GPU readback path.

## Evidence and implementation boundary

The Rust native host compiles on macOS. Native composition and Windows behavior need end-to-end evidence before this architecture can pass Spike 1.

## Consequences and revisit trigger

macOS transparent webviews use Tauri’s private API feature; browser and app-store distribution constraints need review. Child-surface behavior, scale changes, resize, occlusion, hit testing and surface loss require per-OS tests. If these fail, evaluate shared-texture transport or egui in a new ADR.

Source: [PLAN.md](../../PLAN.md), sections 2, 3, 5 and 6.
