# Revision 5 decision review

Reviewed 2026-10-10 after the director's second plan update arrived while revision
4 was in PR #43. This review supersedes references to open decisions in the
[revision 4 migration review](performance-plan-v4-review.md). The native-world
migration order still applies; the shared component-type extraction has begun in
`impl/runtime-component-types`. No new performance or phase gate has passed.

## Adopted decisions and effect on work

- All 27 Section 12 decisions are recorded as settled. The numerical rules are
  executable acceptance criteria, not reasons to ask the director again:
  TypeScript AOT must pass all conformance cases, be at least twice as fast on
  both phones, need no iOS JIT, add less than 5 MB and have maintained security
  releases. Switch Rapier to Jolt only for a 1.5× p95 gain on both phones and a
  maintained shim that builds all six targets. Record measured outcomes.
- Wasmtime serves development/JIT. Native exports use a pinned wasm2c toolchain,
  compile generated C into the application and retain the same sandbox/import
  contract. Browser builds use native Wasm. The Phase 1 module work now proves
  this selected route rather than selecting an iOS route. Compiler settings,
  bounds/traps, floating-point behavior and host conformance remain tests to run.
- Enable Rapier parallel stepping first in the native deterministic build and
  compare its exact existing behavior before adding the fast build. Preserve
  serial browser builds until a supported worker pool exists. The pinned 0.36
  `parallel_path_parity` test explicitly covers serial Wasm/native-parallel
  deployment; Incant still needs its own cross-target checks.
- Browser editing, TypeScript, agent access, browser play and web export stay in
  1.0. Native export, module compilation and native plugins stay desktop-only.
  The current browser UI fixture is not the delivered browser engine.
- WorkOS AuthKit/Connect supplies optional engine identity; the game player
  service remains self-hostable and independent. AWS/OpenTofu, host-authoritative
  relay sessions, optional player accounts, and the listed launch regions are
  fixed implementation inputs. No accounts or services were created by this
  review. Standard OIDC preserves the client protocol; replacing identity
  providers still requires issuer, identity, entitlement and session migration.
- MIT OR Apache-2.0, Incant Cloud, Driftwake pricing/name rules, a separate private
  game repository, public Core Sample, community support, external audits,
  dependency license checks and ufbx conversion are retained as requested.
  The provider-selection rule remains gated on valid output and suitable terms;
  a saved OpenAI login does not itself prove every media endpoint is available.

## Review corrections and evidence

WorkOS now uses Magic Auth email codes; its Magic Link product is deprecated.
Connect supports OIDC clients, including public PKCE applications. The plan and
ADR 0012 now name that route, keep client secrets out of the editor, and avoid
claiming identity migration needs no work. Sources:
[Magic Auth](https://workos.com/docs/authkit/modeling-your-app) and
[Connect OAuth applications](https://workos.com/docs/authkit/connect/oauth).

The Rapier correction was checked against the installed pinned 0.36.0 Cargo
manifest, compile-time `simd8`/`enhanced-determinism` exclusion and upstream
`tests/parallel_path_parity.rs`. Both native variants can use `parallel`; widening
SIMD is target/benchmark dependent. Upstream evidence is not an Incant phone
benchmark. See [the pinned source](https://docs.rs/crate/rapier3d/0.36.0/source/).

wasm2c emits C that must be built with settings preserving Wasm traps and floating
point semantics; selecting it does not prove those properties for our bindings or
toolchain. See [wasm2c's compiler guidance](https://github.com/WebAssembly/wabt/blob/main/wasm2c/README.md#compiling-the-wasm2c-output).
The [ufbx license](https://github.com/ufbx/ufbx/blob/main/LICENSE) offers MIT as an
alternative, consistent with decision 26; actual dependency notices/auditing are
still implementation work.

The skill total is corrected to eighteen: four character-controller skills plus
fourteen others, including the new profile/optimize skill. ADR 0009 now describes
virtualization as required work, not as something already delivered.

One operational mismatch remains explicit: `gh repo view --json visibility`
reported the engine repository as **PUBLIC** during this review, while the plan
describes initial publication at the Phase 8 beta. This review does not change
repository visibility or claim that existing public history can be made private.
Keep commercial Driftwake content out of the engine repository; its separate
private repository remains the selected production arrangement.

## Verification and remaining work

The plan, all fourteen ADR updates, ADR index, Phase 0 decision ledger, generated
instructions and STATUS are reviewed together. Convention generation/check,
tool tests, local links and whitespace checks are required, followed by hosted
checks on the actual revised PR head. The original revision 4 checks cannot be
reported as checks for revision 5.

Runtime separation, binary scenes/saves, bulk scripts, bytecode, wasm2c modules,
physics variants, pipeline cook/pre-warm, profiler/shipping builds, license audit,
browser engine and reference-device measurements remain unfinished. Signing,
hardware purchases, account creation, contracts, pricing setup and phase approval
remain human-owned tasks. Settling the decisions does not perform those tasks.
