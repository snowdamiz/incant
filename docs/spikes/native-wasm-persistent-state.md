# Explicit native Wasm persistent state

The opt-in route stores bounded guest-selected bytes in the native capability
owner. It does not capture Wasm memory, globals, tables, pointers or closures.
Implementation and portable controls are present; native qualification and an
ordinary authored shipping export with save/load are pending.

A validated `incant_state_v1` custom section declares version 1, a nonzero
32-byte schema identity and 1–65,536 bytes. Additive ABI operations 64/65 read
and write the region using the current tick epoch. Earlier operations 0–63
and ABI version 1 retain their signatures. Missing, duplicate or malformed
declarations refuse admission before startup.

Each opted-in host owns committed and staging buffers. The first write copies
committed state; successful native transaction completion swaps the buffers.
Aborted updates discard staging. Reads see earlier writes. Current-epoch
errors are sticky even if a guest ignores them; stale epochs cannot poison the
current callback. The region has its own 256-call/1-MiB transfer limits and
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

Prepared native controls cover tick zero, maximum epoch, empty/out-of-range
buffers, schema/length corruption, transaction and creation failure, delayed
restart consumption, reload compatibility, actual Rust SDK guests in Wasmtime
and wasm2c, and ordinary/opaque/persisted reconstruction with hosted UI and
audible fractional-clock PCM. The UI test uses deterministic text measurement;
it is not rendered appearance evidence. No shipping, device, performance or
full SDK completion claim follows from these source controls.
