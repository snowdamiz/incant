# ADR 0006: SWC TypeScript on bounded QuickJS

Date: 2026-10-08. Status: proposed for director review.

## Decision

Compile TypeScript with pinned SWC to isolated CommonJS behavior modules. QuickJS has no OS, filesystem, network, shell, module loader or native-plugin capabilities. Script output consists only of validated commands and serializable state.

## Evidence and implementation boundary

incant_script exercises deadlines, failed transaction rollback, hot reload with type-compatible state, and live Bevy query projection. One batched behavior updates 1,000 entities for 120 measured ticks; see the spike report for limits.

## Consequences and revisit trigger

Runtime simulation uses the same command implementation without appending every frame to authored CRDT history. Deadline and memory limits are denial-of-service safeguards, not a proof against QuickJS vulnerabilities. V8 is deferred unless profiling requires it. Per-entity isolated VM performance remains unmeasured.

Source: [PLAN.md](../../PLAN.md), sections 2, 3, 5 and 6.
