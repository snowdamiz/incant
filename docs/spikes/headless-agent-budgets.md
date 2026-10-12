# Durable headless agent usage

Routing: `gpt-6.1-sol`, Extra High (`xhigh`), Astra role. Base:
`22d359fb24697aa58cc790a40334d165c340fb16`. Reused the clean
`agent-budget-settings` checkout on `codex/headless-agent-budgets`; the separate
`codex/native-agent-budget-bridge` commit remains unchanged and available.

`incant agent PROJECT PROMPT --model MODEL` previously constructed a budget with
zero used tokens on every process invocation. The extracted command now uses the
same reviewed per-project `Ledger` and `Session::start_durable` as the native editor.
Local usage lives under the OS config directory `Incant/agent-usage/<projectULID>`;
no ledger path, account identity, or credentials enter project content or tool args.
Copies/reopens of the same project ULID share accounting across CLI/editor and
account changes. This patch adds no reset command or ledger-directory flag.

`--max-tokens` keeps its default of 64,000 and means **additional invocation
allowance**. With cumulative usage `U`, authored project cap `C`, and CLI limit `L`,
the effective absolute ceiling is `min(C, U.saturating_add(L), MAX_TOKEN_CAP)`.
The flag can tighten an invocation but cannot raise `settings.agent_budget.max_tokens`
or refund prior usage. Zero limit, zero project cap, and an exhausted allowance
refuse admission before constructing the account/provider/native host. A provider
can still report an actual overrun; those measured tokens remain charged and future
invocations refuse an exhausted project.

The CLI acquires the ledger OS lock using the validated genesis ULID before opening
and replaying the shared journal. Journal replay already enforces that project ID
cannot change. The cap comes from the recovered project, including user cap edits,
Undo, and Redo, rather than the genesis document. This ordering prevents a preceding
agent from finishing its journal writes after this invocation's recovery read.
As before, the headless authoring bus represents the recovered project for this
invocation; this patch adds no live synchronization with another authoring process.

`Session::start_durable` reserves the effective remaining allowance durably before
the accepted worker constructs or calls the provider. The worker owns the ledger
lock through inference, approval, and settlement. The native host is constructed
inside its owner thread so its potentially !Send state is not transported. Actual
project edits retain normal `incant_cmd` agent provenance and schema validation.
There is no new engine-agent shell capability or runtime authoring dependency.

Known completion/error/cancellation usage settles through the shared Session/Lease
receipt checks. Account setup failing before inference is a known zero-new-work
receipt and retains all earlier charges. Unmeasured provider failure remains a
conservative uncertain charge. Panic or inconsistent/missing accounting retains the
full invocation reservation. Existing process termination/SIGINT behavior is unchanged;
termination before settlement leaves the durable pending reservation conservative.
No additional live-provider or signal-process proof was run for this headless patch.

A work-result channel is not accepted as an accounting receipt. The command waits
for durable Session settlement and actual worker reap before saving the derived
`*.agent-result.json` or printing success. Failed output save therefore cannot make
provider work free. The input remains journal genesis, with no direct mutation path.
Raw provider/account errors do not become CLI failure text: fixed Session failure
kinds and fixed output errors are reported.

Successful output retains the original `report`, `output`, and `journal` shape and
per-turn measured usage/tool/transaction details. Additive fields are:

- `budget.max_tokens`: the effective absolute invocation ceiling, **not** a persistent
  cap edit; `budget.used_tokens` is cumulative project consumption.
- `project_max_tokens`: the cap recovered from authored project settings.
- `accounting_uncertain`: persisted uncertainty, including an earlier interrupted owner.

Only main wiring, the extracted command, focused tests, and this evidence change.
Shared Agent/Session/Ledger, schemas, editor bridge/UI, Eval, and native probes remain
unchanged. No real account/provider, GPU, window, PR, or publication was invoked.

Frozen validation used the idle private APFS-cloned `/tmp/incant-agent-budget-target`
with `--profile ci --locked -j2`; no shared target writes:

| Check | Result |
| --- | --- |
| `tools/cargo test -p incant_headless --bin incant_headless --profile ci --locked -j2` | 16 passed (13 new budget tests, 3 existing); no ignored/failed tests |
| `tools/cargo clippy -p incant_headless --all-targets --profile ci --locked -j2 -- -D warnings` | Passed; compile/check only for integration targets |
| `tools/cargo fmt --all --check` and `git diff --check` | Passed |

The tests use synthetic Providers plus temporary real ledgers, journals, and atomic
output files. They cover cumulative usage across fresh invocations, default/lower
CLI ceiling and safe saturation, zero/exhausted/overrun caps, journal recovery/Undo,
cap changes at ownership acquisition, actual provider errors/cancellation/account
boundaries, setup failure, panic and invalid receipts, live lock ownership and
reservation-before-provider ordering, actual output-save failure, and a real shared
agent transaction with the compatible report. Mock inference does not establish a
live-provider, device, UI, performance, or phase gate.

SHA-256 evidence:

| Path | SHA-256 |
| --- | --- |
| crates/incant_headless/src/main.rs | ef56460db6909fd839550e108a6ec79acaca5111321314c45f1f5e6575d6bc84 |
| crates/incant_headless/src/agent_command.rs | 8566b9d675405dbc9f931488b43f6e693d9d7fa3322e45d1d2fa16ca5de6a4a4 |
| crates/incant_headless/src/agent_command/tests.rs | c878e5e4be38148a30bcf609a225fb4c0db258f531991a08cb628989807b01ec |
| unchanged crates/incant_agent/src/session.rs | d7b1e0946fcbbc64ee47d7b5eac7e691d0a4e410a576a5ef7bbc5bd2e68f8b15 |
| unchanged crates/incant_agent/src/session/durable.rs | 886b6ab3573ff485868b9df110eb9d781fad9422002ab19a0588e3d058053984 |
| unchanged crates/incant_agent/src/ledger.rs | c54cde4db5d81f77f7b861fe1f7738012dbc8c7d6ea1cb1934bde2fa4742af6e |
| /tmp/incant-headless-agent-budget-tests.log | f954c6dd1376c02e3ad30c13c4a33a9fb92d5cf2ffc3b856153c80d9b3da3564 |
| /tmp/incant-headless-agent-budget-clippy.log | d4457d0c8a56f64d41c95ece8bb38bebc411d218c390740e07a8f447baff4469 |

## Independent root integration

Astra reviewed the frozen implementation and tests, then integrated the exact
source as `91615ee` into the native editor/agent/budget union. All six implementation
and shared accounting source hashes above match. The 16 binary tests and scoped
all-target Clippy passed again on the integrated sources. The review found no
actionable issue; CLI invocation limits, recovered authored caps, lock ordering,
worker ownership, settlement and failed-output accounting were checked.
[Source and validation receipt](evidence/headless-agent-budget-integration.json).
This verification uses synthetic provider responses and does not add a headless
live-provider or process-signal gate claim.
