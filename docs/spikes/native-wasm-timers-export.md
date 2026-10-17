# Authored native Wasm timer export

The normal authoring and export route now runs a timer-driven Rust SDK guest in
a source-free wasm2c shipping player. Pending one-shot and periodic timers survive
save/load into fresh VMs; the continued world, declared guest bytes, clock, final
checkpoint and audible PCM match an uninterrupted run exactly. This qualifies
this fixed plain-scene route, not complete SDK parity or performance.

## Native producer and behavior

The native producing source is `1684560629db00f822d8bd2eeb0e4b3bfdc01bce`.
The exact player SHA-256 is
`ba1fceb20117bd918ea46f1b132026b2fc2b579ae97b6791325c2232ddd76890`.
A fresh normal headless build imports the actual Rust guest and WAV through public
commands, then atomically authors two entities and selects the scene program.
Undo, Redo, save and a fresh journal reopen preserve the expected project. The
unchanged native exporter builds its own cooker and shipping player, with fat LTO,
one codegen unit, abort on panic, split symbols and the normal dependency/symbol
boundary checks. It removes its private source/build storage before startup.

The guest replaces an early stop timer with tick 26, cancels a never-delivered
timer, and uses a periodic gain timer at ticks 9 and 20 before cancelling it.
One-shot restart and gain timers fire at ticks 29 and 34. An additional periodic
timer due at 101 remains pending through both the tick 13 save and final tick 40.
The guest validates exact event identities and deadlines before changing audio.
The independent data oracle checks the canonical pending schedule and clock;
legacy timer-free state cannot count as this timer proof.

Four separate processes run the 40-tick baseline, the 13-tick prefix, and two
27-tick continuations from that prefix. Every observation matches independent
movement/state arithmetic. Prefix plus resumed PCM equals the whole baseline;
all three final checkpoint files are byte-identical. All four PCM outputs also
match the earlier equivalent [explicit-state native proof](native-wasm-checkpoint-export.md),
whose original producer remains `b733a366`. Timers therefore preserve the prior
observable audio actions. This is a byte comparison, not a timing measurement.

Six additional processes refuse corrupted integrity, a different module,
incompatible state schema, mismatched world/module clocks, and existing PCM or
save destinations. The precise expected errors, untouched destination bytes/modes
and absence of abandoned staging files are checked. Save and PCM publication
remain separate no-clobber operations; an existing save can leave newly published
PCM. The test does not claim a multifile transaction.

## Qualification and retained evidence

The one native attempt completed successfully in 562.900 seconds. All 37 managed
commands and two RPC owners closed; the actual execution handle returned zero.
An independent post-exit check found all 527 observed PIDs and process groups
absent, including the wrapper. Source and compiler identities matched their
frozen inventories before and after execution.

The fresh headless target used 2,370,691,072 allocated bytes under 12 GiB. Sampled
peaks were 3,758,768,128 bytes of private export scratch, 375,394,304 retained output
and 6,922,960,896 RSS, within 8 GiB / 2 GiB / 8 GiB limits. Two Cargo jobs and two Rayon
threads were used. The prior timer CPU account of 2,134.119 seconds remains
recorded separately; this fresh shipping run does not reset it.

The [portable manifest](evidence/native-wasm-timers-export.json) binds 209 inert
records: complete source/tool inventories, real stdout/stderr, public RPC traces,
authored reads, package provenance, PCM/checkpoint bytes, native result and
post-exit closeout. Native executables, symbols and assets remain external with
recorded identities. Bounded saved-data replay executes no archived code and
checks the actual shipping route, source-free dependency boundary, every timer
schedule, all observations, exact continuations and specific refusals. Nine
negative/positive evidence controls cover corrupted schedules, legacy state,
wrong producer, missing process coverage, failed terminal, altered streams,
missing data and unsafe record paths. The first packaging-only verifier attempt
refused an extra file-mode field in its manifest; fixing that serialization did
not change native records or rerun native work.

Run the portable checks with:

```sh
PYTHONPATH=tools python3 -B -m probes.native_wasm_checkpoint.timer_portable
python3 -B docs/spikes/evidence/native-wasm-timers-export-tests.py
```

The scene has no physics or game UI and uses 48 kHz audio at 60 Hz simulation.
Fractional tick/frame phase, arbitrary guest memory, cross-build save compatibility,
rendered appearance, device output, frame budgets and phase approval remain
outside this result. A restricted runtime PATH and deleted owned source/build
files establish the tested source-free route, not an OS filesystem sandbox.
