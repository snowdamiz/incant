# Explicit native Wasm persistent state

The opt-in route stores bounded guest-selected bytes in the native capability
owner. It does not capture Wasm memory, globals, tables, pointers or closures.
Native CPU qualification passed on macOS arm64: 69 test executions and seven
strict Clippy stages across the retained producing revisions. An ordinary
authored shipping export with save/load remains pending.

A validated `incant_state_v1` custom section declares version 1, a nonzero
32-byte schema identity and 1–65,536 bytes. Additive ABI operations 64/65 read
and write the region using the current tick epoch. Earlier operations 0–63
and ABI version 1 retain their signatures. Missing, duplicate or malformed
declarations refuse admission before startup.

Each opted-in host owns committed and staging buffers. The first write copies
committed state; successful native transaction completion swaps the buffers.
Aborted updates discard staging. Reads see earlier writes. Current-epoch
errors are sticky even if a guest ignores them. Stale epochs are rejected without
setting the capability owner’s current persistent rejection flag. Both native
host wrappers retain their existing terminal behavior on a rejected guest import;
this does not promise that guest execution continues after a stale call. The region has its own 256-call/1-MiB transfer limits and
shares the existing native capability limits. Non-opted hosts allocate no state
buffers. Guest creation and start functions have no tick authority.

Capture is a non-calling read of committed bytes at a healthy, inactive, exact
world/module tick boundary. Pending restart requests refuse capture without
being acknowledged. Restoring constructs a fresh candidate VM and native world,
validates trusted startup identity, declaration, snapshot lengths, tick and epoch
watermark, then restores explicit data before publication. Existing world/UI/audio
preparation and rollback boundaries remain in force. Ordinary input restarts
neutral; the owning game's output/audio clock is reconstructed by its existing
checkpoint path. Restart opt-in comes from the receiving owner, never save data.

Same-layout hot reload initializes and validates the candidate before replacing
the old VM, retaining committed bytes, accepted tick and revocation watermark.
Different schemas/lengths, pending restart or a failed persistent host refuse.
Native checkpoint bytes bind backend, target, source/runtime identity and the
state declaration; trusted linked Wasm additionally binds original/prepared Wasm
and generated native translation. Save data never selects native code.

Wasm uses new ordinary v7 and persisted v8 tagged records. The bytecode v1–v6
encoding algorithms remain unchanged, with fixed independent golden controls.
This is a format claim, not cross-build compatibility: runtime/build fingerprints
still invalidate incompatible saved programs. The cooker identity advances for
state-aware validation; old captured-cook metadata must be rebuilt/reimported.
Guests without explicit state keep the previous state-ABI refusal.

Executed native controls cover tick zero, maximum epoch, empty/out-of-range
buffers, schema/length corruption, transaction and creation failure, delayed
restart consumption, reload compatibility, actual Rust SDK guests in Wasmtime
and wasm2c, and ordinary/opaque/persisted reconstruction with hosted UI and
audible fractional-clock PCM. The UI test uses deterministic text measurement;
it is not rendered appearance evidence. No shipping, device, performance or
full SDK completion claim follows from these source controls.


## Qualification and retained boundaries

[Portable records and manifest](evidence/native-wasm-persistent-state/manifest.json)
bind five attempts, including every failed native stage. The data-only
[verifier](evidence/native-wasm-persistent-state/verify.py) replays 250 retained
text/data records plus five process-inventory certificates; it does not execute archived scripts, Wasm or native code.
Full source inventories and native artifacts remain external and hash-bound.
Eight coherent alteration controls repair dependent hashes before requiring
refusal for missing tests, relaxed lint, guest opt-in, cleanup or accounting.

| Scope | Passing producer | Executed result |
| --- | --- | --- |
| Zero/default `incant_script` library configurations; ABI/SDK | `92b52974` | 2 strict lints; 1+1 tests |
| Runtime transactions/restart; existing bytecode ordinary/native/persisted saves; cooker | `9cfedb1d` | 14+34+4 tests; normal cooker built |
| Seven real Rust SDK guests, Wasmtime and wasm2c; tagged codec and bytecode goldens | `96eac776` | 6+3+1+3 host tests; 2 codec tests; 3 strict lints |
| Corrected bytecode test configuration; actual-host conformance lint | `ea666914` | 2 strict lints |

The two final changes after native behavior passed were equivalent cfg-specific
state destructuring inside a test and this documentation. Build fingerprints
conservatively include checkpoint test source; prior results retain their actual
producer identity instead of claiming execution from the documentation commit.

CPU002 stopped on zero-feature Clippy destructuring. CPU003 stopped on a local
fixture variable shadowing its helper. CPU004 then passed 52 tests before its
mixed UI/audio fixture correctly refused writes to UI transforms (five parity
tests passed, one failed). The corrected SDK sample targets gameplay entity 1;
the fixture retains separate UI and audible audio entities. Its late-consumer
control now requires the precise host refusal so an earlier runtime error cannot
satisfy it. CPU005 passed all remaining tests and three lints, then stopped on the
same cfg-only destructuring lint in a test. CPU006 passed the final two lints.
No runtime refusal, quota or acceptance rule was relaxed.

The complete matrix has 69 passing test executions; the earlier failed parity
binary also contains five passing executions, which are not additional coverage.
The six parity controls iterate both hosts and prove fresh guest globals with
continued explicit state; ordinary, opaque-native and persisted restoration;
exact world, UI focus/text and audible fractional-clock PCM continuation; candidate
creation/reload refusal; pending restart exclusion; and hidden output on the
precise late composed failure. UI measurement is deterministic test support,
not rendered-pixel evidence.

The old target was deliberately removed during disk cleanup. Qualification used
fresh storage with the original 5,400-second, 12-GiB target/1-GiB packet allocation,
two jobs and two Rayon threads. All attempts consumed 607.639 seconds cumulatively.
Final and sampled peak target allocation was 3,539,140,608 bytes; retained packets
and the copied cooker occupied 69,492,736 bytes. Each terminal wrapper and its
observed managed processes/groups were checked absent. The added inventory
certificates derive sampled descendants from the original terminal results and
managed identities from owner records, then record another fresh absence check;
they do not claim an independent historical descendant-sampling log. Limits are sampled
supervision of fixed trusted tools, not a hostile-code sandbox. No GPU or window
was used, and no shipping/device/performance/full-SDK or phase-gate claim follows.

Replay without native execution:

```sh
python3 -B docs/spikes/evidence/native-wasm-persistent-state/verify.py
python3 -B docs/spikes/evidence/native-wasm-persistent-state/test_verify.py
```
