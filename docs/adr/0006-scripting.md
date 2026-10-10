# ADR 0006: Sandboxed script hosts on the native runtime world

Date: 2026-10-08. Updated: 2026-10-10. Status: runtime architecture adopted by
the director in PLAN.md revision 4. The document-backed play path is superseded
and remains implemented only until migration passes its behavioral checks.

## Decision

Use TypeScript as the default gameplay language, compiled with pinned SWC and
cooked to version-matched QuickJS bytecode for native exports. Web exports run
build-transpiled JavaScript in an isolated browser worker. Sandboxed WebAssembly
modules, Rust first, provide the performance tier; share the schema-generated
SDK, capability boundary, query semantics and conformance suite across hosts.

Gameplay runs on a native Bevy ECS world instantiated from cooked binary scenes.
Bulk typed component columns are the default script API. Scalar convenience APIs
use the same runtime data access. Spawns, despawns and component additions enter
a structural command buffer committed at a scheduler sync point. Gameplay never
uses incant_cmd, clones the authoring document or serializes it per tick. The
editor, CLI and agent observe explicit world snapshots. Binary saves capture
runtime state; Stop discards the world. Keeping an inspector edit is an explicit
authoring transaction with provenance.

Typed views must have bounded lifetimes and checked element types. Structural
changes cannot invalidate borrowed views while a script executes; a JS object or
Wasm memory offset must not expose an unchecked Rust pointer. Runtime schemas
and codecs must be independent of incant_doc. The shipped runtime's transitive
dependency check excludes document/CRDT, command bus and agent code.

QuickJS retains no OS, filesystem, network, shell or native-plugin capabilities.
Wasm imports expose only granted engine capabilities, with bounded memory and
execution. Compile Rust modules offline in a sandbox against vetted vendored
dependencies and a pinned optional toolchain. No arbitrary build scripts or
procedural macros. Trust only locally produced or authenticated bytecode/native
artifacts; content hashes do not authenticate attacker-supplied executable caches.

## Evidence and implementation boundary

incant_script currently exercises deadlines, failed transaction rollback, hot
reload with type-compatible state, and live Bevy query projection. One batched
behavior updates 1,000 entities for 120 measured ticks on an Apple M5 Pro. That
is evidence about the old implementation only. PlaySession still clones a Project,
executes commands, serializes the whole project into QuickJS, and syncs the ECS
each tick. Its JSON saves and per-tick path must migrate, preserving observable
physics, navigation, input, timers, localization, save/resume and failure behavior.

## Consequences and revisit trigger

The earlier decision to simulate through the command bus is superseded. Do not
extend it with more gameplay systems. Begin with independent runtime component
types, a checked binary cook/load boundary and the dependency guard, then migrate
systems and script hosts. Keep the old path explicitly labeled until replacement
passes the existing behavior suite; an empty runtime crate is not completion.

Wasm editor/desktop/Android JIT and hot reload, iOS AOT code linked into the signed
binary, and browser-native Wasm all need target-specific proof. Wasmtime's ordinary
serialized AOT loader is not evidence of an iOS-compatible static link/signing
path. Deterministic Wasm also requires canonical floating-point/NaN behavior,
stable iteration and deterministic imports; disabling relaxed SIMD alone is not
sufficient. Static Hermes/typed-TS-to-Wasm remains an evidence-gated Phase 1 spike
(open decision 18), not an adopted runtime.

The script gate is 2 ms/frame at p95 for 10,000 scripted entities on iPhone 13
and Pixel 6 in the shipping profile. Desktop benchmarks do not pass it. Deadlines
and memory limits remain denial-of-service safeguards, not a proof against VM
vulnerabilities. See the [migration review](../spikes/performance-plan-v4-review.md).

Source: [PLAN.md](../../PLAN.md), sections 2, 3, 5 and 6.
