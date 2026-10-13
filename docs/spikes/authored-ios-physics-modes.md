# Authored iOS archives with fixed project physics

The ordinary iOS exporter now honors the authored `deterministic` or `fast`
physics setting. Implementation and probe commit
`818ff1b2a4413ed813b30f19f6db06e5fc4b6e24` is based on actual main178
`15f31d0375983ac3eac82086cd6a2379ad6adb46`. Independent source and probe review is
accepted; the completed execution packet is pending independent final review.

`tools/export_ios.py` selects the fixed Cargo features through the shared
physics helper, with defaults disabled. It checks the selected normal dependency
graph, all five actual target solver compiler artifacts, shipping codegen and
the authored producer's exact kernel identity. Compiler invocation matching now
requires the complete target token: the simulator triple cannot satisfy the
device triple's audit. The original target-prefix failure and passing regression
are retained. Historical deterministic helper defaults remain explicit.

This slice changes no Rust, Cargo manifest, player loop, bytecode ABI or C ABI.
The desktop cross-target guard and the iOS static-Wasm refusal remain unchanged.
The separately accepted Android producer is not part of this frozen source base.

## Public authoring and execution

Both mode fixtures were created through public `incant_cmd` transactions:
captured WAV import, TypeScript imports, attached props, actors, audio and
`set_physics_mode`. Four Undo/Redo steps and persistent-journal reopen preserve
the project exactly. Matching native processes pass held, released and repeated
input cases before export.

Four fresh ordinary shipping archives passed. Both modes were linked through
the real C header into C and Swift callers. For each simulator archive, three
fresh processes per language ran after removing its private project, sources,
cooked cache, fresh build storage and source maps. Every positive process
matched the independent oracle and corresponding native result: 12 ticks,
36 full log records, 72 numerical checks and 76,800 nonzero PCM bytes. Across
both modes this is 12 positive processes, 432 log records and 864 checks.

| Mode | Target | Runtime archive bytes | Result |
| --- | --- | ---: | --- |
| deterministic | iOS simulator arm64 | 14,527,072 | 6 C/Swift positives, 16 exact refusals |
| deterministic | iOS device arm64 | 14,522,416 | unsigned C/Swift links only |
| fast | iOS simulator arm64 | 14,523,000 | 6 C/Swift positives, 16 exact refusals |
| fast | iOS device arm64 | 14,519,392 | unsigned C/Swift links only |

The 32 refusal controls include actual checksum-valid opposite-mode packages in
both directions and both languages. They return the exact cooked/compiled kernel
mismatch before game output or PCM. Other controls cover output no-clobber,
late guest failure, invalid ABI requests and corrupt package checksums. Each
owned simulator was shut down and deleted successfully.

Every archive preserves object DWARF separately and strips it from the runtime
archive. Unlinked archives can contain unused QuickJS parser definitions. The
actual dead-stripped C/Swift executables were separately audited: bytecode/runtime
entry points remain, source-parser entry points and DWARF sections do not.

## Checks and retained evidence

The focused Python suite passes 35 cases, including real failed-child output
retention, both modes and targets, hostile solver/producer identities and the
target-prefix regression. Strict TypeScript compilation passes for the three
probe fixtures. Matching deterministic and fast native binaries were built with
the locked CI profile. Shipping archives use actual optimized target compilation;
this evidence is not a new whole-workspace Rust test claim.

The first runner was paused at the user's request after the deterministic pair
finished. Its intentional pre-build refusal, original runner, marker and exact
pause receipt remain intact. The continuation rehashed and reused those two
archives, then built only the missing fast pair. A later pause marker was revoked
before the current device build completed and did not interrupt the run. No
successful archive was rebuilt. Continuation handle 32604 and producer re-audit
handle 80553 both exited zero and were reaped.

All 13,399 frozen repository inputs were unchanged at terminal validation. The
producer re-audit rechecked every compiler/producer/selected-graph assertion,
292 raw audit/caller command streams, archive and executable hashes, all exact
results, PCM, source absence and owned cleanup. The durable lossless archive
retains 713 records with 242 unique payloads; 18 large binaries remain separately
hash-bound at their durable local paths. The original failed target-prefix test
and intentional pause refusal are retained and distinguished from successful
execution.

- [Manifest and binary inventory](evidence/authored-ios-physics/manifest.json)
- [Lossless command, source, result and PCM evidence](evidence/authored-ios-physics/completed-evidence.json.gz)
- [Independent source/probe review](evidence/authored-ios-physics/authored-ios-physics-source-20261012.json)

Original evidence is under
`~/.codex/incant-evidence/authored-ios-physics-shipping-001`; its `audit.py`
re-evaluates the retained proof without rebuilding or launching a simulator.
The manifest binds the archive and original binary paths. Post-proof documentation
may differ from the frozen tree; the audit explicitly records any exact Git-object
fallback instead of claiming unchanged current files.

## Limits

Actual execution is local arm64 iOS 27.0 simulator CPU work. Device artifacts are
unsigned compile/link only. Archive objects record mixed minimum versions
14.0/27.0 for simulator and 10.0/27.0 for device; neither the object metadata nor
the caller deployment target proves an older OS can execute the package.
There is no physical-phone, performance, rendered-output, mobile window/audio
host, signing, store or phase-gate claim. Static Wasm mobile export remains
unsupported. Current-main integration and final independent execution review
are separate from this producer proof.

## Retained final producer review

The independent final audit is [retained verbatim](evidence/authored-ios-physics/final-review.json), SHA-256 `3610ba63a599b8ed994b8c96267e27cc2472a665f48d8ce2e5afaa652d93a21e`. It accepts the original main178-based producer archives and callers. The later Android, session-restart and streaming-PCM composition requires separately identified current-main checks; this receipt does not relabel those original binaries.
