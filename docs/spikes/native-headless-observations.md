# Native headless assertions

The source-free `incant_game_player` accepts an optional bounded observation plan.
It uses the same native input, embedded bytecode, physics, audio and optional
rendering session as an ordinary run. It neither loads a project nor parses
JavaScript or TypeScript at startup. Authoring still happens through `incant_cmd`
and the vetted cook/build path.

Append `--observe PLAN.json` after the existing positional arguments and optional
`--cpu-render-input` / `--gpu-offscreen` arguments. For example:

```sh
incant_game_player level.scene audio clips.bin recording.json actions.json \
  120 48000 output.pcm --observe observations.json
```

```json
{
  "format": "incant-native-observation",
  "version": 1,
  "checks": [
    {"name": "source exists at load", "tick": 0,
     "expect": {"type": "exists", "entity": "00000000000000000000000001", "value": true}},
    {"name": "right press", "tick": 1,
     "expect": {"type": "action", "name": "right", "pressed": true}},
    {"name": "final position", "tick": 120,
     "expect": {"type": "vector", "entity": "00000000000000000000000001",
                "property": "world_translation", "value": [2.95, 0, 0], "tolerance": 0.001}}
  ],
  "entities": ["00000000000000000000000001"]
}
```

Tick zero is the loaded native world. Later checks run only after the entire
accepted tick has completed, including the script, audio and requested render
consumer. Checks are ordered by tick then name. They read only requested typed
values; ordinary unobserved ticks perform no world snapshot or JSON conversion.
The bounded results serialize once after the completed run.

Supported expectations are `entity_count`, `exists`, `vector` and `action`.
Vector properties are local `translation`, `world_translation`, `velocity`,
`physics_translation` and `physics_velocity`. Vector comparisons require an
explicit finite nonnegative absolute tolerance. Action checks may compare any
combination of `active`, `pressed`, `released` and two-dimensional `value`;
`value` and its tolerance must appear together. Missing entities, components or
actions fail a value assertion and report `actual: null`. An explicit
`exists: false` expectation can pass for an absent entity.

The final `observations` object contains `passed`, tick/rate, each named check's
expectation/actual/result and the requested entities' typed transforms/velocity.
Selected entity IDs are canonical uppercase ULIDs. A missing selected entity is
reported as null; selection alone is not an existence assertion. No complete
world export or VM-private state is exposed.

The startup plan is limited to 256 KiB, 256 checks and 256 selected entities.
Check names must be unique, nonempty, at most 120 UTF-8 bytes and contain no
control characters. Check ticks must be within the requested run. Unknown fields,
formats, versions, properties, duplicate IDs and malformed/noncanonical ULIDs
fail before the simulation executes. At least a check or an entity is required.
Existing run bounds (3,600 ticks, one million output frames) remain in force.

A completed run with false assertions writes its normal PCM and structured
report, then exits nonzero. All requested checks still run so the report shows
all mismatches. Invalid startup input or a terminal simulation/host error emits
no completed report or PCM, even if an earlier check passed. An existing output
file is never overwritten. Output I/O can still leave a partial file on an I/O
error; the existing writer does not promise atomic filesystem publication.

This is a native assertion path, not completion of the whole headless runner or
Core Sample gate. The legacy project-oriented `incant_headless play` path remains
to be migrated. Complete script logs, composite session saves, editor/agent
integration, the full game fixture and reference-device budgets remain open.
No mobile performance or whole-frame allocation claim follows from this work.

## Verification

On aarch64 macOS, seven focused tests and **577 ordinary workspace tests/doctests**
passed; the workspace's 66 unrelated GPU tests remained ignored in that command.
All-target release Clippy passed for the workspace and the optional GPU player,
as did 49 Python tool tests, formatting, generated conventions and six resolved
normal-dependency guards with the GPU player feature enabled.

The existing fresh shipping probe retained all 40 prior process controls and
added 14 observation cases: two repeatable passing runs, two completed false
assertions, nine startup refusals and one late terminal failure. All **54 process
runs** passed after the authoring inputs and fresh build storage were deleted.
The passing observed runs exactly matched the original report's motion, audio
hash and numerical GPU readbacks. The false cases preserved the same PCM;
missing physics returned null. A late script failure suppressed completed output.
No extra shipping build or CI job was introduced.

The matched player is **7,570,160 bytes**, SHA-256
`eb017892754f1d26f50f6211ead226b3fc349b268d442e270f7e0c73196a4b85`,
with arm64 dSYM UUID `09e12850-611a-3dcd-a893-dd8e1fa4e2b0`.
These whole executable numbers include prior query and timer work; they do not
isolate observation overhead or establish a device performance budget.
Source, command, log and executable identities are recorded in
[evidence/native-headless-observations.json](evidence/native-headless-observations.json).
Hosted execution of this revision remains pending.
