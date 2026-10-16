# Authored Wasm restart through ordinary shipping export

The ordinary exported plain player performed three cold replacements in each
of three fresh processes after its private project, source files, Wasm cache,
compiler copies and build storage were deleted. This is an actual desktop
shipping export result, separate from the earlier [native host tests](native-wasm-restart.md).

The producing commit was `c81eb7a95ce5fa9801a41e85f46f73d3d693dfdc`, tree
`a3b399a185132bba341728f8823d049ced987dde`, on `aarch64-apple-darwin` with Rust
1.99.0 and WABT 1.0.42. The run completed at 2026-10-16 01:38 UTC (October 15
locally). Later main376/main377 joins and this documentation are not a new native
qualification. The proof adds a bounded probe; it does not change restart behavior.

The unchanged, previously qualified Rust SDK `rust-session-once` guest was reused,
not recompiled in this run. Its original SHA-256 is
`f46df26fad21d80ed817a5a63ae77f88d8920adcbeaae46fd8b343cd75cede70` and its metered
SHA-256 is `568f8dc69d95d25ebbd1b62b266f7b865764b3d25db4e261ce5a8691a3ceaf0d`.
The archive binds the original guest preparation, all 36 SDK/ABI source identities,
the 25 changed runtime/guest files from host qualification, and the complete
22,152-entry shipping producer inventory. All relevant reused source bytes match.

The probe initializes an empty project and uses public import twice, requiring a
true cache hit with unchanged revision on reimport. Public `command.execute`
creates the witness entity and selects the Wasm program atomically. Undo/Redo,
clear-selection Undo, save, graceful RPC close and a fresh journal reopen must
preserve the selected project. It then calls the ordinary fresh-only
`export_native.export(mode='headless')`; there is no shared build-session helper,
fixture feature or runtime module-selection flag.

| Actual stage | Observed result |
| --- | --- |
| Normal headless binary | Fresh `ci` build; `default,physics-deterministic,wasmtime-host` |
| Ordinary export | Fresh capture cooker, wasm2c translation and shipping player |
| Player feature closure | `project,render-input,physics-deterministic,static-module` |
| Shipping audit | Runtime/script/player library and executable opt-3, one codegen unit, panic abort; executable fat LTO; no authoring/JIT normal dependencies or source compiler symbols |
| Exporter startup after deleting build storage | One replacement, generation 2, host tick 1, reset simulation tick 0, 800 PCM frames |
| Three subsequent source-free processes | Each: three replacements, generation 4, host tick 3, reset simulation tick 0, 2,400 PCM frames |
| Default `native-play` | Expected refusal: host does not accept session restart requests; no PCM published |
| Static checkpoint request | Expected `Checkpoint(Unsupported("static module state ABI"))`; no checkpoint or PCM published |

Each accepted step observes witness translation `[1,0,0]` before replacement;
each final world has translation `[0,0,0]`, identity rotation/matrix, unit scale
and no parent or velocity. Boundary output counts are exactly 800, 1,600 and
2,400 frames. Reports are identical across all three processes, as are their
19,200 bytes of silent stereo f32 PCM. This silent fixture does not prove audible
audio continuity. The guest compares an actual linear-memory counter against
world translation and requests once per VM: reusing a VM after resetting the
world cannot produce these three accepted replacements.

The copied headless SHA-256 is
`29efb6c0dab4c994b91ae11d04eb6172a40c4dd4bf84da932d863e9c850079eb` (75,362,944 bytes).
The shipping player SHA-256 is
`9deb15477ad87957cc2ddfed882aef4d237e11913ef6a71eb6d05436eade5a6b` (9,750,240 bytes).
Native artifacts remain external under
`~/.codex/incant-evidence/authored-wasm-restart-export-002-xime0yvf/evidence`;
the [manifest](evidence/native-wasm-restart-export/manifest.json) binds their
identities. The runtime package contains only the player and five cooked data
files. Replay children use a player-only PATH, absent HOME and private temporary
directory. This is a controlled source-deletion experiment, not a filesystem or
network sandbox against a hostile executable.

The once-only wrapper exited 0 after 420.905 seconds. Its 27 managed calls include
two intentional exit-1 refusals; two RPC owners bring the process-group count to
29. All groups and the wrapper were positively absent at closeout and independently
rechecked by root. The original FONT account was retained: 63,801,221,120-byte
allocated baseline, 65,948,704,768-byte ceiling, 64,799,830,016-byte sampled peak.
Scratch peaked at 3,739,807,744 bytes and retained outputs/logs at 377,200,640 bytes
(including the packet reserve); sampled owned RSS peaked at 6,942,556,160 bytes.
The 4,200-second attempt, two jobs/Rayon threads, 8 GiB scratch/RSS and 2 GiB
retained limits were unchanged. All prior target costs and failures remain bound.
The earlier mixed logical/allocated repair delta is explicitly invalid and is
not a charge. These are sampled trusted-process guards, not kernel-hard quotas;
cleanup may exceed a nominal execution deadline.

The portable packet contains 143 UTF-8 records, including source as inert text,
raw Cargo/RPC/output logs and hex-encoded PCM. It is about 2.41 MB compressed;
the verifier bounds compressed/expanded input at 8/64 MiB. It checks inventory
and hashes, replays the public transaction and exact runtime oracles, and
independently re-audits actual build features, codegen, dependency graph and
symbols. It does not execute archived code or load native artifacts. Historical
filesystem deletion and process absence remain receipt/source-supported
observations, not something a portable replay can observe again. Hashes bind
records; they are not authentication against a coherently fabricated archive.

From the repository root, data-only replay and its 19 controls are:

```sh
python3 -B docs/spikes/evidence/native-wasm-restart-export/verify.py
python3 -B docs/spikes/evidence/native-wasm-restart-export/test_verify.py
```

Negative controls rehash changed records and receipts before checking missing
boundaries, stale VM/world state, booleans as counters, wrong PCM, broken
Undo/reopen, false refusal success, unclean children, weaker codegen, fixture
features, compiler/authoring leakage, source files in the runtime inventory and
resource overruns. The earlier 20 portable probe/routing/capture controls also
remain passing. The expected refusals and earlier unexecuted packet are retained.

This closes this authored plain-player export/restart route. It does not establish
Wasm save/restore, development `native-play` restart support, native-window/GPU
or device-audio behavior, signing, phone performance, full SDK parity or a phase
gate. Static checkpoint refusal remains explicit; cold restart is not restoration.
