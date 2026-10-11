# Project agent token allowance settings

Implementation routing: `gpt-6.1-sol`, Extra High (`xhigh`), Astra nonvisual role.
Base: main `04f8d9b18a4b29893c63a346841fffd41ce437c7` (PR97).
No dependency resolutions, runtime implementation, agent implementation, editor
UI, visual artifacts or provider integration changed.

## Authoring contract

`ProjectSettings.agent_budget` is a typed `AgentBudgetSettings { max_tokens: u64 }`.
The default is 1,000,000; zero means no new agent turns; the maximum is
9,007,199,254,740,991, the largest exact JavaScript integer. `incant_doc` publishes
`DEFAULT_AGENT_TOKEN_CAP` and `MAX_AGENT_TOKEN_CAP` without an agent dependency.
An absent setting defaults. Explicit settings require `max_tokens`, reject extra
fields, and pass the project's semantic validation before publication. Negative,
fractional, nonnumeric and excessive values fail. No consumption ledger belongs
in the project.

The shared command wire shape is:

```json
{"op":"set_agent_budget","settings":{"max_tokens":1000000}}
```

Only `Origin::User` can issue this command, even for a no-op or a mixed transaction
that would restore the starting value. Agent, script and import origins receive
the typed `CommandError::AgentBudgetUserOnly`. Refusal occurs before source-edit
preparation or live document/journal publication. Whole CRDT projection merges and
journal Apply replay additionally check cap differences and explicit budget
commands, preventing a stripped command list from bypassing this boundary.
Origins remain metadata supplied by trusted host adapters; this does not add
cryptographic authentication to project files or journals.

User transactions retain normal provenance, revision conflicts, persistent
checkpoints, Undo/Redo and journal replay. Unrelated nonuser changes remain valid.
Historical user Undo/Redo is preserved. Default settings are omitted on write,
preserving legacy canonical project bytes and journal/checkpoint compatibility.
Deleting an explicit cap to restore the default still counts as a cap change.

The existing exporter regenerated Project, Command, agent-tools and legacy
GameSave contracts, followed by SDK declarations. GameSave embeds Project and
therefore gains the optional authoring setting; its default serialization is
unchanged. Native scene/runtime formats and shipped dependency edges are unchanged.
Ledger persistence, consumption enforcement, account/project lifecycle wiring and
the settings UI remain separate integration work; this slice does not claim a
usable budget control or a provider/device/phase gate.

## Local verification

All Rust inputs remained fixed while each build ran. A private APFS clone of the
idle native-fbx-import CI cache was used as `/tmp/incant-agent-budget-target`;
the shared source target received no writes and was released after cloning.
Local host: aarch64-apple-darwin, Rust/Cargo 1.99.0, Node 22.23.3, TypeScript 5.9.3.

| Check | Result |
| --- | --- |
| `CARGO_TARGET_DIR=/tmp/incant-agent-budget-target tools/cargo test -p incant_doc -p incant_cmd --profile ci --locked -j 2` | 64 tests passed, including 13 new budget tests |
| `CARGO_TARGET_DIR=/tmp/incant-agent-budget-target tools/cargo clippy -p incant_doc -p incant_cmd --all-targets --profile ci --locked -j 2 -- -D warnings` | Passed |
| `tools/cargo fmt --all --check` | Passed |
| `CARGO_TARGET_DIR=/tmp/incant-agent-budget-target tools/cargo run -p incant_headless --profile ci --locked -j 2 -- schema schemas` | Published exact Rust-derived contracts |
| `node tools/generate_sdk.mjs` and `node tools/generate_sdk.mjs --check` | Generated; equality passed |
| `tsc --noEmit -p sdk/ts/tsconfig.json` with the existing pinned compiler | Strict check passed |
| `python3 tools/check_runtime_dependencies.py incant_game_runtime incant_game_player incant_runtime` | Resolved normal dependency graphs remain authoring-free |
| `git diff --check` | Passed |

The new tests exercise legacy byte equality, defaults, zero and maximum bounds,
malformed settings, schema limits, command shape, atomic refusal including no-ops
and mixed transactions, all nonuser origins, unrelated nonuser edits, projection
imports/deletion, real persistent reopen/Undo/Redo and journal replay. Replay
regressions re-hash actual journal entries after changing actor authority and
stripping commands, so authorization is tested independently of integrity checks.

An initial schema test exposed the missing default annotation when default values
are omitted by Serde; the final field has an explicit schema default and the test
passes. No tests were skipped to resolve that failure.

The actual `cook_scene` example was rebuilt with `--profile ci --locked -j 2`.
A moving Transform/Velocity scene with fixed ULIDs was cooked from three project
files: omitted legacy default, cap zero and the largest exact cap. All three
payloads were byte-identical, 200 bytes, SHA-256
`9acecb8058c4c708d0f82b8ece058c0eab9e5166b27b0d6067e6969458cbecb1`.
The cooker binary SHA-256 was
`d03be2fb516a9d96d1d59ab411cf67f8114b4986e91c7e1d4ecba1b440abd014`.
This is local payload compatibility evidence, with no window/GPU launch or
performance/device claim.

## Frozen input and result hashes

Paths below are repository-relative except the raw local logs.

| Input | SHA-256 |
| --- | --- |
| `Cargo.lock` | `75be65ed8212740e56c44a9a2c3e3d0875ceb0f05979699bced84bb142b183e5` |
| `crates/incant_doc/src/agent_budget.rs` | `d4400624c278dcd2d1d3d31a94c2113dd6bbfa9f6d4b944780dd0a1f82793536` |
| `crates/incant_doc/src/lib.rs` | `35ea95149887cf7e23f6c206fbc3cf223d8bbd59e21f988e2d5a59562b52d2c9` |
| `crates/incant_doc/tests/agent_budget.rs` | `b570ffdda8a5619ca0698d5dfbc38a3ae399c7f249d3cbbbf5c8f605caa6cc91` |
| `crates/incant_cmd/src/lib.rs` | `1ae889f39d9c2abc566d77e0fe118d875cc23e8ab4d48758bb97cbce92ac3be2` |
| `crates/incant_cmd/src/journal.rs` | `1bcf5cb00273c635ee55d8357baee264cda334b728b12df2cc15f40f41048458` |
| `crates/incant_cmd/src/agent_budget_replay_tests.rs` | `3cd27afc6691ada417c9997b6acd4cf5909eb6f4a4f581b93e3f3bdc84078a82` |
| `crates/incant_cmd/tests/agent_budget.rs` | `2b8e86e35fe77ef28078ebde34e46581dddd062b05da5be17f00a65d80cb4b1e` |
| `/tmp/incant-agent-budget-tests.log` | `0aea39e1c75e277ef736752d6748145621249cb7f0a623ab09fb9ea5e09e001b` |
| `/tmp/incant-agent-budget-clippy.log` | `65fce95dff930c0cd5209cc2e4eabad3c1ce06edc2022bc6c18c8661c257b3cf` |
| `/tmp/incant-agent-budget-schemas.log` | `8f8a11fdfa53527ef57def7c6fa8900e4f2ce62b3e9598b70b45c689ad2d8878` |
| `/tmp/incant-agent-budget-cook-build.log` | `0ca8c35cd9ffaa1f45e68e6c467653462f900c0200778398aeb9d52c6215253a` |
| `/tmp/incant-agent-budget-payload-parity.log` | `1860780719b499ee6df9609d5a11eba6875c8b455bf8f036b5de0eaba4dc78ef` |
