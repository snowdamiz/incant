# Agent static colliders from alpha

The engine-agent `collider_from_alpha` tool delegates to the same
`ImportSnapshot::prepare_alpha_colliders` and `PreparedAlphaColliders::commit`
service as the host CLI. It adds no geometry implementation or mutation path.
Its source is based on alpha authoring commit `94fc63b9`. At adapter source
`caff2acd`, all 23 selected native CPU tests and all-target strict Clippy pass.
These controls use mock providers, not a live account or hardware interaction.

```json
{
  "selection": {
    "scene_id": "SCENE_ULID",
    "entity_id": "SPRITE_ULID",
    "threshold": 128
  },
  "expected_revision": 2
}
```

Both argument objects reject unknown fields. The observed revision is required;
threshold is an inclusive alpha byte in 1..255. The tool consumes the selected
sprite's checked cooked texture, with the shared service's static-simple-sprite,
texture, transform, region and exact-cover bounds/refusals. It never imports the
original source or falls back to other geometry when those checks fail. See the
[shared geometry contract](static-alpha-colliders.md).

Only an engine-granted `ProjectFiles` capability exposes the tool. The capability
binds a project ID and canonical root; arguments cannot supply source/cache paths,
another root, an actor, commands or executable code. Hosts without that capability
omit the tool from their catalog and refuse direct calls. Default cache path
confinement and symlink refusal stay in the shared importer. As with the existing
asset capability, this does not promise protection from a separate malicious
local process racing filesystem checks.

The tool follows `asset_import` write approval: Auto proceeds, Destructive and
Always request approval. Other tools and approval predicates are unchanged.
Cancellation is checked before capturing/preparing and after preparation, again
before publication. Preparation does not hold the shared editor lock. A stop or
project switch while reacquiring that lock refuses publication. Commit also
checks the complete captured document and revision, so a same-ID, same-revision
replacement cannot pass merely because its revision matches.

One successful call appends one ordinary `incant_cmd` transaction. The engine
supplies the actual provider model and conversation as actor; the shared service
supplies the source/texture/threshold description. The response includes its
current revision and transaction ID, with all derivation fields under
`untrusted_project_data`. Stable generated entity IDs, command validation,
provenance, journal recovery and Undo/Redo come from that same transaction.
These are fixed root-level sibling boxes: a later sprite move/reimport/delete
does not update them, and another call creates a new set. There is no dynamic
body, live linkage, GUI selection or runtime performance claim.

Nine focused tests use deterministic mock providers and a real imported 2x2 PNG:

- Source-deleted cooked derivation checks exact texel rectangles, box extents,
  transforms, preserved sprite/assets, actor/provenance, durable Undo/Redo.
- Strict schema, stale revision, absent/wrong host capability, unsupported sprite,
  missing cache and redirected cache refuse without publishing.
- All three approval modes, denial, cancellation during approval, current tool
  advertising and a concurrent approval edit use the normal agent loop.
- A deterministic shared project slot changes authority at final publication:
  cancellation, concurrent edit, a different project, and a replacement document
  with the same ID/revision all preserve the appropriate current world/history.

The CPU qualification runs `cargo test -p incant_agent --test alpha_colliders
--test assets --test shared` followed by all-target strict Clippy on
`incant_agent`, using Rust 1.99 and the locked/offline CI profile with two workers.
Nine new alpha tests, three asset tests and eleven shared-access tests pass;
independent review checks every raw test name and all copied executable hashes.
The actual process exits zero and all 74 observed wrapper/child identities are
absent after cleanup. [Recorded commands and evidence](evidence/agent-alpha-colliders-native.json).

Review found that the checked-in `agent-tools.json` still needed the new tool.
The corrected file comes from a fresh headless build at `caff2acd`: all 80
schemas were exported, 79 remained byte-identical, and the only addition is
`collider_from_alpha`, preserving every prior tool schema and its order. The
integrated registry passes the ordinary isolated `tools/check_schemas.py`
comparison against that executable. [Schema evidence](evidence/alpha-colliders-schema.json).
No live model, engine window, GPU, screenshots or capture helper was used.
