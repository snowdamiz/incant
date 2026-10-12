# Native TypeScript animation access

QuickJS gameplay receives `animations` with aligned committed `ids`, `clips`,
`seconds`, `speeds`, and `flags`. Clip `-1` means the rest pose; other indices are
asset-local. Flags encode playing (1), looping (2), and completed (4). Native
scratch vectors are retained. The exposed numeric arrays are copied into the VM
allocator and count against its memory limit; retaining or mutating them cannot
change the world or a later snapshot. Capture precedes numeric loans and queued
controls remain invisible until the next update.

`commands.controlAnimation` queues select, seek, playing, looping, and speed
controls in the existing ordered structural transaction. Arguments are primitive,
exact-arity values: clip null or integer 0–255, finite nonnegative seconds, strict
booleans, and finite speed in [-16,16]. Native admission checks the actual asset's
clips and duration. Invalid active calls, even when caught, reject earlier
commands, numeric writes, logs, timers, and persistent state. Expired commands
throw without reactivating or poisoning an otherwise valid current update. The
shared command limit remains 65,536 per tick.

Both standalone and authored source-catalog compiler dispatchers carry the new
snapshot. New host/parser sources participate in the cooked QuickJS build
identity, so incompatible older cooked scripts refuse rather than silently
receiving a shifted argument list. The strict SDK includes the overloads and a
numeric gameplay template. A fresh trusted TypeScript/SWC cook is compared with
the committed guest fixture that the actual QuickJS integration tests execute.

Local Mac CPU verification passes 145 script tests (including 11 new integration
cases and one publication unit case), all-target/all-feature Clippy, compiler-free
static host compilation, strict SDK/template checks, and all 21 native TypeScript
cook tests. The actual guest uses two source groups and three distinct animation
instances. Other cases cover no-animation scenes, expired controls, invalid
primitives and asset-duration/final-role refusal, shared command quota, whole-batch
rollback, completion, committed despawn, 4,096-row VM retention exhaustion, and
NativeSession output hiding after terminal refusal. Evidence is recorded in
[native-animation-script.json](evidence/native-animation-script.json).

This is the TypeScript native gameplay slice. It adds no Wasm import or claimed
`incant_animation_v1` capability, and no editor controls, source loading in shipped
players, GPU acceptance, animation blending/state machines, or device performance
claim. Wasm ABI parity and render consumption remain separate work.
