# Native Wasm session restart

The Rust guest SDK's `Tick::request_restart()` now exposes deferred whole-session
restart through both build-linked wasm2c and development Wasmtime. The additive
`incant_session_v1.request_restart(epoch: i64) -> i32` import uses opcode 63.
All 63 preceding functions, existing namespaces and other registry fields are
unchanged; unknown namespaces and incorrect parameter/result signatures refuse.

Authority belongs to an explicitly opted-in native owner. Ordinary module hosts
refuse requests unless that owner enables them. Calls use the current tick epoch,
share the structural-command quota, and coalesce into one pending request. A
request does not interrupt its callback or replace the current world. Publication
requires accepted script/world changes and, for composed sessions, audio and
consumer acceptance. A rejected tick makes the session terminal and hides
the request. An unconsumed request survives subsequent successful ticks until
the owner takes it. Successful development reload preserves the owner's opt-in
but clears pending requests and starts a new guest instance; it does not replay
the request or migrate guest VM state.

`SessionProgram` is cloneable trusted startup data: compiled bytecode, validated
on-start development module bytes, or a build-linked static descriptor.
`GameSeed` retains it alongside the original cooked scene, action bindings and
immutable clip handles. Cold reconstruction drops the previous live composition
before using the ordinary constructors. It creates a fresh VM and world identity
and resets simulation, input and offline audio clocks. It does not copy live
capabilities or replay earlier side effects. The ordinary plain and CPU-rendered
player owners now retain Wasm seeds; their host-step/generation/output chronology
continues across replacement. Scene-recording and offscreen GPU owners remain refused.

Local qualification on 2026-10-15 used the `ci` profile on
`aarch64-apple-darwin`, Rust 1.99.0 and WABT 1.0.42. The tested producer is
`8e2c0a76373cdae87f2aa49912d94b6ec0b26efa` plus the exact 25 changed/untracked code
files recorded in the [evidence manifest](evidence/native-wasm-restart.json).
Documentation was added afterward. The six actual Rust guest variants are
ordinary restart, request-once, quota refusal, invalid structural owner,
callback refusal and trap. They were compiled to Wasm, passed through the current
cooker, translated with wasm2c, and exercised by the static and development hosts.

| Executed scope | Result |
| --- | --- |
| Shared capability transactions, epoch expiry, quota, delayed consumption and replacement | 6 passed |
| Actual Rust guest parity, cold reconstruction and late consumer refusal | 5 passed |
| Wasmtime owner opt-in, trap, reload and import admission | 3 passed |
| Production plain player restart owner: actual static guest and existing bytecode control | 2 passed |
| Existing composed hosting and restart regressions | 12 + 1 passed |
| Strict actual-host, player and default runtime Clippy | 3 stages passed |
| Python SDK controls | 19 passed |
| Formatting, generated SDK, conventions and whitespace checks | Passed |

The cold-restart guest checks a counter in its actual linear memory against the
world's transform. Reusing the old VM after resetting the world therefore fails.
The tests also compare reset world identity, held input and fractional output
counts. The player test reaches `prepare → enable → tick → accept` and records
three real replacements. The late-consumer test uses a looping audible clip with
matched accepting/refusing hosts on both Wasm backends: accepted output must be
audible; refusal must silence the entire buffer and hide world and restart state.

Two initial compile failures remain recorded: the request match arm lacked an
explicit `return`, and a test attempted to move a velocity out of an indexed
snapshot instead of borrowing it. Neither attempt ran tests. The corrected
capability and its six tests are byte-identical between runtime003 and the final
conformance source. Later differences add player coverage and the audible
fixture. Final checks did not automatically freeze the whole source; an
independent subsequent comparison matched every one of the 25 code files to the
conformance snapshot.

The [portable raw records](evidence/native-wasm-restart-records.json.gz) contain
107 UTF-8 log, JSON and runner-source records as inert JSON strings. Every record
has its original byte length and SHA-256; verification requires only decompression,
JSON parsing and hashing, never executing an archived runner. The manifest binds
the archive and its full index. External Wasm, generated C, tool and native test
artifacts remain hash-bound in the retained preparation inventory. Root receipts
record terminal wrapper exits and independently observed absence of all 33
conformance and six final-check child PIDs/process groups. Disk/log limits were
sampled guards, not kernel-hard isolation or an RSS bound; failed attempts and
existing target allocations are retained.

This is scoped CPU behavior evidence. It is not a shipping-profile player export,
source-free package run, native-window/GPU/device-audio result, phone or performance
measurement, full SDK-parity result, Wasm checkpoint restoration, or phase gate.
CPU-rendered owner wiring was reviewed but not separately exercised with a render
fixture here. The broader combined conformance command was updated to include
the player feature; only the focused commands in these records were executed.
The earlier [native session restart evidence](native-session-restart.md) remains
a separate historical bytecode/package result.
