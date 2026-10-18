# Authored native Wasm coroutine export

The normal authoring and export route now runs an owned, resumable Rust SDK
coroutine in a source-free wasm2c shipping player. Saving at tick 13 and loading
in fresh processes preserves the exact routine, owner, frame, schema, deadline
and coexisting timer. The continued gameplay, final checkpoint and complete audio
output equal an uninterrupted run. This qualifies the fixed plain-scene route;
complete SDK parity and rendered-game performance remain separate requirements.

## Producing route and observable behavior

Native source is `4eeca4c60cf7f371f20bc9a6531adff4c0d41119`; the copied player
SHA-256 is `26327fcb706809ab0ce450d450c542815c681c74d9671102e585ac0e036e77f4`.
A fresh normal headless build imports the actual SDK guest and WAV through public
commands, atomically authors the scene, and verifies Undo/Redo, save and journal
reopen. The ordinary exporter builds its own cooker and shipping player with
fat LTO, one codegen unit, abort on panic, split symbols and the existing
dependency/symbol boundary checks. Its private source/build storage is removed
before player startup.

At tick 1 the guest starts `audio-route`, owned by entity 2, and a second routine.
At tick 2 it cancels the second routine before its first callback. The owned routine first resumes at tick 2,
then issues gain, seek, stop, restart and gain commands at ticks 9, 20, 26, 29 and
34. Each resume validates its exact identity, owner, deadline and explicit frame.
A periodic timer due at 101 coexists without firing during this 40-tick route.
The independent checkpoint oracle requires frame 2 due at 20 in the tick 13 save,
and frame 6 due at 134 in every final save, with the declared schema and owner.
Timer-only and legacy state cannot count as coroutine evidence.

Four fresh processes execute the 40-tick baseline, 13-tick prefix and two
27-tick continuations. All input-dependent observations match independent
arithmetic; all three final checkpoints are byte-identical. Joined prefix and
continuation PCM equals the complete baseline. All four complete PCM outputs also
equal the earlier [timer-driven authored proof](native-wasm-timers-export.md),
whose original producer remains `16845606`. This compares bytes, not execution
speed or physical audio output.

Six additional processes refuse corrupted integrity, a different module,
incompatible state schema, mismatched clocks and existing PCM/save destinations.
Expected errors, preserved destination bytes/modes and absence of staging files
are checked. PCM and save publication remain separate no-clobber operations.

## Verification and limits

The one native attempt completed in 729.687 seconds with actual tool exit zero.
All 37 managed commands and two RPC owners closed; a fresh post-exit check found
all 555 observed PIDs and process groups absent, including the wrapper. Frozen
source and tool inventories matched before and after execution.

The fresh headless target used 2,370,895,872 allocated bytes. Sampled private
scratch, retained output and RSS peaks were 3,765,587,968 / 376,328,192 /
4,459,593,728 bytes, within the original 12 / 8 / 2 / 8 GiB target/scratch/output/RSS
bounds. Two Cargo jobs and two Rayon threads were used. Earlier implementation
and failed-experiment costs remain recorded separately; this is additional work.

The [manifest](evidence/native-wasm-coroutines-export.json) binds 212 inert records
in a 3,084,244-byte compressed archive: native source/tool inventories, commands,
public RPC traces, package identities, all PCM/checkpoint bytes and terminal
closeout. Native binaries, symbols and assets remain external. Portable replay
executes no archived code. Eleven evidence controls cover bounded input reads, failed terminals, wrong
producers, missing processes/data, altered logs, unsafe paths and coherently
rehashable timer, deadline, schema, owner and frame corruption. The earlier
timer and explicit-state archives remain unchanged and pass their existing
controls against the extended format-aware oracle.

```sh
PYTHONPATH=tools python3 -B -m probes.native_wasm_checkpoint.coroutine_portable
python3 -B docs/spikes/evidence/native-wasm-coroutines-export-tests.py
```

The scene uses 48 kHz audio at 60 Hz simulation and has no physics or game UI.
Fractional tick/frame phase, arbitrary guest-memory snapshots, cross-build save
compatibility, physical devices, appearance, frame budgets and phase approval
are outside this result. Restricted runtime PATH and removal of owned source/build
inputs establish the tested source-free route, not an OS filesystem sandbox.
