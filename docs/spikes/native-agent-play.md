# Native agent play tests

The headless agent now exposes `play_run` and `play_state_query` through the
concrete native host. A request cooks the selected authored scene once and runs
its disposable native session with the trusted build-embedded behavior. Ordinary
headless builds use the explicit no-op behavior; the developer TypeScript cooker
can embed custom behavior before the agent runs. Agents cannot select source,
executables, compiler flags, filesystem outputs or network destinations.

`play_run` takes seconds, an optional scene/camera and expected script hash, a
data-only input recording, and typed observation checks. It rejects unknown
fields, requests above 1 MiB, durations above 600 seconds or 36,000 ticks, and
invalid recordings before startup. It reuses an audio buffer and typed runtime
state through fixed ticks, without project cloning or per-tick JSON. The
10-second wall-time budget is cooperative: it checks between bounded operations
and before publication, but cannot preempt a running startup operation or tick.

Only a completed run replaces the retained session. Input rejection, runtime
failure, interruption or budget expiry leaves the earlier completed session
intact. A false assertion is a completed, inspectable result with `passed: false`.
Queries use a bounded allowlist and at most one canonical entity, carrying the
retained tick and original authoring revision. Full-world enumeration is refused.
These operations never create project commands or history entries. The existing
agent approval policy still applies; unsupported hosts do not advertise them.
The same implementation serves `play.run` and `play.state_query` over RPC.

CPU-only agent operation no longer initializes a GPU. A screenshot request lazily
creates the same renderer as before. These play methods explicitly report that
native gameplay images are unavailable; this is not the plan's finished capture
API. Editor controls, project-attached scripts, complete SDK/session saves, and
legacy CLI migration remain separate work.

## Verification

On base `a73df4a44230703b2201cf0dabfca0e089ae82fc`, 87 scoped release-profile
Rust tests and all-target Clippy passed; two unrelated GPU tests were ignored.
The 92 Python and 12 Node tooling tests, generated schemas/SDK/bridge/conventions,
formatting, and actionlint passed. Independent Astra source review covered
candidate publication, bounds, cancellation, authoring isolation and lazy GPU
startup. A dependency-closure test caught three missing credential workflow
trigger paths; all were added without relaxing coverage. An earlier stale
constructor call failed compilation and was corrected before the passing run.

The actual source-deleted TypeScript developer player passed twelve processes:
six CLI, four retained RPC, and two additional agent-capability RPC controls.
They preserve custom motion, Unicode logs and authoring files, check failed
candidate recovery and inspectable false assertions, and repeat deterministically.
This uses a warm private developer target and is not a fresh shipping proof.

A separate real `gpt-6-astra` provider turn used the director's existing ChatGPT
OAuth login in the rebuilt binary. Its recorded three-step turn successfully
called `play_run` and then `play_state_query`, created no transactions, and left
the fixture unchanged. No sign-in prompt, API key, GPU or screenshot was needed.
The live turn report proves native tool invocation; numeric correctness comes
from the executable native controls, not the model's prose. This is one scoped
live test, not the full agent evaluation suite or a phase gate.

[Source identities, logs, process results and live turn report](evidence/native-agent-play-local.json).
Hosted checks and main integration remain pending the shared merge queue.
