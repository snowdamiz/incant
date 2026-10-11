# Native sprite flipbooks

Local implementation checkpoint; GPU execution, fresh shipping proof, final
integrated checks and publication remain pending. This is not a performance,
phone, editor appearance or phase-gate result.

`SpriteFlipbook` attaches 1–256 ordered pixel/atlas regions to one
`SpriteRenderer` texture. Its rate is an exact positive rational (each term at
most 1,000,000, ratio at most 1,000 frames/second). A scene has at most 262,144
animation frames. Cooking resolves stable IDs into version-8 binary records;
scenes without animation retain their earlier version-1–7 emission. The bounded
reader validates canonical entity order, flags, region tags, counts, rate and
component compatibility before a native world is constructed.

Playback is retained numeric ECS state, advanced by a cached Bevy system only
when native fixed-tick publication succeeds. It does not read project JSON,
look up asset names, clone the world or reconstruct query state each tick.
The clock uses quotient/remainder arithmetic, including rates above/below the
simulation frequency; it does not accumulate floating-point time or execute a
catch-up loop. Pausing preserves the subframe phase; seek resets phase and
completion while preserving play/pause; restart plays from frame zero. A once
animation holds its last frame for the full frame duration before completing;
playing a completed animation restarts it.

`ControlFlipbook` is a typed native structural command. A whole batch validates
against the final component graph before any component or playback publication.
Control-only batches avoid hierarchy staging. Controls run in submitted order
against that final declaration. An identical reinstall preserves playback;
changed animation configuration starts a new cursor, then applies queued controls.
These controls are not yet QuickJS/Wasm animation SDK operations. Native session
script/solver/host failures retain their existing terminal behavior; there is no
claim of composite rollback or animation/session save support. Spatial world
saves remain explicitly refused. Legacy gameplay refuses animation rather than
silently simulating a static sprite.

At visual binding, every frame resolves against the sealed retained texture,
including noncurrent frames. Missing/out-of-range frames fail before a consumer
receives candidate input. Animation replaces the unused static source region;
removing animation checks the static region again. Numeric frame updates and
consumer Err/unwind use the adapter's existing bounded undo storage. Retained
CPU tests count zero allocations for ordinary frame synchronization.

The renderer prepares all frame geometry at binding time, subject to the existing
128 MiB logical geometry and 256 MiB sprite texture limits, plus bounded frame
counts. Live/staging scenes share immutable attributes, batches and local
geometry. Ticks select the committed frame and update its transformed positions.
Static scenes keep their original single contiguous upload; mixed scenes merge
adjacent active ranges on the stack, so one animation does not create an upload
call per static entity. Painter order, nearest/linear sampling, nine-slice math,
WGSL, color transforms and pipeline construction are unchanged. Work still scans
the bounded prepared placements/batches; this is not a phone-performance result.

Current local checks:

- 144 scoped 2D/runtime/cook/native-render tests and doctests passed.
- 157 runtime/core/cook/render ordinary tests and doctests passed, with 64 GPU
  tests compiled but not executed in that command (includes two new tests).
- Four native-render flipbook tests passed after the unused-static-region fix.
- Generated TypeScript types check with the repository's installed TypeScript.
- All-target release Clippy passed across the nine affected crates.
- Schema generation uses the compiled Rust registry; the final integrated
  headless schema equivalence check is still pending.

The prepared `native-game.py --gpu --flipbook` extension compares animated first
and last actual GPU readbacks with independent static-frame control scenes at
identical script/input/physics/audio poses. It keeps the original source deletion,
compiler exclusion, profile/symbol identity, replay, refusal and audio checks.
Its execution is pending the coordinated GPU slot and integrated source union.
