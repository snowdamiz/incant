# Phase 0 evidence ledger

Date: 2026-10-08. Host: Apple M5 Pro, arm64 macOS. Rust 1.99.0, Node 22.23.3.
This is a working evidence ledger, not a gate approval. No later phase has started.
The complete 36-month engine/game plan remains open.

## Spike 1: native viewport — in progress

Tauri 2.12.1 hosts a transparent child webview above a wgpu 29 native parent-window
surface. The renderer compiles and actual offscreen GPU readback produced
`artifacts/phase0-capture.png` (1280×720, Apple M5 Pro). This only proves native GPU
render/readback, not webview composition. Native hierarchy selection, rename, undo and redo have passed; Claude
pixel review remains pending. Windows execution remains pending.

## Spike 2: document and CRDT — local evidence passed

`cargo run -p incant_cmd --release --example collaboration_spike` imports a live
Bevy snapshot after 120 ticks through the command bus, forks two clients from the
same CRDT ancestry, edits name and velocity concurrently, exchanges snapshots,
asserts convergence, saves canonical JSON and reloads byte-identical text. Final
translation x is 5.999999999999987 (the expected floating-point representation of
six). The generated merged/reloaded files have no diff.

The command tests include 10,000 randomized short undo/redo sequences, atomic
rollback on an invalid component, stale-revision rejection, redo invalidation,
exclusive journal locking, interrupted-tail recovery, corruption rejection,
checkpoint reopening and concurrent peer merging after a restart. Invalid remote
state is validated on a fork before it can affect the live document.

### CRDT microbenchmark

Command: `cargo run -p incant_doc --release --example crdt_bench`.
10,000 entities, five scalar fields each, two concurrent field edits, full snapshot
exchange. One local measured run; no network, schema validation or UI cost.

| Measurement | Loro 1.16.2 | Automerge 0.12.0 |
|---|---:|---:|
| Initial document | 82.026 ms | 722.965 ms |
| Fork/edit/export/merge workload | 119.654 ms | 82.525 ms |
| Snapshot bytes | 1,028,311 | 115,297 |

Both implementations converged. Loro is a provisional authoring choice because
its initialization and nested-map integration suit the current implementation;
Automerge won this snapshot-size and merge-workload comparison. Do not interpret
this as a comprehensive production benchmark. Full-state journals and snapshots
need a storage/compaction budget before larger workloads.

## Spike 3: QuickJS, TypeScript and Bevy — local evidence passed with scope limits

SWC compiles `sdk/templates/kinematic/move.ts`. The sandbox queries a serialized
projection of live Bevy ECS state and submits typed commands to a disposable
simulation bus. Authoring state and CRDT history remain untouched during play.
Hot reload preserves matching state fields and keeps the old behavior on a failed
reload. Rust tests verify a script sees successive Bevy-integrated positions.

Command: `incant_headless script artifacts/thousand.incant.json artifacts/move.js
--ticks 120` in a release build. The fixture has 1,000 entities and a batched
TypeScript behavior that updates each of them every frame: 120,000 commands total.
Measured p95 9.539 ms; maximum 10.197 ms; frame budget 16.667 ms. Includes QuickJS,
command validation and commit, ECS snapshot and projection synchronization. Excludes
rendering, hardware presentation and sleep. This is not 1,000 isolated VMs and does
not establish performance on other devices or with complex gameplay. A later repeat
while browser evidence and other desktop work were running measured p95 15.955 ms
and maximum 16.781 ms (one frame exceeded 16.667 ms). Host load average was 14.88.
The result is sensitive to concurrent load; production frame-time guarantees remain
unproven. Both results are retained in the evidence directory.

An earlier implementation appended runtime ticks to authoring CRDT history and
measured p95 364.74 ms, failing the budget. Separating disposable runtime state
removed that inappropriate authoring overhead while retaining shared commands.

## Spike 4: live OpenAI agent — evaluation in progress

The local loop has query, typed atomic patch, schema and actual GPU screenshot
tools, streamed Responses handling, bounded tokens/steps, approval modes,
interruption checks and provenance. Mock tests cover continuation, failures,
denial, cancellation and budgets. No mock score is a live result.

`evals/phase0/tasks.json` contains twenty independent cases, including a ten-step
edit, duplicate labels, hierarchy changes and two injection examples. The oracle
compares the full final document excluding generated provenance, checks successful
query/patch/screenshot calls, validates agent provenance and undoes every patch to
recover the exact initial project. A partial run cannot pass the gate. The corpus
validates locally. Real OAuth inference and an approved scene edit now pass. The
unassisted twenty-task evaluation is in progress; no gate score is claimed yet.

The CLI supports a dedicated CI evaluation key or an explicitly connected local
OpenAI account. It never reuses Codex or Claude credentials.

## Spike 5: authentication — partial evidence

Implemented direct PKCE/loopback OAuth, random state and nonce, issued client ID,
ID-token signature/audience/issuer/expiry/nonce/subject checks, required scope
checks, serialized refresh, local profile metadata, API-key fallback and local
revocation. Callback tests reject incorrect state and duplicate parameters.

`incant_headless auth keychain-check` on macOS stored, read and deleted a unique
temporary probe credential in Incant's OS keychain namespace. No real provider
secret was needed or printed. This proves the local keychain backend only.

The initial request used an invalid host identifier. After correction, the director
completed real browser sign-in. macOS session refresh, native restart after a
changed build, and a saved-account scene-edit request pass. At the director's
request, macOS now uses atomic owner-only credential files to avoid repeated
Keychain prompts. Windows/Linux retain their OS stores. The
[repair and live evidence](auth-login-repair.md) documents storage, verification and
limits. Windows/Linux real-account authentication, API-key inference and live
revocation remain pending.

## Spike 6: Claude ACP handoff — in progress

Authenticated Claude Code reports a valid subscription. ACP initializes, lists
models, explicitly selects Opus 5.5 and executes the hierarchy/editor packet in an
isolated worktree. Credentials are not read or copied by the handoff tool. Protocol
filesystem paths are scoped to the worktree. Tests cover path traversal, symlink
escape, interleaved client requests and error-body redaction.

The first prompt hit an absolute timeout while still working. The client now uses
an inactivity timeout. A later machine sleep disconnected the adapter. Session
listing/loading now resumes the latest session only when its cwd exactly matches
the handoff worktree. Visual files are preserved. Result packet, reviewed screenshots
and completed integration are still required before calling this spike passed.

## Six-target runnable artifacts — partial evidence

| Target | Build | Execution | Distribution status |
|---|---|---|---|
| macOS arm64 | Passed locally | 120 ticks, x≈6, document roundtrip passed | Local unsigned executable |
| Web/WASM | Passed locally | In-app browser logged INCANT_SMOKE_PASS | Local static folder |
| iOS arm64 simulator | Passed locally | Scene lifecycle, C ABI and ECS assertions passed | Ad-hoc simulator .app only |
| Windows | CI source prepared | Pending runner | No artifact yet |
| Linux | CI source prepared | Pending runner | No artifact yet |
| Android | Gradle/JNI/NDK source prepared | Pending SDK and runner/device | No artifact yet |

The iOS host was corrected to adopt the scene lifecycle required by the installed
SDK before recording an execution pass. A compiled but failed launch was not counted.
The Rust LLVM tools component was required to resolve the local WASM linker's LLVM
library. No device signing certificate or platform release has been produced.

GitHub workflows exist and a private remote was created at
https://github.com/snowdamiz/incant. No remote run has completed yet. Hosted probes bootstrap
validation; the plan's self-hosted graphics/device runners and nightly artifact
history are not provisioned. Signing certificates and Year 1 staffing are director
prerequisites. Phase 0 is not ready for approval.

### Native interaction check during the visual revision

On 2026-10-08, the bundled macOS development app loaded `tauri://localhost`,
received the native project/schema snapshot, and exposed the actual hierarchy
and Transform inspector through accessibility. Double-click rename changed Cube
to Native rename check; Undo restored Cube; Redo restored Native rename check.
The history count and button availability followed the engine results. These are
functional observations of the earlier UI, not visual approval of the redesign.
Launching a proper `.app` bundle made the binary discoverable for native UI tests;
`tools/editor-dev.py` now creates that unsigned development bundle.

The director requested a substantially stronger visual design with a custom
titlebar. Claude acknowledged this through ACP and is revising the UI. The director
also authorized open permissions for that work; the session uses the adapter’s
explicit `bypassPermissions` mode, without changing global Claude settings. ACP
filesystem requests remain worktree-scoped; tool execution follows the handoff
instructions and is not an OS sandbox. The existing limits on credentials,
publishing, merging and outside-worktree edits remain in the prompt.

### Completed visual handoff and integration

Claude Opus 5.5 returned the first handoff through the ACP runner; the received
commit is integrated as `90b4213` with its `Built-by: claude` trailer. Revision 1
substantially changes the UI in response to the director's feedback and retains
the first design as before evidence. The result and Chrome captures are in
`handoffs/0001-editor-foundation`. The browser titlebar variants are explicitly
labeled test doubles; they do not prove native OS behavior.

After integration, 158 UI/bridge tests, 29 Rust behavior tests, four Python tool
tests, strict TypeScript builds and workspace clippy passed. The native app loads
the revised UI with the real project and native bridge. The host implements the
window actions, publishes focus/maximize/fullscreen metadata and applies the
specified macOS traffic-light inset and canvas color. Rounded viewport radii are
converted to physical pixels and used by a GPU composition mask. The mask is
native GPU work with no readback; render pipelines are cached per target format.
These changes still require Claude's actual native pixel review (packet 0002).

### First remote CI results

Draft PR #1 is https://github.com/snowdamiz/incant/pull/1. Its complete macOS check
workflow passed at commit 356c11a, including the bundled release editor build.
The platform workflow built, executed and uploaded runnable probes for macOS,
Windows, Linux, iOS simulator and browser/WASM. Downloaded reports are recorded
in `evidence/ci-pr-2026-10-08.json`. The Android job stopped because `sdkmanager`
was not on PATH; the workflow now names the installed SDK tool explicitly, with
its rerun pending. No license-acceptance bypass was added.

These are PR-triggered hosted jobs. Scheduled runs on main, their history,
physical devices, graphics runners and signed distribution are still unproven.
