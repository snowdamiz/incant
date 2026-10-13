# Persisted native game checkpoints

The default-off `native-checkpoints` composition adds a separately versioned
file path for supported native physics, explicit script persistence, retained
UI state and offline audio. Its native, outer-world and complete-game pieces
have passing CPU qualification; final integration review remains separate.
Ordinary checkpoint formats and their physics
refusals remain unchanged. This is not a Phase 1 exit-gate claim.

## File and ownership boundaries

- World v12 retains the existing bounded component scene and independently
  validated spatial/planar native owners. Its extra physics envelope is bounded
  to 32 MiB, including outer headers and planar binding cache.
- Script v4 uses the existing `INCSES01` envelope with a 136-byte header and
  requires world v12. The earlier script v1–v3 loader does not accept it. Code,
  action definitions and immutable catalogs come from the receiving package.
- Game v1 adds the 64-byte `INCNGM01` envelope, capturing the offline sample
  rate and suspension state. These fields must agree with the retained audio
  owner, including a silent mixer's absolute sample clock. Backend-driven and
  offline game factories retain their distinct clock requirements.

Checksums detect corruption; they do not authenticate history or grant host
capabilities. All stages construct unpublished candidates. Hosts bind to the
candidate before trusted guest initialization, and live session replacement
occurs only after those steps succeed. Module globals and closures initialize
again; only explicitly opted-in script data persists. Physical input starts
neutral, and the receiving host alone decides whether restart requests are
allowed. No pointer, device handle, executable payload or external command queue
is restored from the file.

`NativeSession::prepare_persisted_restore` supports host-controlled staging.
The headless `restore_persisted_bytes` helper replaces its receiver atomically;
hosted callers use the existing prepared UI/audio factories. Game files decode
through `PendingGameCheckpoint::from_persisted_bytes`, then the appropriate
offline or hosted game factory.

Physics payload, logical work and native construction use cumulative budgets.
Those budgets explicitly exclude separately bounded ECS/script/UI/audio
sections and general allocator overhead. They are not a whole-process memory
or elapsed-time promise. Unsupported native domains retain typed refusals; see
[native admission](physics-native-persisted.md). Device audio, arbitrary guest
heaps, shipped Wasm state and GPU-owned saves are not added by this work.

## Standalone player

The opt-in player uses its existing `--save-checkpoint` and `--load-checkpoint`
options. It retains trusted build-time bytecode; no source compiler or saved
executable is selected by a checkpoint. Saved sample rate must match the
requested PCM output rate. On resume, the input recording covers the requested
additional ticks from a fresh neutral input timeline.

A fresh physics world has no completed simulation boundary. Initial save
preflight therefore checks only the selected route. Actual native/script/host
save admission happens at the final tick before either new output is published;
it never advances a hidden tick to make a checkpoint pass. Save publication
remains create-only, with same-directory staging and failed-staging cleanup.
PCM and checkpoint publication are separate transactions, not a multi-file
power-loss guarantee.

## Qualification status

Source reviews accepted the script/game/player staging APIs. On source002,
the ordinary default-feature compatibility run passed all 238 cases in 23
immediately retained test executables, with no failures or ignored cases.
The first run's changed legacy diagnostic failed an existing assertion; the
original message was restored and that failed result remains in the evidence.
Source003 additionally strengthens the malformed-file output assertions and
corrects a feature comment; it is not relabeled as the earlier execution.

The first joined native-feature build compiled successfully. Its retained
executables passed all 260 pre-existing cases in 22 harnesses separately from
the new persistence controls. The new headless script control passed, while
four game controls refused the engine's default CCD configuration during save;
the new standalone-player cases did not run. These failures remain in
`native-focused-001`, and the compatibility results in `native-existing-001`.
They are not a successful qualification of the new file format.

Native correction was qualified without changing those default game fixtures.
The pinned engine activates collision sweeps for fast dynamic bodies even when
their explicit CCD flag is false. Admission must therefore constrain the actual
integration configuration and retain native caches, rather than infer a disabled
sweep from body flags. An independent moving-sphere control also exposed a
spatial-tree change flag incorrectly treated as an enablement flag. Final
qualification includes both corrections and the associated work bounds.

On corrected joined source `2352679e`, the focused native run passed all seven
new script/game/player cases. Complete deterministic and SIMD8 consumer runs
then each passed 267 cases in 23 immediately retained executables, with no
failures or ignored cases. Both runs retained their matching standalone player
binary before another build could replace it, and all frozen source inputs
remained unchanged. Four strict Clippy configurations passed: deterministic,
SIMD8, ordinary feature-off and headless script. Static-module and Wasmtime
development builds, format/convention/manifest/dependency checks and both actual
compiler-artifact numerical-mode audits passed. These are thirteen distinct
checks. The initial static-module check lacked its mandatory trusted bundle and
failed; the corrected invocation used a retained, hash-verified earlier wasm2c
fixture. This is compile compatibility with that fixture, not a fresh module
translation or guest execution. No production repair was needed for that setup
failure, and its original log is retained.

These native-feature controls cover warm 2D/3D continuation, exact
real/silent PCM continuation, immediate re-export after load, retained UI text
and focus, clock pairing, corrupted/rehashed files, budget failures and atomic
refusal. The standalone control compares 104 uninterrupted ticks against
separate 67-tick and 37-tick player processes using actual saved files, including
fractional PCM timing. These are actual separate processes using the matching
compiled player, not a simulated file round-trip. This remains CPU evidence;
no reference-phone, performance, shipped-Wasm persistence or phase gate follows.

The [portable manifest](evidence/persisted-game-checkpoints/manifest.json) retains
7,671 records and 3,161 unique payloads, including both committed native source
generations, raw results and earlier reviews. It binds 142 immediately retained
local executable copies; binaries are not embedded in the archive. The earlier
ordinary runs retain their 147-input hashes, logs and independent review;
uncommitted historical source bytes are not reconstructed. Counts across the
focused, historical and full suites overlap.

Verify the portable evidence without compiling or executing a game:

```sh
python3 tools/probes/persisted-game-checkpoints/collect.py --verify-only
```

On the producing machine, `--originals --local-binaries` additionally rehashes
the raw local evidence under
`~/.codex/incant-evidence/physics-persisted-session-001`. Final independent packet
review and current-main integration are required before publication.
