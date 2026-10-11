# Native sprite flipbooks

The native implementation and local numerical/shipping checks are complete;
Claude appearance review is accepted; hosted checks and publication remain pending. This is not a performance,
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

- Integrated union `573af84` passed 666 workspace tests/doctests and all 71
  actual native GPU tests, including retained flipbook/static-frame equivalence
  and allocation-free staging. The two capture-helper bounds tests also pass.
- Full-workspace all-target Clippy passes with the scene audio feature under
  the functional `ci` profile. The 90 tools tests, editor typecheck, generated
  bridge/SDK checks and normal runtime dependency guards pass.
- The freshly built public headless executable regenerates byte-identical
  schemas. This replaces the earlier temporary registry-only check.

- 144 scoped 2D/runtime/cook/native-render tests and doctests passed.
- 157 runtime/core/cook/render ordinary tests and doctests passed, with 64 GPU
  tests compiled but not executed in that command (includes two new tests).
- Four native-render flipbook tests passed after the unused-static-region fix.
- Generated TypeScript types check with the repository's installed TypeScript.
- All-target release Clippy passed across the nine affected crates.

The prepared `native-game.py --gpu --flipbook` extension compares animated first
and last actual GPU readbacks with independent static-frame control scenes at
identical script/input/physics/audio poses. It keeps the original source deletion,
compiler exclusion, profile/symbol identity, replay, refusal and audio checks.
On source `95df19d`, the fresh macOS ARM shipping run passed in 219.96 seconds.
All 32 repeated fixture processes passed, including four animated first/final
GPU comparisons at 44,117 and 48,000 Hz; 35 refusal controls passed, including
an invalid noncurrent animation frame and strict v8 refusal without a visual
host. Existing logs, observations, CPU-only and plain-v1 controls also pass.
Authoring and fresh build storage were deleted before execution. Source compiler
symbols are absent, and the copied 8,943,600-byte executable matches dSYM UUID
`cf52a010-4b33-3265-b421-cdb3d949161b` and SHA256
`73e94eded698ee209128a5a10670ed86c7c1b3473544508f01dea8787c839f26`.

The matched union `07c0d433` shipping executable is 8,893,968 bytes. Target,
compiler, verified codegen and complete embedded-script metadata match; this
candidate adds 49,632 bytes. That is a local executable-size comparison, not a
performance gate. A wasm32 runtime check also passes, but only establishes
compilation, not browser playback. Exact source/check hashes and full proof are
in [native-flipbooks-local.json](evidence/native-flipbooks-local.json). The existing
CI probe invocation includes the additive controls in the same fresh build.

The `incant_render` `native_frames` example is a bounded review helper, using the
real native world and retained renderer with no project, scripts, audio or
presentation. Request files are bounded and parsed only at startup; sorted capture
ticks and typed controls select committed states. It caps 4096 entities, 256
flipbooks, 64 captures, 256 controls, 36000 ticks and 256 MiB total decoded output.
Its PNG output is not atomic: earlier files can remain after a later failure.
Claude packet 0042 completed its scoped appearance review on frozen source
`95df19d`, with Opus 5.5 and Max verified through ACP. All 19 guarded calls
completed (63 captures); the gate is removed and all processes reaped. Claude
accepted 15 full static-control moments, 850 fixed-entity crops and 2,640 playback
states, including source-free repeats. Root verified all 92 manifest files and
preserved the original Claude commit `ddc164b`.
[Appearance report](../../handoffs/0042-native-flipbook/result.md).

The combined prewarming/flipbook tree additionally passed all 72 GPU tests and a
fresh source-free shipping run with 32 repeated fixtures, four animated controls
and 35 refusals. [Combined evidence](evidence/native-render-followup-local.json). The supplied capture wrapper checks the gate and executable
hash per call, bounds the cumulative budget, and kills/reaps timed-out process
groups. Its absent-gate refusal was tested without invoking a renderer.
