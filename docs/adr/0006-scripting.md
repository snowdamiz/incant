# ADR 0006: Sandboxed script hosts on the native runtime world

Date: 2026-10-08. Updated: 2026-10-10. Status: runtime architecture adopted by
the director in PLAN.md revision 4; shipped WebAssembly path and the typed-TypeScript
rule fixed by the revision 5 decision record. The document-backed play path is superseded
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

Shipped native builds translate modules to C with wasm2c and compile them into
the signed binary on every native platform (decision 20). This needs no executable
memory at runtime, so it is the iOS route, and using it everywhere keeps one
shipped code path. Wasmtime with JIT and hot reload serves the editor and
development builds; browsers run native WebAssembly. Existing target builds remain required. Under the director's 2026-10-13 decision,
the Phase 1 device gate requires actual execution only on this Mac; other-device
execution is deferred, nonblocking follow-up. Wasmtime's
ordinary serialized AOT loader is not evidence of an iOS-compatible static
link/signing path, which is why it is not the shipped route. Deterministic Wasm also requires canonical floating-point/NaN behavior,
stable iteration and deterministic imports; disabling relaxed SIMD alone is not
sufficient. Static Hermes/typed-TS-to-Wasm remains a Phase 1 spike, not an adopted runtime.
Decision 18 fixes the adoption rule: adopt it for shipped builds only if it passes
100 percent of the SDK conformance suite, runs at least twice as fast as bytecode
on the matched shipping-profile script benchmark on this Mac, needs no JIT on iOS,
adds under 5 MB to the mobile binary and has an upstream release line with security
fixes. The iOS architecture/size clauses use build/inspection evidence; reference-
phone speed comparisons are deferred and do not block the decision.
Otherwise TypeScript stays on bytecode. Record the outcome here at the Phase 1 gate.

The script gate is 2 ms/frame at p95 for 10,000 scripted entities on this Mac
in the shipping profile, recording model/OS and a matched baseline. Phone
measurements are deferred, nonblocking follow-up and are not inferred from Mac results. Deadlines
and memory limits remain denial-of-service safeguards, not a proof against VM
vulnerabilities. See the [migration review](../spikes/performance-plan-v4-review.md).

Source: [PLAN.md](../../PLAN.md), sections 2, 3, 5 and 6.

A bounded [development Wasmtime implementation](../spikes/development-wasm-host.md)
now executes actual modules through native typed transfers and structural commands.
It establishes a tested first ABI and development-only dependency isolation, not
zero-copy access or complete SDK conformance. The first
[static wasm2c slice](../spikes/shipping-wasm2c.md) now shares that copied ABI and
executes a trusted build-linked module in a compiler-free macOS shipping player.
It has bounded fuel, C-contained traps and numeric/structural rejection tests;
MSVC, other-platform execution, full exports/SDK coverage, deterministic
arithmetic remain separate requirements. Historical other-device gaps are
deferred, nonblocking follow-up under the 2026-10-13 policy; they are not passed.
