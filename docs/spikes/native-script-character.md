# Native script character movement and triggers

The native QuickJS physics capability now exposes `computeCharacterMotion` and
`triggerEvents` beside raycasts. Character query/result and trigger types derive
from the same generated schemas as the established physics boundary. No project
representation, command-bus edit or JSON conversion enters the update.

Character sweeps compute world-space displacement against committed solver poses.
They do not move a body. The [native template](../../sdk/templates/native-physics/character.ts)
converts the displacement into a structural velocity command for the next fixed
simulation step. Existing nonsensor primitive, kinematic/root/unit-scale and zero
angular velocity requirements remain enforced. Pending numeric writes and commands
are invisible to queries until their normal commit/step boundaries.

Trigger reads return owned, stable-ID-sorted events from the last committed solver
step, without draining them. Mutating a returned array/object cannot change the
solver; moving a body produces new events only when the solver steps. All three
functions expire after the update and old functions never reactivate.

The shared budget is 1,024 physics calls per update, including a maximum of 128
character sweeps. Calls reserve their slots before guest getters execute. Argument,
query, reentrant budget and result-allocation failures latch even when caught in
JavaScript, rejecting every numeric write and structural command in that update.
Argument/result conversion is isolated in `query_values.rs`; returned own properties
bypass guest prototype setters. The original solver ownership guard is unchanged.

Validation: **131 scoped tests/doctests**, **561 ordinary workspace release tests
and doctests**, workspace and GPU-player all-target Clippy, strict SDK typecheck,
six-root normal dependency guard, and 43 Python tool tests. Independent source
review found no blocking issue. The 66 unchanged renderer GPU tests were not rerun.

The existing fresh shipping probe passes **45 processes**: all previous 34 cases,
eight repeated character runs at 0, 1, 60 and 120 ticks, and three new caught-error
refusals. The character stops against a wall/floor and observes exactly one sensor
entry and exit. Sources, project files and fresh Cargo storage are deleted before
execution. The same precompiled executable handles every case; no extra cold
shipping build or new CI job is introduced.

Actual aarch64 macOS player: **4,107,256 bytes**, SHA-256
`6db3e79f8976de0837905dcb54308a2e66687cf9ed852f5b43531c074c9d8c46`,
arm64 dSYM UUID `f6bfb774-080c-3177-9d2b-2017138fc957`.
Raw package/logs: `/tmp/incant-native-character-shipping/`.
[Source identities, independent review and observations](evidence/native-script-character-2026-10-11.json).

Native query work cannot be interrupted mid-call; the CPU budget is checked before
and after it. The configuration-dirty startup path can clone collider data for a
query-only BVH; the ordinary stepped path borrows the existing BVH. Results and
capability slots allocate. These checks make no phone, frame-budget or allocation-free
claim; remaining SDK and gameplay migration continue separately.
