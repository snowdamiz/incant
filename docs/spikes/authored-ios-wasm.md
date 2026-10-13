# Authored iOS static WebAssembly export

The ordinary authored iOS archive exporter can compile one publicly selected,
captured Wasm scene program through pinned wasm2c 1.0.42. It uses the existing
native Wasm runtime, generated bulk SDK imports and automatic authored-program
selection. No fixture selector, arbitrary executable loader, JIT, runtime source
compiler or new SDK opcode is introduced.

The producer starts from a normal public project. Bounded `.wasm` import and
atomic scene selection use the command bus, including provenance, Undo/Redo and
journal reopen. The exporter resolves the same captured module, validates its
original/prepared/registry fingerprints, and translates the prepared bytes in
its private trusted build. The iOS build hook checks the complete six-file bundle
and exact generated-input hashes before embedding it. Missing capabilities or
bundle, stale identities, mixed per-entity TypeScript, unexpected files and
non-iOS static-module destinations are explicit refusals. Android TypeScript
admission does not extend the static-module destination allowlist.

Target validation preserves the project's deterministic/fast physics choice:
the selected normal dependency graph, actual five target solver artifacts,
shipping Rust code generation, explicit Apple C target and SDK, and the five
wasm2c runtime/module C compilation commands are audited. An additional exact
cc-rs configuration probe is recorded separately. The output's copied program
identity is checked against its actual captured source and scene.

The static archive includes unused QuickJS parser definitions before final
linking. Final dead-stripped C and Swift callers must retain the native entry
point and compiled scene module while excluding parser symbols and DWARF. This
is an executable audit, not a claim that all members of the unlinked archive
are runtime-only.

## Evidence boundaries

The public fixture uses an actual Rust SDK module with all-field copied audio
observations and typed native controls. A separate native world/mixer oracle
checks 96 ticks, six records and 186 scalars per tick (17,856 scalars per run),
including exact stable-ID words and u64 transport epochs. A bounded 248-vector
observation set supplements the full records. Two source-deleted Wasmtime
processes in each physics family establish the development baseline. The
simulator callers compare every record and the complete 614,400-byte PCM output
against those baselines.

Twelve successful fresh simulator processes (three per language and physics
family) match every scalar and PCM byte. Fifty-two exact refusal controls pass,
including valid opposite-mode packages in both directions, short snapshot and
command buffers, invalid gain, late invalid owner/role/seek, no-clobber, malformed
C ABI requests and package checksum failure. Both device archives link with C
and Swift, without device execution. Final executables retain the compiled
module and exclude parser symbols and DWARF. Both successful simulator suites
shut down and deleted their owned UUIDs. The retained manifest separates native
preflight, archive compilation, actual iOS simulator execution and unsigned
device linking. Local simulator CPU results do not satisfy a physical-phone,
minimum-OS, performance, rendering, mobile window, live audio, signing, store or
phase gate.

## Checks and immutable inputs

The cooker suite passes 60 Rust tests in 17 groups; scoped all-target Clippy,
workspace formatting and diff checks pass. The tools suite reports 99 cases
with four intentional skips. Nineteen focused exporter/Wasm checks pass after
the observer correction, including the actual-transcript regression.

Production source is `decd11ca78603626e2caa90eb13a6e561ebfd3b3`; the observer-only
correction is `791a45a8afbeb998777c474329c9b8e5fb24dff7`. The original build froze
13,408 inputs; the three later archives froze 13,409, with 13,405 original inputs
unchanged and three observer/test paths changed. All Rust/C/ABI/build settings
remained unchanged. The additional input is the real compiler transcript fixture.
Source and probe reviews are accepted. The complete final execution packet is
pending independent review; current-main integration is separate.

| Physics | Target | Stripped archive bytes | SHA-256 |
| --- | --- | ---: | --- |
| deterministic | simulator | 15368304 | `f0a49990e9682abdca13c0e8b8dec94af91a8985439988f236f1c4870ab78a44` |
| deterministic | device | 15363864 | `61b35b186a99229be4f91cf423e7e7d492692732682e7d94d9aa4c098cf943e2` |
| fast | simulator | 15364072 | `ef1ac9d863ff22b08d7cd231ec07584a450a2703582946d88596dbf37749d91b` |
| fast | device | 15360968 | `b7b19c8a70a27453ec2e988108c7d4acc45ca5118c285907f3be83be0cae3691` |

The lossless [evidence manifest](evidence/authored-ios-wasm/manifest.json) binds
raw command streams, producer receipts, frozen source inventories, native
oracles, all successful and failed caller reports, and external binaries.
The retained `audit.py` re-evaluates compiler, package, full-field, PCM and exact
refusal assertions without rebuilding or launching a simulator.

## Preserved observer failure and completed-output reuse

The first deterministic simulator compiler completed successfully. The frozen
exporter then refused because its observer searched stderr for cc-rs commands;
Cargo had placed them in stdout alongside JSON records. The real output also
contained a bounded cc-rs O0 flag probe before the five O3 runtime compilations.
The refusal and all original command streams remain retained. A regression uses
the normalized actual transcript and rejects unexpected probe paths and target
artifacts.

The completed private build was cloned before the exporter's cleanup. After the
observer-only correction, a scoped recovery helper checked the exact original
archive, compiler completion, frozen inputs, generated module files, target
physics artifacts and original compiler versions, then used the ordinary
packaging helpers. This is reused compiler output, not another fresh build.
The unstripped archive remains retained and its private clone was removed.

The original compiler invocation path and version stdout were recorded, but a
separate compiler-binary SHA was not persisted before the observer refusal.
The recovery receipt labels its compiler SHA as measured during re-audit;
historical compiler-binary hash equality is not claimed.

## Limitations

This slice demonstrates one selected scene module with the shared native bulk
runtime. Mixed per-entity TypeScript is refused. Static guest checkpoints remain
unsupported. An unknown `save` field is tested only as a C ABI refusal, not as a
guest checkpoint implementation. Exact late-failure reports and absent PCM
measure the public output boundary; callback rollback is covered separately by
the existing host conformance suite. No all-SDK-on-mobile, physical-device,
reference-phone or complete Phase 1 claim follows from this probe.

## Preserved simulator infrastructure failure

The first fast simulator attempt matched all six positive runs but later
`simctl spawn` returned status 124 with `Domain is tearing down`. The cleanup
reported that the owned simulator was already shut down, then deleted it
successfully. That incomplete suite is retained and is not accepted refusal
evidence. A fresh owned simulator reran the full fast suite against the same
archive and unchanged sources, passing all 32 controls and normal cleanup.
The prior deterministic suite and all completed archives were preserved.
