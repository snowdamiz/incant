# Public animation authoring and source-free CPU export

`tools/probes/native-animation.py` creates a one-joint weighted triangle with two
real glTF clips: two seconds of translation and rotation. Public import and one
atomic `incant_cmd` transaction register its model, TypeScript behavior, Animation,
string table and stationary camera. Undo removes the matching created source;
Redo restores it. Save and a fresh process reopen the same default
`project.journal.jsonl`, project and source. Native Play leaves authoring unchanged.

The actual strict TypeScript guest captures committed playback snapshots and
queues select, seek, playing, looping and speed controls. Twenty-three independent
numerical checks over 160 ticks cover pauses, both clips, loop boundaries, reverse
speed, rest selection, completion and restart. Numeric Transform writes expose
captured time/flags and persistent-state/timer counters to the public observer;
committed logs retain all four playback values. The guest checks that queued
controls cannot alter the captured snapshot. It also checks the retained locale
and catalog across checkpoint continuation.

The local aarch64 macOS proof exports a fresh `shipping` player with the normal
compiler/codegen, symbol and authoring-free dependency audits. Its private build
storage is removed before startup, then the probe deletes its own project,
TypeScript, glTF, buffer, journal and cache directory. Three fresh player processes
with a minimal runtime environment reproduce identical reports and checkpoint
bytes. A checkpoint after 41 ticks resumes for 119 more ticks to the same world,
persistent state, timers, locale, log sequence and PCM as the full 160-tick run.
Corrupt checkpoint and wrong-observation controls fail without publishing save
or PCM output. The actual cooked animated model remains in the copied package and
is loaded into CPU render input; no GPU device or window is created.

`--review-fixture` independently authors a retained project with a restarting
move/rotate timeline and stationary entity/camera for Claude's native review. It
is separate from the deleted export input. A private copy of that saved fixture
also passes fresh default-journal Undo/Redo of the entire authored transaction,
including source removal/restoration and identical recovered history. An earlier
fixture used two journals and was superseded by public reauthoring in a new
directory. An initial numerical expectation assumed script-before-step order;
the preserved failed run has only the first two checks wrong. The corrected
oracle follows NativeSession's actual world-step-before-update order.

Run with a current production-equivalent headless binary and a new private output:

```sh
python3 tools/probes/native-animation.py /tmp/incant-animation-proof --binary /absolute/path/incant_headless
python3 tools/probes/native-animation.py /tmp/incant-animation-review --binary /absolute/path/incant_headless --review-fixture
```

The evidence records exact inputs, commands, binary/package/checkpoint identities,
raw result receipts and both earlier harness corrections. This is local CPU
functional evidence. Rendered appearance, physical device/performance gates,
Wasm animation ABI parity, blending/state machines/IK/root motion and complete
animation tooling remain separate work. See
[the receipt](evidence/native-animation-export.json).
