# ADR 0005: Taffy layout and engine-owned widgets

Date: 2026-10-08. Status: proposed for director review.

## Decision

Retain Taffy for game layout, with a typed widget tree and schema-driven serialization. The editor web UI is separate from in-game widgets.

## Evidence and implementation boundary

No game UI runtime has been implemented. The React editor does not constitute this stack row.

## Consequences and revisit trigger

Input focus, screen readers, localization and mobile touch behavior require independent tests. Widget rendering and visual review route through Claude 5.5.

Source: [PLAN.md](../../PLAN.md), sections 2, 3, 5 and 6.
