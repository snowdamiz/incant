# Retained native project Play over RPC

The headless `rpc` command's `play.start`, `play.step`, `play.state` and
`play.stop` methods now use the same cooked native session as `native-play`.
They no longer construct the legacy document-backed `incant_core::Engine`.
This is the RPC simulation boundary; it does not yet add editor controls,
project-attached Script routing, window presentation, runtime
inspector edits or session saves. The separate old `play`, `run` and `script`
CLI commands remain legacy migration work.

## Lifetime and data boundaries

Start validates/cooks the current command-bus project once, chooses exactly one
scene, loads its cooked audio/visual resources and constructs a disposable native
world. Only the selected scene ID, original revision, trusted compiled behavior,
native game session and a reusable offline audio buffer remain in the controller.
Spatial content requires an explicit camera; its retained CPU scene host is
reported honestly with `gpu_submitted: false`. Audio is mixed on the native
accepted-tick clock into the retained buffer; RPC does not open an audio device
or send PCM as JSON.

Step accepts one bounded input packet and advances one native fixed tick. It has
no project, command-bus or authoring-validator parameter. State queries inspect
only the explicitly requested IDs (at most 256 distinct canonical ULIDs).
No full-world or project snapshot is generated on each step. Input and committed
logs are optional bounded observations. Response JSON is serialized at the RPC
boundary after the update; there is no authoring JSON conversion inside the
native simulation.

Malformed requests, invalid observation selections and rejected input packets
leave the live tick unchanged. Any failure after input acceptance follows the
native game's terminal failure rule: world, input and logs become unavailable.
Stopping discards that session. Starting while a live/failed session exists is
refused until an explicit stop; a failed start never replaces or installs a
session. Stopping an absent session is harmless.

Explicit project commands during play still edit the authoring bus normally.
They do not modify the running world. Stop returns the current authored project
and revision, and a new start cooks those edits. Gameplay does not add authoring
transactions or modify project files.

## Protocol

Each request is one JSON line with an `id`, `method` and optional `params` object.
Play parameter objects reject unknown fields. For example:

```json
{"id":1,"method":"play.start","params":{"scene":"00000000000000000000000001","sample_rate":48000}}
{"id":2,"method":"play.step","params":{"events":[{"type":"key","code":"KeyD","down":true}]}}
{"id":3,"method":"play.state","params":{"entities":["00000000000000000000000002"],"include_input":true,"include_logs":true}}
{"id":4,"method":"play.stop"}
```

Start accepts optional `scene`, `camera`, `script_sha256` and `sample_rate`
(default 48,000, permitted range 8,000–192,000). Omit `scene` only when the project
has exactly one. The SHA-256, when supplied, must match the behavior embedded by
the trusted build; it does not select a source file or executable.

Step accepts `events` (default empty) and optional `observe`, which has the same
fields as state. State defaults to summary only: runtime identity, scene,
original authoring revision, current tick/rate, entity count and rendered audio
frame count. Selected missing or despawned entities have null values. Other
selected entities include local transform, velocity, parent and world matrix.
Input/log fields are omitted unless requested. Stop accepts only an empty object.

`python3 tools/native_play.py project.json --typescript entry.ts --rpc` checks
and cooks the bounded TypeScript graph, builds the trusted engine package,
copies the player privately and removes temporary compiler inputs before
serving RPC. The coordinator is a developer tool, never an engine-agent shell
capability. Without a custom behavior build, the ordinary headless binary uses
its explicit no-op native behavior. Unmigrated project Script components still
fail cooking rather than silently running through the legacy path.

## Verification

The implementation and its clean integration with verified main80
(`611d867bee1ff3c8f33da8c221c82867408e96c6`) pass these executable checks.
Integrated source was frozen at `b7514055e9913af62065f4789002870cdc07efba`;
hosted checks remain pending publication within the merge queue:

- Eight native CLI regression tests and five retained RPC tests cover motion,
  input, selected state, authoring isolation, rejection/retry, terminal failure
  and restart. The actual physics/persistence probe also passes after migrating
  its observation call; all prior legacy comparison controls remain.
- `native-project-play.py --typescript --rpc`: existing six real CLI controls
  plus four actual RPC processes on the same private source-deleted player,
  including custom motion, committed Unicode logs, failure and restart.
- All-target headless release Clippy and conventions pass on the integrated
  source. The unchanged tooling passed 90 Python/12 Node tests before integration. Independent source review found the old physics probe consumer; that
  finding is resolved by the passing migrated real-process probe.

[Exact source/log hashes and process results](evidence/native-play-rpc-local.json).

No shipping-profile, hosted-platform, mobile performance, editor interaction or
phase completion is inferred from these developer-process checks.

A later [native agent integration](native-agent-play.md) adds bounded `play.run` and
`play.state_query` through the same controller, with separate live-provider and
source-deleted execution evidence.
