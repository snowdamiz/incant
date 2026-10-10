# AI-Native Game Engine: Start-to-Finish Plan

Engine name: **Incant** (decided 2026-10-08; the shortlist it was chosen from is in Appendix D).
Working name for the launch game: **Driftwake** (placeholder).

Plan author: Claude, for Andrey Yurlov. Date: 2026-10-08. Revision 2 (same day): renamed engine, added agent-first execution model and Claude ACP visual handoff. Revision 3 (2026-10-09): filled the networking and online-services gaps: transport and service rows in 2.1 and 2.2, Phase 6 rewritten, Sections 6.8 and 6.10, additions to Phases 5, 7, 8 and 9, risks, metrics, budget, open decisions 10 to 17, and network entries in Appendices A to C. The same revision filled engine gaps (navigation, 2D, text and localization, lighting for runtime-assembled content, gameplay analytics, Steam Deck and video playback, editor versioning, haptics), recorded the decided stack rows (Rapier, Loro, canonical JSON, React) and the dispositions of open decisions 1 to 9, and re-baselined the timeline at the top of Section 5.
Planning horizon: 36 months, starting November 2026, ending with a shipped game in Q4 2029.

---

## 0. How to read this document

1. Section 1 defines the product, the principles that must not be violated, and what "done" means.
2. Section 2 is the technical architecture: stack, repo layout, and the data model everything hangs on.
3. Section 3 covers accounts, login, and the OpenAI integration requirement in detail.
4. Section 4 is the execution model and team: which AI agents build what, the hard rule that routes visual work to Claude 5.5 over ACP, and the small human team around them.
5. Section 5 is the phase-by-phase timeline with deliverables and exit gates.
6. Section 6 details each engineering workstream.
7. Section 7 is the game production plan.
8. Sections 8 through 12 cover QA, risks, metrics, budget, and open decisions.
9. Appendices contain the agent tool surface spec, the document schema sketch, and a glossary.

Every phase has an **exit gate**. Do not start the next phase until the gate passes. Gates are the only real schedule control in a project this size.

Director decision, 2026-10-08: begin Phase 1 while the remaining Phase 0 gate
items stay explicitly open. The director also instructed: “next time dont ask,
just continue if it makes sense to do so”. Use engineering judgment to continue
useful implementation without repeated phase-advancement confirmations. Keep
deferred evidence and human-owned actions tracked; do not mark gates passed,
claim completed releases, or waive final requirements without their evidence.
This overrides the stop-before-next-phase rule above for implementation progress.

Director decision, 2026-10-09: agents merge completed PRs into main once the scoped
work is reviewed and its required checks pass. Do not wait for a separate director
merge or request repeated merge confirmation. Resolve integration conflicts and
verify the result. Merging implemented work does not approve a phase gate or mark
deferred requirements complete; the remaining human-owned actions still apply.

---

## 1. Vision, principles, and definition of done

### 1.1 One-paragraph vision

A single application that is a 3D/2D modeling tool, a game engine, and an editor, where the human and an AI agent work on the same project through the same interface. The user can click and drag like Unity or Blender, or type "add a grappling hook that pulls the player toward the hit point, test it, and show me" and watch it happen. Games built in it export to Windows, macOS, Linux, iOS, Android, and the web.

### 1.2 Non-negotiable principles

1. **One command bus, many clients.** Every edit, whether from a mouse, a keyboard shortcut, a script, or the agent, is a command on a single bus. There is no GUI-only path and no AI-only path.
2. **The project is a text-native, schema'd document.** Scenes, prefabs, materials, graphs, animation state machines, and settings are all human-readable, diffable, and validated against a published schema. Binary data (meshes, textures, audio) lives in referenced asset files, never inline.
3. **The agent sees what the user sees.** Viewport screenshots, console, profiler, play-test results, and validation errors are all available as tools. No "the AI guessed and hoped."
4. **Every agent action is an undoable transaction.** The user can review, revert, or amend anything the agent did, with the same history UI used for manual edits.
5. **Bring-your-own-model.** The user connects their own OpenAI account. We never proxy model calls through our servers by default, and we never store their keys or tokens server-side.
6. **Dogfood everything.** The launch game is built in the engine, by the team, with the agent. Anything the game needs that the engine lacks is an engine bug.
7. **Rust core, TypeScript gameplay, web editor chrome.** Chosen for safety, portability, and LLM fluency. Revisit only with strong evidence.
8. **Agent-first build with a hard routing rule.** The engine and game are built primarily by Astra running in OpenAI Codex. Any visual work is handed off to Claude 5.5 over ACP (Agent Client Protocol) using the director's already logged-in Claude account. Section 4 defines "visual" precisely. This is a build-time arrangement; the engine's end users still bring their own OpenAI account (Section 3), and the shipped engine never embeds the director's credentials.

### 1.3 Definition of done

The project is done when all of the following are true:

- **Engine 1.0 is released** with signed installers for macOS and Windows, a Linux build, and a browser-based editor mode.
- **Driftwake is shipped** on Steam (Windows, macOS, Linux), the iOS App Store, and Google Play, and has passed platform certification.
- Driftwake is a 3D co-op action roguelite with online multiplayer for up to four players, cross-play between desktop and mobile, six to ten hours of content, and cloud saves.
- Driftwake's online services (sessions, relay, player identity and cloud saves) run in production in every launch region with monitoring, backups, runbooks and an on-call rotation, and an engine user can self-host the same services from the documentation alone (Section 6.10).
- At least 60% of Driftwake's gameplay scripts, scene edits, and material setups were authored or modified through the in-engine agent (measured, not estimated, via command-bus provenance tags).
- A new user with an OpenAI account can install the engine, sign in, connect OpenAI, and have a playable prototype from a template with one custom mechanic in under fifteen minutes.
- Public documentation, a template library, and a sample project gallery exist.

---

## 2. Architecture

### 2.1 Stack decisions

| Layer | Choice | Why | Fallback |
|---|---|---|---|
| Engine core | Rust, built on Bevy (ECS, scheduling, asset system) | Memory safety, cross-compilation to all six targets, active ecosystem | Custom ECS if Bevy's scheduler becomes a bottleneck |
| Rendering | wgpu, custom render graph on top of Bevy's | Metal, Vulkan, DX12, WebGPU from one codebase | Vulkan-only via ash for desktop high-end tier |
| Physics | Rapier 3D and Rapier 2D behind a Rust-owned boundary (ADR 0003) | Pure Rust builds on all six targets including the iOS simulator and WASM, where the measured Jolt bindings failed; `enhanced-determinism` for rollback and replay | Jolt through a maintained portable C++ shim if representative game or mobile measurements show a material issue |
| Audio | Kira plus a thin mixer layer | Pure Rust, good for games | FMOD via FFI for studios who require it |
| Game UI | Taffy (flexbox layout) plus custom widget tree | Familiar web-style layout, agent writes it easily | None needed |
| Gameplay scripting | TypeScript on a sandboxed QuickJS runtime, hot reload | LLMs write TS best; sandboxable; users already know it | V8 via deno_core on desktop for perf-heavy projects |
| Native extensions | Rust plugin ABI with stable C interface | Escape hatch for perf-critical code | None |
| Project document | Canonical typed JSON with derived JSON Schemas, Loro CRDT in memory (ADR 0008) | Diffable, mergeable, agent-readable; both CRDTs were benchmarked and Loro won | Automerge behind the same document boundary |
| Editor shell | React and TypeScript web UI inside Tauri (ADR 0009) | Fast UI iteration, same UI reused in browser mode | egui native UI |
| Viewport | Native wgpu surface composited into the Tauri window | Zero-copy rendering | Texture streaming over shared memory |
| Agent runtime | Local agent loop in the editor process, provider-agnostic | Keeps user data local, supports BYO keys | Hosted agent for teams who opt in |
| Accounts service | Small Rust (axum) service plus Postgres | Login, project sync, licensing | Managed auth provider (see 3.2) |
| Game transport | QUIC via quinn (TLS 1.3, reliable streams, unreliable datagrams, connection migration); WebRTC data channels in browsers | Encrypted and NAT-friendly by default, one code path for reliable and unreliable traffic, survives Wi-Fi to cellular switches | Custom UDP reliability layer over a Noise handshake |
| Online game services | Rust (axum) plus Postgres: sessions and matchmaking, relay, player identity and cloud saves, dedicated-server allocator; shipped as self-hostable containers | Same stack as accounts; "multiplayer in the box" that Driftwake dogfoods; no third-party runtime dependency | Epic Online Services, Nakama or PlayFab behind the platform-services abstraction |
| Asset generation | Pluggable providers (image, 3D, audio, animation) | Models change every six months | None |
| CI/CD | GitHub Actions, self-hosted macOS and Windows runners, device farm | Required for six targets | Buildkite |

Revision 3 added the game transport and online game services rows. Like the stack ADRs, they are engineering proposals until the director reviews them (open decisions 11 and 12). The same revision recorded the physics, project document and editor shell rows as decided in ADRs 0003, 0008 and 0009; those remain engineering decisions pending the director's Phase 0 architecture review.

### 2.2 Repository layout (monorepo)

```
incant/
  crates/
    incant_core        ECS glue, scheduling, events, time
    incant_doc         Project document model, schema, CRDT, validation
    incant_cmd         Command bus, transactions, undo, provenance
    incant_render      Render graph, materials, lights, GI, post
    incant_geo         Procedural geometry graph, mesh ops, UV, retopo, LOD
    incant_shader      Shader graph to WGSL compiler
    incant_anim        Skeletal animation, blend trees, IK, retargeting
    incant_physics     Rapier 3D and 2D behind a Rust-owned boundary, colliders, character controller
    incant_audio       Mixer, spatialization, buses
    incant_ui          Game UI layout and widgets
    incant_text        Fonts, shaping, bidi, line breaking, string tables, locale, IME bridge
    incant_2d          Sprites, atlases, tilemaps, sorting layers, 2D camera and lights
    incant_nav         Navmesh generation, pathfinding, steering, grid navigation
    incant_script      QuickJS host, TS bindings, hot reload, sandbox
    incant_net         Transport, sessions, replication, prediction, rollback, authority, determinism toolkit, netsim
    incant_assets      Cook, stream, hash, cache (CPU only, no editor dependency)
    incant_import      Editor and CLI import coordination through the command bus
    incant_input       Fixed-tick input processing, device adapters, recordings
    incant_agent       Agent loop, tool registry, providers, skills
    incant_headless    Headless runner for play-tests and CI
    incant_platform_smoke  Runnable cross-platform probe of the document and Bevy crates
    incant_export      Build pipeline for all targets
    incant_runtime     The shipped game runtime (no editor)
  editor/
    app/              Tauri shell
    ui/               Web UI (panels, inspectors, chat, history)
    bridge/           IPC between UI and engine process
  services/
    accounts/         Login, provider connections metadata, licensing, sync
    telemetry/        Opt-in crash and usage reporting
    sessions/         Lobbies, room codes, matchmaking, session tickets, WebRTC signaling
    relay/            Regional QUIC/UDP relay for NAT traversal
    player/           Optional cross-store player identity, account linking, cloud saves, deletion, export
    analytics/        Consent-gated gameplay event ingestion and aggregation
    allocator/        Dedicated-server fleet allocation (reference deployment; optional for Driftwake)
    deploy/           Infrastructure as code for every service; never credentials
  sdk/
    ts/               @incant/runtime TypeScript types and helpers
    templates/        Starter projects
    skills/           Agent skills and recipes
  games/
    driftwake/        The launch game
  evals/              In-engine agent eval tasks and recorded results
  platforms/          iOS, Android and web probe projects and packaging
  schemas/            Published JSON Schemas derived from the Rust types
  website/            Public site and landing page
  licenses/           Third-party license texts referenced by THIRD_PARTY_NOTICES.md
  handoffs/           One folder per Claude handoff packet and result
  artifacts/          Local build outputs and recorded evidence
  docs/               ADRs, spikes, gate ledgers, generated agent conventions
  tools/
```

### 2.3 The project document model

This is the heart of the system. Treat it as a product in its own right.

- **Everything is an entity with components**, including scenes, prefabs, materials, and graphs. Components are typed structs with a schema, a version, and migration functions.
- **Stable IDs.** Every entity and asset gets a ULID at creation. Names are labels, never keys.
- **Text on disk, CRDT in memory.** Files are deterministic, sorted, and formatted so git diffs are small. In memory, the document is a CRDT so concurrent edits (two humans, or a human and the agent) merge without conflicts.
- **Schema registry.** Every component and asset type registers a JSON Schema. The agent receives these schemas as tool documentation, so it never has to guess field names.
- **Validation is a first-class result.** Any patch returns a list of errors and warnings with paths. The agent uses these to self-correct.
- **Provenance.** Every command records its origin: user, agent (with model and conversation ID), script, or import. This powers the "60% built by the agent" metric and the history UI.

### 2.4 The command bus

- A command is a named operation with typed arguments, a `apply`, an `invert`, and a `describe` for the history panel.
- Commands compose into transactions. A transaction is atomic and undoable as a unit.
- The GUI emits commands. Scripts emit commands. The agent emits commands. The headless runner emits commands.
- Commands are serialized to a journal, which enables replay, crash recovery, and session sharing.

### 2.5 Process model

- **Editor process** (Rust): owns the engine, the document, the command bus, the agent loop, and the native viewport surface.
- **UI process** (Tauri webview): panels, inspector, chat, history, asset browser. Talks to the editor process over a typed IPC channel.
- **Play-test processes**: spawned headless or windowed runtime instances. They report screenshots and logs back to the editor process.
- **Browser mode**: engine compiled to WASM runs inside the same web UI. Reduced feature set (no native plugins, no local file system without the File System Access API).

---

## 3. Accounts, login, and OpenAI integration

This section exists because it is an explicit requirement: users must be able to log in and use their existing OpenAI accounts for the AI work.

### 3.1 What "use their OpenAI account" means in practice

Two connection paths, both supported, because OpenAI's own availability of each changes over time:

1. **OAuth sign-in with OpenAI (preferred when available).** OpenAI exposes an OAuth-based "Sign in with ChatGPT" flow used by their Codex tooling. Where OpenAI permits third-party applications to use it, the engine opens the system browser, the user authorizes, and the engine receives tokens via a loopback redirect with PKCE. Usage is billed against the user's ChatGPT plan or organization according to OpenAI's rules for that flow.
2. **API key connection (always available).** The user pastes an API key from their OpenAI dashboard. The engine validates it with a cheap models-list call and stores it in the OS keychain. Usage is billed to their OpenAI API account.

Design decisions:

- **Model calls go directly from the user's machine to OpenAI.** Our servers never see prompts, project data, keys, or tokens. This is both a privacy stance and a cost stance.
- **Keys and tokens live in the OS keychain** (macOS Keychain, Windows Credential Manager, libsecret on Linux). In browser mode, they live in encrypted IndexedDB with a user passphrase, and the UI warns about the weaker guarantee.
- **The provider abstraction is generic.** OpenAI is the required first provider. Anthropic, Google, and local models (via an OpenAI-compatible endpoint) are second-tier providers added once the abstraction is proven. The user can connect several and pick per task.
- **Token refresh and revocation** are handled by the engine. Disconnecting a provider deletes local credentials and, for OAuth, calls the revoke endpoint.
- **Cost transparency.** Every agent turn shows tokens used and estimated cost, with a per-project budget cap the user sets. The agent is told its remaining budget and plans accordingly.

Director decision, 2026-10-08: repeated macOS Keychain password prompts during
development are unacceptable. For the current macOS implementation, store Incant's
credentials in atomic owner-only files in its stable application-support directory
(0700 directory, 0600 files), surviving relaunches and changed development builds.
This replaces the macOS Keychain choice above; it relies on OS file permissions,
without additional encryption. Windows/Linux continue using their OS credential
stores. Do not read or migrate legacy Keychain entries automatically. Evidence and
scope are in docs/spikes/auth-login-repair.md.

### 3.2 Engine account (our login)

Separate from the OpenAI connection. The engine account provides:

- Identity for licensing, cloud project sync, collaboration invites, and the template gallery.
- Sign-in methods: email magic link, Google, GitHub, Apple (required for iOS App Store distribution of the editor if we ever ship it there).
- Implementation: a small Rust service, or a managed provider (Clerk, Auth0, or Cognito) if we decide identity is not core. Recommendation: managed provider for the first two years, with an abstraction so it can be swapped.

The engine stores **only metadata** about provider connections server-side: "this user connected OpenAI via OAuth on this date." Never the credential.

### 3.3 Offline and no-account modes

- The editor works fully offline with no engine account. The agent requires a provider connection, so it is unavailable offline unless a local model is connected.
- No feature of the manual editor is gated behind login. Login gates sync, collaboration, and publishing to our gallery.
- Shipped games have no dependency on Incant services unless they use multiplayer, cross-store cloud saves or other online features, and those features degrade to offline play when the services are unreachable. A single-player game built with Incant runs with no network access at all.

### 3.4 Agent-to-provider plumbing

- A single `Provider` trait: `complete(messages, tools, options) -> stream of events`. Supports streaming, tool calls, structured outputs, and vision inputs. OpenAI's Responses API is the reference implementation.
- Model routing: the user picks a default model per role (planner, coder, vision checker, cheap classifier). Defaults are set per provider and updated with engine releases.
- Retry, backoff, and rate-limit handling live in the provider layer. The agent loop never sees transport errors unless they are terminal.
- Prompt caching is used wherever the provider supports it. The schema registry and tool definitions are the stable prefix.

### 3.5 Security and compliance checklist

- Threat model documented for: stolen keychain entries, malicious project files that contain prompt injection, malicious templates, and malicious scripts.
- Prompt injection defense: project content shown to the agent is wrapped and labeled as data. The agent cannot execute shell commands on the user's machine, only engine commands and sandboxed scripts.
- Scripts run in a sandbox with no file system or network access unless the project grants a capability, and the user confirms the grant.
- Asset imports are parsed with fuzzed, memory-safe parsers. Native plugins require explicit user trust.
- OpenAI usage policies are surfaced in the connection UI. We do not resell or meter their service.

---

## 4. Execution model and team

### 4.1 Agent-first build

This plan is executed primarily by AI coding agents under human direction. Two agents do the building, one human directs.

| Actor | Runs in | Account used | Owns |
|---|---|---|---|
| **Astra** | OpenAI Codex (cloud tasks and the Codex CLI) | Andrey's OpenAI account | Everything that is not visual: architecture, Rust crates, document model, command bus, scripting runtime and TS SDK, physics, audio, animation runtime, networking, asset pipeline, platform and export, backend services, CI, tests, eval graders, documentation text, Driftwake gameplay and systems code. Also integrates every Claude handoff and opens all PRs. |
| **Claude 5.5** (Opus 5.5 by default) | Claude Code, reached over ACP (Agent Client Protocol) | Andrey's already logged-in Claude account. No API key, nothing stored in the repo or CI | All visual work as defined in 4.2: editor UI and UX design and implementation, viewport and gizmo visuals, look-dev, shader and material appearance, VFX, lighting tuning, golden-image and screenshot review, art direction, Driftwake art, HUD, trailer and store art, diagrams and site design. |
| **Andrey** (director) | | | Priorities, phase gate approvals, routing tie-breaks, and anything requiring a human: store and developer accounts, signing keys, payments, legal, age ratings. |

The build-time agents are separate from the in-engine agent that end users talk to. End users connect their own OpenAI account (Section 3). The engine, the templates, and the shipped game must never contain or depend on the director's Codex or Claude sessions.

### 4.2 Routing rule: what counts as visual work

A task is handed to Claude if any of these is true:

1. Acceptance can only be judged by looking at rendered pixels or a laid-out screen.
2. The deliverable is a design: layout, color, typography, iconography, motion, or composition.
3. The task is reviewing screenshots, golden images, playtest footage, or art.
4. The task is look-dev: materials, lighting, post-processing, VFX, or shaders judged by appearance rather than by math or performance.

Everything else stays with Astra. Astra owns correctness, performance, architecture, and data. Claude owns appearance and judgment-by-eye. When a task has both parts it is split: Astra implements, Claude reviews the result visually and returns either approval or concrete diffs.

| Area | Astra (Codex) | Claude 5.5 (ACP) |
|---|---|---|
| Editor | Panel data binding, IPC bridge, command wiring, schema-driven inspector logic, undo and history mechanics | Panel layouts, inspector visual design, chat UI, history UI, gizmo visuals, icons, themes, empty states, onboarding flow design, accessibility visual checks |
| Rendering | Render graph, culling, GPU buffers, shader compiler, tier system, performance budgets | Tonemapping, bloom, TAA and GI quality judgment, golden image approval, visual regression triage, reference scenes |
| Shaders and materials | WGSL codegen, graph compiler, node evaluation | Node library appearance, default material library, sample materials, look-dev |
| Content tools | Geometry graph evaluator, mesh ops, retopo, UV, LOD algorithms | Graph editor UI, default presets, the two-day environment exit test in Phase 4 |
| Animation and VFX | Playback, blend trees, IK solvers, particle simulation | Animation feel review, effect authoring, particle look |
| In-engine agent | Agent loop, tools, providers, skills, eval graders | Vision-check graders and their reference images, chat UI, inline diff presentation |
| Platforms | Export pipeline, store packaging, device farm, crash reporting | Store screenshots, icons, splash screens, platform UI conformance review |
| Driftwake | Gameplay scripts, netcode, save, progression, procedural generation logic | Art bible, HUD and menus, level visual dressing, lighting, VFX, trailer, store art |
| Networking | Transport, replication, prediction and rollback, services, protocol versioning, load and soak tests | Network simulation panel, connection quality indicators, lobby, invite, rejoin and forced-update screens, desync inspector layout |
| 2D | Atlas packing, tilemap data and autotile evaluation, 2D physics, sorting | Sprite and tilemap editor UI, 2D lighting look, template art |
| Text and localization | Shaping, bidi, string tables, locale data, IME bridge | Font fallback and right-to-left layout review, localization panel design, pseudo-localization visual check |
| Navigation | Navmesh generation, pathfinding, steering | Navmesh and path debug visualization design |
| Docs and marketing | Reference and tutorial text, API docs | Diagrams, screenshots, site design, video |

If routing is ambiguous, Astra asks the director in the task thread and defaults to the split pattern in the meantime.

### 4.3 Handoff protocol (Astra to Claude over ACP)

1. **Packet.** Astra writes a self-contained handoff packet to `handoffs/<id>/brief.md`: goal, acceptance criteria, relevant file paths, build and run commands, the headless-runner command that captures screenshots, reference images, constraints (performance budgets, target platforms, tiers), and exactly what to return.
2. **Session.** Astra runs `tools/handoff run <id>`. The tool opens an ACP session to Claude Code through the Claude Code ACP adapter. The session inherits the director's logged-in Claude account. No API key or token is written anywhere.
3. **Work.** Claude works in a git worktree on branch `handoff/<id>`, runs the visual regression suite, and writes `handoffs/<id>/result.md` with before and after screenshots, rationale, and open questions.
4. **Integration.** Astra reviews the result, merges the worktree, opens the PR, and runs full CI. Once the scoped work is complete, reviewed, and its required checks pass, Astra merges the PR into main under the director’s standing authorization. Every commit carries a trailer `Built-by: astra` or `Built-by: claude` so provenance covers engine development as well as game content.
5. **Fallback.** If ACP or the Claude account is unavailable, the packet stays in `handoffs/` and the director runs Claude Code on it manually. Packets are written so that this always works.
6. **Batching.** Non-urgent visual tasks are batched per sprint to respect subscription limits. Blocking ones go immediately.

Repository files that make this work:

- `AGENTS.md` at the repo root. Codex reads it. Contains the routing rule, the handoff procedure, coding conventions, and the list of human-only actions.
- `CLAUDE.md` at the repo root. Claude Code reads it. Contains visual conventions, screenshot and golden-image commands, the design system, and the result format.
- `tools/handoff/` CLI, `tools/acp/` adapter configuration, `handoffs/` directory with one folder per handoff.
- Both agent instruction files are generated from a single `docs/conventions/` source so they never drift.

### 4.4 Accounts used during the build

- Astra uses the director's OpenAI account through Codex.
- Claude uses the director's logged-in Claude Code session. Nothing is stored server-side or in CI.
- CI never calls either agent. CI runs builds, tests, golden images, device farm, and the in-engine agent eval harness. The eval harness uses dedicated eval provider keys from the CI secrets store, and those keys are only ever used by the eval harness.
- Human-only: Apple, Google, and Steam developer accounts, signing keys, payment setup, legal filings, age rating submissions, cloud provider accounts and billing for live services, DDoS protection contracts, legal review of the privacy policy, age gates and account deletion flows, and accountability for the on-call rotation.

### 4.5 Human team

The agents do the building. Humans direct, review, and do what agents cannot.

| Role | Year 1 | Year 2 | Year 3 | Notes |
|---|---|---|---|---|
| Director, product, architecture approvals | 1 | 1 | 1 | Andrey |
| Senior graphics reviewer | 0.5 | 1 | 1 | Reviews rendering PRs and Claude look-dev results; part-time in Year 1 |
| Platform and release engineer | 0 | 1 | 1 | Store accounts, signing, certification, device lab ownership |
| Game designer | 0 | 1 | 2 | Owns Driftwake's design document and playtests |
| QA and device lab | 0 | 1 | 2 | Physical devices, store-submission dry runs |
| Audio and composer (contract) | 0 | 0.5 | 1 | |
| Backend and live operations | 0 | 0.5 | 1 | Owns the online services in production, on-call accountability, load tests and cost; agents triage and fix |
| **Total** | **1.5** | **6** | **9** | |

### 4.6 Operating rules

- Every PR is agent-authored and agent-merged into main after review and passing required checks. No separate merge confirmation is needed. The director approves every phase gate; a PR merge does not approve a gate.
- Weekly the director reviews the handoff log, eval results, and friction logs, and resolves any routing disputes.
- Agents never self-approve a routing exception. Ambiguity defaults to the split pattern.
- Both agents dogfood the product. Astra and Claude build Driftwake content through the in-engine agent wherever possible, which is what the provenance metric measures.
- Agent-first execution runs around the clock and parallelizes by crate, so the 36-month schedule in Section 5 is conservative. Phases 1 through 4 may compress. Phases 5 and 9 will not, because they are bound by physical device testing and store review times.

---

## 5. Phases and timeline

Start: November 2026 in the original baseline; actual start 2026-10-08. Durations overlap deliberately. Each phase lists goals, deliverables, and the exit gate.

Baseline (revision 3, 2026-10-09): the dates in the phase headings and in 5.1 are the original human-paced baseline and remain upper bounds. Under the agent-first model the Phase 0 spikes were completed within days of 2026-10-08, and the director authorized Phase 1 with the Phase 0 gate items still open. Phase windows are now driven by exit gates rather than calendar months: a phase starts when the previous gate review authorizes it, and each gate review re-baselines the next window from measured progress and records it in this table. The Q4 2029 shipped-game target stands until a gate review moves it.

| Phase | Baseline window | Actual start | Status on 2026-10-09 |
|---|---|---|---|
| 0 | Nov 2026 to Jan 2027 | 2026-10-08 | Spikes done; gate open: signing certificates, staffing, nightly history, live revocation, director approval |
| 1 | Feb to Sep 2027 | 2026-10-08 | In progress: import and cook pipeline, physics, character movement, input, saves and headless assertions landed |
| 2 to 9 | As listed below | Not started | Re-baseline at the Phase 1 gate review |

### Phase 0: Foundation (Nov 2026 to Jan 2027, 3 months)

Goals: lock architecture, prove the riskiest assumptions, stand up infrastructure.

Deliverables:
- Architecture decision records for every row in the stack table.
- Spike 1: Bevy app rendering into a native surface inside a Tauri window on macOS and Windows, with the web UI overlaying it. This is the riskiest UI assumption.
- Spike 2: Project document round-trip. Load a Bevy scene into the CRDT document, edit it from two clients concurrently, save to text, reload, diff cleanly in git.
- Spike 3: QuickJS inside Bevy with hot reload, calling into ECS queries, at 60 fps with a thousand scripted entities.
- Spike 4: Agent loop against OpenAI with three tools (query scene, patch scene, screenshot) performing a ten-step edit on a sample scene. Measure success rate on twenty scripted tasks.
- Spike 5: OAuth loopback flow with OpenAI on all three desktop OSes, plus API-key fallback, with keychain storage.
- Spike 6: Handoff tooling. Astra opens an ACP session to Claude Code using the director's logged-in account, hands off a mock visual task (design the hierarchy panel and return screenshots), and receives the result packet. `AGENTS.md`, `CLAUDE.md`, and `tools/handoff` exist and are used for every spike after this one.
- Monorepo, CI on all six targets building a hello-world, signing certificates obtained for Apple and Windows.
- Director, part-time graphics reviewer, and contract arrangements in place for Year 1.

Exit gate:
- All six spikes pass with written results.
- Spike 6 round-trips a handoff end to end with no credentials written to disk.
- Spike 4 succeeds on at least 14 of 20 tasks without human help.
- CI produces a runnable artifact for every target nightly.

Director decision, 2026-10-08: manual Windows/Linux sign-in checks and the manual
Windows native viewport review are replaced by CI builds and automated tests.
Use native editor/engine builds, shared UI/bridge tests, GPU readback probes and
synthetic credential persistence across changed executables on those CI hosts.
Do not require the director to provide Windows/Linux desktop machines or perform
interactive logins there. Retain the macOS live-account and native-review evidence;
CI results must state which behavior was exercised, without claiming manual or
live-account validation. This change does not waive other phase gates.

### Phase 1: Core engine (Feb 2027 to Sep 2027, 8 months)

Goals: a runtime that can run a real 3D game without an editor.

Deliverables:
- Render graph with clustered forward+ lighting, PBR materials, shadow cascades, SSAO, bloom, tonemapping, TAA. Mobile tier with reduced features.
- Asset pipeline: glTF, FBX (via converter), PNG/JPG/EXR, WAV/OGG import. Cook to KTX2 and meshopt. Content-addressed cache. Hot reload of assets.
- Physics: Rapier 3D and 2D behind a Rust-owned boundary (ADR 0003), colliders, rigid bodies, character controller, raycasts, triggers, deterministic step mode.
- Audio: mixer with buses, 3D spatialization, streaming for music.
- Animation: skeletal playback, blend trees, state machines, two-bone IK, root motion.
- Game UI: flexbox layout, text rendering with font atlases over a shaping stack (cosmic-text: rustybuzz shaping, bidi, Unicode line breaking, font fallback chains for CJK and emoji), a text input widget with IME composition on desktop and the platform on-screen keyboard on mobile, input focus, gamepad navigation.
- Scripting: full TS SDK covering entities, components, input, haptics, physics queries, navigation, audio, UI, localization, 2D, timers, coroutines. Type definitions generated from the schema registry.
- Headless runner: run a project for N seconds, capture frames and logs, exit with a status.
- Input: keyboard, mouse, gamepad, touch with gesture recognition; haptics (gamepad rumble and mobile haptic feedback) through one API.
- Save system: serialize game state to a versioned format.
- Navigation: runtime navmesh generation from colliders and meshes (recast-style voxelize, region, contour, polygonize) built in tiles so runtime-assembled rooms rebuild only changed tiles; A* with funnel string-pulling; agent steering with local avoidance; off-mesh links; grid and tilemap navigation for 2D; debug draw. Crowd simulation is out of scope.
- 2D: sprite rendering with atlases packed at cook time, nine-slice, sorting layers, orthographic and pixel-perfect cameras; rectangular tilemaps with collision generation; sprite-sheet flipbook animation; Rapier 2D behind the same physics boundary. Shares the render graph; the mobile tier gets the full 2D feature set. Skeletal 2D animation and isometric or hex tilemaps are 1.1.
- Localization: string tables as typed document assets with per-locale values, plural, select and gender rules (an ICU MessageFormat subset), fallback chains, locale-aware number and date formatting through ICU4X, runtime locale switching, a pseudo-localization mode and missing-string reports. The editor UI ships in English at 1.0 and reuses the same string tables for its own localization in 1.1.

Exit gate:
- A "Core Sample" test game (third-person character on a terrain with enemies, pickups, UI, music) runs at 60 fps on an M1 MacBook Air, a mid-range Windows laptop, an iPhone 13, and a Pixel 6, written entirely in TypeScript against the SDK with no editor.
- Headless runner used in CI to play Core Sample and assert on game state.
- Core Sample's enemies navigate by navmesh, its UI passes pseudo-localization and renders one right-to-left and one CJK locale correctly, and a 2D sample scene (tilemap, sprites, 2D physics) runs on the same four devices.

### Phase 2: Editor and document model (May 2027 to Feb 2028, 10 months, overlaps Phase 1)

Goals: a usable editor built entirely on the command bus.

Deliverables:
- Document model complete: schema registry, migrations, validation, CRDT, text serialization, provenance.
- Command bus complete: transactions, undo/redo, journal, replay.
- Editor panels: hierarchy, inspector (schema-driven, auto-generated for any component), viewport with gizmos (translate, rotate, scale, snapping), asset browser, console, profiler, history.
- Scene editing: create, parent, duplicate, prefab instancing with overrides, multi-select.
- Material editor: property-based (node graph arrives in Phase 4).
- Play-in-editor: press play, the runtime runs in the viewport, pause, step, inspect live state, stop restores the document.
- Project templates: empty, 3D third-person, 2D platformer, top-down, first-person.
- Settings, project settings, input mapping editor.
- 2D editing: tilemap painter with autotile rules, sprite and atlas inspector, collider-from-alpha tool.
- Localization panel: string table editor with per-locale columns, a missing-string filter, and XLIFF export and import for translators.
- Engine version pinning: a project declares the engine version range it requires. Opening an older project offers migration as a reviewable transaction with a backup; a newer format is refused with a clear message.
- Browser mode: the editor web UI running against the WASM engine, read-only viewport first, editing second.
- Engine account sign-in and project cloud sync (single user, last-write-wins at file level, CRDT merge at document level).

Exit gate:
- Core Sample can be rebuilt from an empty project using only the editor in under four hours by someone who has never seen the code.
- Every editor action is a command on the bus (verified by a test that disables direct document mutation and runs the UI test suite).
- Undo/redo passes a fuzz test of ten thousand random command sequences with document equality checks.
- The 2D platformer template is rebuilt from an empty project through the editor alone and plays with tilemap collision, sprite animation and a following camera.

### Phase 3: AI layer (Sep 2027 to Jun 2028, 10 months, overlaps Phase 2)

Goals: the agent is a genuine second user of the editor.

Deliverables:
- Provider layer with OpenAI OAuth and API key, keychain storage, cost tracking, model routing. Anthropic and local OpenAI-compatible endpoints as second providers.
- Agent loop: planning, tool calls, streaming to chat UI, interruption, approval modes (auto, ask for destructive, ask always).
- Tool surface v1 (see Appendix A): document query and patch, schema lookup, asset listing, screenshot, console, profiler, play-test, script read/write, validation, search.
- Context management: project summary, recent history, relevant schemas, and open panels are assembled per turn. Long projects use a maintained project memory file.
- Skills system: a skill is a markdown playbook plus optional TS helpers. Ship sixteen skills: character controllers (third-person, first-person, 2D platformer, top-down), camera rigs, inventory, dialogue, health and damage, spawners, checkpoints, mobile touch controls, settings menu, main menu, save/load, simple AI (patrol, chase, on the navmesh), day/night cycle, and localize project (drafts translations through the connected provider and flags every string for human review).
- Eval harness: two hundred scripted tasks with automated graders (document assertions, headless play-test assertions, vision checks). Runs nightly against every supported model. Regressions block release.
- Chat UI: inline diffs of document changes, clickable entity references, screenshots inline, "revert this turn" button, cost meter.
- Agent-driven asset generation v1: textures via image models, placeholder meshes via text-to-3D, sound effects via audio models, all through pluggable providers.
- Prompt injection defenses and a red-team pass.

Exit gate:
- Eval pass rate at or above 80% on the two hundred tasks with the default OpenAI model.
- A tester with an OpenAI account and no engine experience builds a playable prototype with one custom mechanic in under fifteen minutes, in five of five attempts.
- No critical findings from the red-team pass remain open.

### Phase 4: Content creation tools (Jan 2028 to Oct 2028, 10 months)

Goals: modeling and look-dev without leaving the engine.

Deliverables:
- Procedural geometry graph: primitives, boolean, extrude, bevel, subdivide, array, scatter, noise displacement, curves, lofts, instancing. Text-serialized, agent-authorable.
- Mesh tools: generative mesh import cleanup, auto-retopo, auto-UV, LOD generation, decimation, normal and AO baking.
- Light sculpting: a minimal brush-based sculpt mode for adjustments, not full Blender sculpting.
- Shader graph compiling to WGSL, with a node library covering PBR inputs, math, textures, UV ops, vertex animation, and custom WGSL nodes. Agent can write WGSL directly.
- Terrain: heightmap editing, layered materials, foliage scatter, streaming.
- Animation tools: retargeting, animation graph editor, IK rigs, timeline for cinematics.
- VFX: GPU particle system with a graph editor.
- Global illumination: baked lightmaps and probe volumes for all tiers, screen-space GI for desktop, hardware RT reflections as an optional desktop tier.
- Lighting for runtime-assembled content: per-template lightmap and probe bakes stored with the template, probe volumes blended across room seams at load, a probe relighting pass on desktop when time of day or destruction changes the lighting, and a mobile path of per-template probes plus analytic lights. Driftwake's room graph is the test case.
- 2D lighting: normal-mapped sprites, 2D point and spot lights with soft shadows at desktop tier, unlit and vertex-lit at mobile tier.
- Asset generation v2: image-to-3D with automatic cleanup to engine-ready meshes, texture set generation (albedo, normal, roughness), animation generation from text for humanoid rigs.

Exit gate:
- A technical artist builds a complete environment (terrain, twenty props, five materials, lighting, VFX) in two days using only the engine, with at least half the props from the geometry graph or generation.
- Agent eval extended with fifty content tasks, pass rate at or above 70%.
- A room graph of ten templates assembled at runtime shows no visible lighting seams in Claude's review at mobile and desktop tiers.

### Phase 5: Platforms and export (Jun 2028 to Jan 2029, 8 months)

Goals: one-click builds that pass store review.

Deliverables:
- Export pipeline for Windows (MSIX and portable), macOS (notarized app), Linux (AppImage), iOS (Xcode project generation plus direct archive), Android (Gradle project plus direct APK/AAB), web (WASM plus a hosting bundle).
- Mobile: touch input layer, safe-area handling, orientation, app lifecycle, battery-aware frame pacing, thermal throttling response, on-device texture compression tiers (ASTC, ETC2).
- Desktop: Steam integration (achievements, cloud saves, overlay, input), window management, display modes.
- Steam Deck as a verified-compatibility target: controller glyphs, 1280 by 800 default, on-screen keyboard, suspend and resume, no external launcher; one Deck in the device farm.
- Video playback through platform decoders (AVFoundation, Media Foundation, MediaCodec, browser video) for WebM/VP9 and MP4/H.264, rendered to a UI texture, with a software VP9 fallback on Linux; for in-game trailers, tutorials and pre-rendered cutscenes.
- Consoles (Nintendo Switch, PlayStation, Xbox) are out of scope for Engine 1.0 and the Driftwake launch. The export pipeline keeps its platform abstraction so a console port is a 1.x decision; it needs NDA SDKs and developer accounts that only the human team can obtain.
- Platform services abstraction: achievements, leaderboards, cloud save (Steam Cloud, iCloud and Google Play Saved Games as store-native mirrors; cross-store sync arrives with the Phase 6 player service), IAP (hooks only, not a store), platform friends and invites.
- Crash reporting and symbolication for all targets.
- Device farm in CI: ten physical devices across iOS and Android, nightly smoke tests.
- Build size and startup time budgets enforced in CI.

Exit gate:
- Core Sample ships to TestFlight, Google Play internal testing, and a private Steam app, from CI, with no manual steps beyond pressing a button.
- Cold start under three seconds on an iPhone 13 and under five seconds on a Pixel 6.

### Phase 6: Networking and online services (Sep 2028 to Mar 2029, 7 months)

Goals: multiplayer in the box, the services needed to run it, and Driftwake's co-op built on them.

Scheduling note: Phase 7 needs two-player co-op by Dec 2028, so transport, replication and the relay come first and may start as early as Jun 2028 alongside Phase 5, which Section 4.6 allows. Service hardening and live-operations readiness continue into Phase 8 (Section 6.10).

Deliverables, transport and sessions:
- Transport: QUIC via quinn on native targets (encryption, congestion control, reliable streams, unreliable datagrams, connection migration across network changes); WebRTC data channels with a signaling endpoint in the sessions service for browser builds. A custom UDP reliability layer over a Noise handshake is the fallback. Every endpoint is dual-stack; IPv6-only networks are a test case because Apple requires them to work.
- Connectivity: direct connection when both peers' NAT types allow it (ICE-style probing), relay otherwise. Relay is the default on cellular and whenever a player chooses to hide their address; direct connections are opt-in and never expose a host's address without consent.
- Session model: lobbies with room codes (join by code, no account needed), platform invites (Steam, Game Center, Google Play Games), session tickets signed by the sessions service and checked by hosts and the relay, host selection by connection quality and device class with desktop preferred, host migration with periodic authoritative state handoff, and rejoin-in-progress with a configurable grace period (default 120 seconds) so iOS backgrounding or a network switch does not end a run.
- Protocol versioning: a protocol version and schema hash in every handshake; matchmaking partitions by compatibility version; sessions and relay accept the current and previous client version so staggered store releases keep cross-play working; a forced-update flow in the runtime UI.

Deliverables, replication and simulation models:
- Replication: component-level replication with ownership, interest management, priority and delta compression.
- Two models on one replication layer: authority with client prediction and reconciliation, and deterministic rollback for small player counts. Server-authoritative and host-authoritative are the same authority code running on a dedicated server or on one player's client.
- Determinism toolkit for rollback: deterministic physics step (Rapier `enhanced-determinism` per ADR 0003, or Jolt's deterministic mode if the backend changes), a deterministic math library and seeded RNG in the TS SDK, deterministic iteration-order guarantees, per-tick state checksums, a desync report that names the first diverging tick and component path, and a CI job that runs one input script on macOS ARM, Windows x86, Android ARM and iOS and compares checksums. Rollback cross-play between CPU architecture classes is enabled only when that job passes.
- Mobile constraints: configurable send rates (20 Hz default on mobile), a data budget of 20 MB per 25-minute run on cellular, battery-aware pacing of network ticks.

Deliverables, services (Rust, axum, Postgres; shipped as container images with a compose file so engine users can self-host):
- Sessions: lobbies, room codes, latency-aware matchmaking (minimal, extensible), session tickets, WebRTC signaling.
- Relay: stateless regional QUIC/UDP relay that forwards only for ticketed peers, rate-limited and horizontally scaled.
- Player: optional cross-store player identity (sign in with Steam, Apple or Google; Sign in with Apple is offered on iOS whenever any third-party sign-in is), account linking, cloud saves with the last five versions retained, conflict resolution by version vectors with last-write-wins and a choose-save UI, in-app account deletion, data export, and age-gate flags.
- Allocator: dedicated-server fleet allocation and lifecycle for games that choose dedicated servers. Reference deployment only; not a Driftwake launch requirement.
- Analytics: consent-gated gameplay event ingestion with events declared as schemas in the project document, aggregation and dashboards, offline buffering and sampling in the runtime, no personal data, and age-gate flags from the player service that switch it off. Separate from engine telemetry (Section 8).
- Dedicated server build target (headless Linux).
- Store-native cloud saves (Steam Cloud, iCloud, Google Play Saved Games) stay available behind the Phase 5 platform-services abstraction; cross-store sync requires the player service.

Deliverables, security and abuse:
- Encrypted transport everywhere (TLS 1.3 via QUIC, DTLS via WebRTC). Session tickets are short-lived and bound to player and session IDs. Room codes use six symbols from a 32-symbol alphabet with per-address and per-account join rate limits. Service front doors sit behind the provider's DDoS protection; relay nodes never reveal a game host's address.
- Network message parsers are fuzzed like asset parsers. Hosts and servers validate inputs (speed, cooldown, range); there is no kernel anti-cheat. Co-op has no competitive-integrity requirement, so cheating affects only the cheater's own party. Global leaderboards are out of scope at launch (open decision 17).
- Report and block for players, enforced in matchmaking.

Deliverables, editor and agent:
- Network simulation tools in the editor: latency, jitter, loss, bandwidth caps, and a determinism checksum view. Claude designs the panel per Section 4.2.
- Multi-client headless play-tests: the headless runner launches a host or server plus N clients under a latency profile and asserts on converged state.
- Analytics SDK: `analytics.track` in the TS SDK validates events against the project's declared schemas; the agent reads aggregates through the Appendix A tool.
- Agent skills: "make this entity replicated," "add a lobby," "diagnose desync," "add rejoin," with the tools in Appendix A.

Editor collaboration: real-time multi-user editing of a project (the CRDT document over this transport through a collaboration relay) is Engine 1.1 scope. Engine 1.0 ships single-user cloud sync (Phase 2) and journal-based session sharing.

Exit gate:
- Core Sample runs four-player co-op across desktop and mobile with 150 ms simulated latency and feels acceptable to a playtest group.
- Desync detection catches all seeded determinism bugs in a test suite.
- A client on cellular behind carrier NAT joins by room code through the relay, backgrounds the app for 60 seconds, returns, and is still in the run; the same holds for a Wi-Fi to cellular switch.
- A current-version client and a previous-version client complete a session together; a client two versions behind is refused with the forced-update flow.
- Sessions, relay and player services pass a load test at twice the Phase 8 projected launch concurrency in one region, and a self-hosting run from the documentation on a clean machine works without any Incant credentials.
- Determinism CI passes on all four reference device classes, or the plan records which architecture classes rollback cross-play is limited to.

### Phase 7: Driftwake vertical slice (Oct 2028 to Mar 2029, 6 months, overlaps 5 and 6)

Goals: prove the engine can make the game and set the production bar.

Deliverables:
- Game design document, written and maintained in the project, readable by the agent.
- One complete biome, three enemy types, one boss, the full core loop (run, die, upgrade, repeat), two playable characters, co-op for two.
- Co-op for two runs over the Phase 6 transport and relay with one desktop and one mobile client, including rejoin after a disconnect and a host leaving mid-run.
- Art pipeline defined: which assets are generated, which are procedural, which are hand-made.
- Mobile and desktop input schemes finalized.
- Friction log: every engine gap found, with a ticket.
- Analytics dashboards for run completion, death causes, upgrade pick rates and session length, feeding the designer's balance passes.

Exit gate:
- Vertical slice is fun in blind playtests (above 70% "would play more").
- Agent provenance share at or above 50% of gameplay scripts and scene edits.
- Engine friction log has no open blockers.

### Phase 8: Engine 1.0 and Driftwake production (Apr 2029 to Sep 2029, 6 months)

Goals: harden the engine, build the rest of the game.

Engine deliverables:
- Stabilize the document schema. Publish schema versioning guarantees.
- Performance pass on all targets. Memory budgets enforced.
- Documentation site, API reference generated from the schema registry and TS SDK, twenty tutorials, template gallery.
- Public beta with a thousand invited users. Telemetry opt-in. Weekly releases.
- Engine 1.0 release candidate.
- Live-service readiness: load test at twice projected launch concurrency in every launch region, runbooks, status page, alerting, a backup restore drill, a staffed on-call rotation, and cross-version compatibility (current and previous client) tested in CI.
- Release channels (stable, beta, nightly), a signed editor updater (Tauri updater with a human-held signing key; offline installers stay available), per-project runtime pinning so a shipped game bundles the runtime it was tested with, and a support policy: each 1.x stable receives fixes for twelve months.

Game deliverables:
- Four biomes, twelve enemy types, four bosses, four characters, meta-progression, four-player co-op, cross-play, cloud saves.
- Online: room codes, platform invites, optional account linking for cross-store saves, in-app account deletion and data export, age gate, report and block, forced-update flow.
- Localization into six languages on the Phase 1 string tables (drafted through the localization skill, human reviewed).
- Audio: full music and SFX pass.
- Accessibility: remappable controls, colorblind modes, text scaling, subtitles.

Exit gate:
- Content complete. Alpha milestone: all systems in, all content in rough form.
- Engine 1.0 RC has no P0 or P1 bugs open.

### Phase 9: Polish, certification, launch (Oct 2029 to Dec 2029, 3 months)

Deliverables:
- Beta: external playtest with five hundred players, balance and bug passes.
- Platform certification: Apple review, Google Play review, Steam review, age ratings (IARC, ESRB, PEGI).
- Launch marketing: trailer, store pages, press kit, engine case study showing agent-built content.
- Engine 1.0 public release the same week as the game.
- Online launch readiness: services live in all launch regions two weeks before launch, a load test fed by the five-hundred-player beta, an incident drill, and a launch-week on-call schedule.
- Post-launch plan: two content patches, engine 1.1 roadmap from beta feedback.

Exit gate (project done):
- Driftwake live on all three stores.
- Engine 1.0 downloadable with signed installers.
- Definition-of-done metrics in Section 1.3 all verified and published.

### 5.1 Timeline summary

The chart shows the original baseline; the table at the top of Section 5 records actual starts.

```
2026  N D | 2027  J F M A M J J A S O N D | 2028  J F M A M J J A S O N D | 2029  J F M A M J J A S O N D
P0    ███ |
P1        |   ████████████████            |
P2        |         ████████████████████  | ██
P3        |                 ████████████  | ██████████
P4        |                               | ████████████████████
P5        |                               |           ████████████████  | ██
P6        |                               |                 ██████████  | ██████
P7        |                               |                   ████████  | ██████
P8        |                               |                             |       ████████████
P9        |                               |                             |                   ██████
```

### 5.2 Acceleration options

- Doubling the Year 1 team cuts roughly six months, mostly from Phases 1 and 2, because they are parallelizable by subsystem.
- Dropping the browser mode from 1.0 saves three months of editor and WASM work.
- Dropping hardware ray tracing and screen-space GI saves two months of rendering work.
- Dropping the 2D pipeline from 1.0 saves about two months and removes the 2D platformer and top-down templates.
- Choosing a 2D game instead of 3D co-op would cut the game production by half but would fail to exercise most of the engine. Not recommended.

---

## 6. Workstream detail

Each workstream lists its human owner (the reviewer accountable for it), its first milestone, its key design choices, and its tests. Build execution for every workstream follows Section 4: Astra in Codex builds, and anything matching the visual rule in 4.2 is handed to Claude 5.5 over ACP.

### 6.1 Document model and command bus
- Owner: engine lead.
- First milestone: Phase 0 Spike 2.
- Choices: Loro for the CRDT, chosen after benchmarking Loro and Automerge in Phase 0 (ADR 0008). Canonical typed JSON on disk with a deterministic formatter and derived JSON Schemas; RON was not adopted. Schema in JSON Schema with Rust derive macros generating both the schema and the serializers.
- Tests: round-trip property tests, CRDT merge fuzzing, undo fuzzing, migration tests for every schema version.

### 6.2 Rendering
- Owner: rendering lead.
- First milestone: Phase 1 clustered forward+ on desktop and mobile tiers.
- Choices: three quality tiers (mobile, desktop, desktop-high). GPU-driven culling with meshlets on desktop, CPU culling on mobile. Virtual texturing deferred to post-1.0. Nanite-style virtualized geometry explicitly out of scope. The 2D renderer, 2D lights and video textures share the render graph.
- Tests: golden-image tests per tier per platform, frame-time budgets in CI, shader compile tests across all backends.

### 6.3 Scripting
- Owner: scripting engineer.
- First milestone: Phase 0 Spike 3.
- Choices: QuickJS for portability and sandboxing; V8 optional on desktop. TS compiled with a bundled SWC. Bindings generated from the schema registry so the SDK never drifts. Hot reload preserves entity state where types match.
- Tests: SDK conformance suite, sandbox escape tests, hot reload state preservation tests, performance tests with ten thousand scripted entities.

### 6.4 Editor
- Owner: editor lead.
- First milestone: Phase 0 Spike 1.
- Choices: React inside Tauri (ADR 0009). Schema-driven inspector so new components get UI for free. Panels are plugins so the community can add them.
- Tests: UI tests driven through the command bus, visual regression on panels, accessibility audit.

### 6.5 AI agent
- Owner: AI lead.
- First milestone: Phase 0 Spike 4.
- Choices: local agent loop, provider-agnostic, OpenAI first. Tools are generated from the command bus and schema registry so the agent can never call something the UI cannot. Approval modes. Skills as markdown playbooks. Evals as a release gate.
- Tests: the eval harness, prompt injection corpus, cost regression tests, latency budgets per turn.

### 6.6 Asset generation
- Owner: technical artist plus AI engineer.
- First milestone: Phase 3 texture generation.
- Choices: provider plugins for image, 3D, audio, animation. Generated assets go through the same cook pipeline as imported ones. Licensing metadata attached to every generated asset.
- Tests: output validity (manifold meshes, power-of-two textures, loudness-normalized audio), provider failover.

### 6.7 Platforms and build
- Owner: platform lead.
- First milestone: Phase 0 CI hello-world on six targets.
- Choices: generate native projects (Xcode, Gradle) rather than hide them, so advanced users can customize. Reproducible builds. Signing in CI with hardware-backed keys. Release channels (stable, beta, nightly) and a signed editor updater; projects pin an engine version range; Steam Deck is a verified-compatibility target; consoles are out of scope for 1.0.
- Tests: nightly device farm, size and startup budgets, store-submission dry runs quarterly, Steam Deck smoke run, updater and project-migration tests across the last three stable releases.

### 6.8 Networking
- Owner: core engine engineer with netcode experience for the crate; the backend and live-operations engineer for the services (Section 6.10).
- First milestone: transport, replication and relay working for two clients by Dec 2028 so Phase 7 co-op is not blocked; work may start in Jun 2028 alongside Phase 5.
- Choices: QUIC via quinn on native targets and WebRTC in browsers, custom UDP layer as fallback (open decision 11). Authority with prediction and deterministic rollback share one replication layer; server-authoritative and host-authoritative are the same authority code in different places. Driftwake's recommended launch topology is host-authoritative over the relay (open decision 10). Relay by default, direct connection opt-in. Deterministic physics step from Rapier's `enhanced-determinism` feature (ADR 0003), or Jolt's deterministic mode if the backend changes. Determinism toolkit in the TS SDK. Protocol versioning with a current-plus-previous compatibility window.
- Tests: network simulation suite, desync detection on seeded bugs, determinism checksum CI across the four reference device classes, nightly soak with four bot clients for eight hours, protocol fuzzing, cross-version compatibility tests, mobile network-transition and backgrounding tests on the device farm, relay and sessions load tests, host migration and rejoin tests.

### 6.9 Accounts and OpenAI integration
- Owner: backend engineer plus AI engineer.
- First milestone: Phase 0 Spike 5.
- Choices: see Section 3. Managed identity provider behind an abstraction. OAuth with PKCE and loopback redirect. OS keychain storage. Direct client-to-OpenAI calls.
- Tests: OAuth flow tests on all OSes, keychain tests, revoke tests, token refresh under clock skew, a test that asserts no credential ever appears in logs or telemetry.

### 6.10 Online services and live operations
- Owner: backend and live-operations engineer (Section 4.5); the director is accountable until that role is staffed.
- First milestone: sessions and relay deployed to one staging region before Phase 7 co-op testing begins (Dec 2028).
- Scope: the sessions, relay, player, analytics and allocator services from Phase 6, the accounts service from Section 3.2, and their production operation for Driftwake.

Hosting and deployment:
- One managed cloud provider (open decision 16). Containers behind a regional load balancer; Postgres managed by the provider with point-in-time recovery. Infrastructure as code lives in `services/deploy/`. CI deploys through short-lived OIDC credentials from the CI secrets store. No cloud credentials exist in the repo, the editor, the templates or the shipped game.
- Launch regions: North America east and west, Europe west, Asia-Pacific (Tokyo or Singapore). South America and Oceania are added if beta telemetry shows more than five percent of players with over 120 ms to the nearest relay. Sessions and player run active-active in two regions; relay runs in every region; the allocator is deployed only if Driftwake adopts dedicated servers.
- Build-time accounts never become runtime dependencies: deployment never uses the director's Codex or Claude sessions, and self-hosting the services needs no Incant engine account.

Self-hosting for engine users:
- Every service ships as a container image with a compose file, a configuration reference and a one-page "run it yourself" guide. The Phase 6 exit gate requires a clean-machine self-hosting run. Whether Incant also offers a hosted tier is a business decision tied to open decision 6; the engine never depends on it.

Operations:
- Observability: per-region RTT and loss distributions, relay egress, session durations, matchmaking time, desync incidents, error rates and saturation for every service. Logs carry no player content, addresses are truncated after 24 hours, and the Section 6.9 no-credential test extends to every service.
- Alerting and on-call: a human-owned on-call rotation from the Phase 8 beta onward, with agent-assisted triage. Runbooks for relay saturation, matchmaking backlog, Postgres failover, bad-release rollback and region loss. A public status page. Incident reviews within two working days.
- Releases: services deploy behind the client compatibility window (current and previous client version), with canary regions and automatic rollback on error-rate alarms.
- Backups and recovery: daily snapshots plus point-in-time recovery for the player database, cross-region copies, quarterly restore drills, and recovery objectives of one hour of data and four hours of downtime for the player service. Sessions and relay hold no durable state and recover by redeploy.

Cost model (estimates until beta telemetry replaces them; Section 11 carries the budget):

| Item | Estimate | Basis |
|---|---|---|
| Relay egress per four-player session-hour | 0.1 to 0.2 GB, one to two cents | 20 Hz send rate, 150 to 300 byte packets, host-authoritative fan-out |
| Dedicated server per session-hour, if adopted | three to six cents | Includes idle warm capacity; the main reason Driftwake defaults to host-authoritative |
| Fixed monthly cost at launch scale | 2,000 to 3,000 USD | Four relay regions, two service regions, managed Postgres, DDoS protection, monitoring |
| Launch-month variable cost at a 5,000 concurrent-session peak | about 10,000 USD | Assumes average load at a quarter of peak |

Compliance and player safety (legal review is human-owned):
- Player accounts are optional. Without one, a player gets local saves, store-native cloud saves, join-by-code multiplayer and platform invites. Linking Steam, Apple or Google sign-in enables cross-store saves and friends. Sign in with Apple is offered on iOS whenever any third-party sign-in is.
- Age gate at first launch. Players under the regional digital-consent age (13 in the United States, 13 to 16 across the EU) get no account linking, no communication features and no telemetry beyond crash reports. Counsel confirms the per-region feature set before beta.
- In-app account deletion and data export meet Apple App Store guideline 5.1.1(v), Google Play's account deletion requirement, GDPR and CCPA. Privacy policy, terms of service and the telemetry retention schedule are written before the Phase 8 public beta.
- No voice or free-text chat at launch (open decision 14); pings, emotes and preset phrases only. Report and block exist and are enforced in matchmaking. This keeps store review and moderation obligations small.
- Encryption export compliance declarations for iOS builds use the standard TLS exemption; the platform engineer records them per release.

Editor collaboration: real-time multi-user editing over a collaboration relay is Engine 1.1 scope. Phase 2 single-user sync and journal replay are the 1.0 collaboration story.

Tests: service load tests at twice projected launch concurrency, chaos tests (region loss, Postgres failover, relay node kill), backup restore drills, the no-credential-in-logs test, and a clean-machine self-hosting run in CI.

### 6.11 Navigation and gameplay AI
- Owner: core engine engineer.
- First milestone: Phase 1 navmesh on the Core Sample terrain with chasing enemies.
- Choices: recast-style tiled navmesh built from physics colliders and render meshes, A* with funnel string-pulling, steering with local avoidance (no full crowd simulation), off-mesh links, grid navigation for 2D. Tiles rebuild incrementally for runtime-assembled rooms. Behavior trees and utility AI are skills and TS helpers, not engine features.
- Tests: navmesh generation golden tests on fixture scenes, path optimality and determinism tests, tile rebuild budget (one tile under 2 ms on the mobile tier), steering tests with one hundred agents.

### 6.12 2D
- Owner: rendering lead, with the editor lead for tools.
- First milestone: Phase 1 sprite, tilemap and 2D physics sample on all four reference devices.
- Choices: sprites, atlases, nine-slice, sorting layers and tilemaps in `incant_2d` on the shared render graph; Rapier 2D behind the ADR 0003 physics boundary; pixel-perfect camera; autotile rules in the Phase 2 editor; 2D lights in Phase 4. Out of scope for 1.0: skeletal 2D animation, isometric and hex tilemaps.
- Tests: golden images for sprite sorting and pixel-perfect rendering, atlas packing property tests, tilemap collision tests, 2D physics determinism, the 2D platformer template exit gate.

### 6.13 Text and localization
- Owner: editor lead, with the game UI engineer.
- First milestone: Phase 1 shaped text with font fallback and a text input widget with IME.
- Choices: cosmic-text for shaping, bidi, line breaking and fallback; ICU4X for locale data, plural rules and formatting; string tables as typed document assets with an ICU MessageFormat subset; XLIFF for translator exchange; pseudo-localization as a CI check. The editor UI is English-only in 1.0 and localized in 1.1 on the same tables.
- Tests: shaping golden images per script (Latin, Arabic, Hebrew, CJK, Devanagari, emoji), IME composition tests on each desktop OS, on-screen keyboard tests on the device farm, string table round-trip and migration tests, a CI check that every user-facing string in Core Sample resolves in every project locale.

---

## 7. Driftwake production plan

### 7.1 Why this game

A 3D co-op action roguelite exercises everything: 3D rendering on mobile and desktop, physics-driven combat, skeletal animation, procedural level generation (a natural fit for the geometry graph and the agent), online multiplayer with cross-play, UI-heavy meta-progression, cloud saves, and store certification on three platforms. It also has a small content footprint per hour of play, which suits a team that is also building the engine.

### 7.2 Pillars

1. Short runs (fifteen to twenty-five minutes) with permanent meta-progression.
2. Combat built on physics: knockback, environmental hazards, destructible props.
3. Co-op is the default; solo is fully supported.
4. Mobile is a first-class platform with its own control scheme, not a port.

### 7.3 Production stages

| Stage | Window | Team | Output |
|---|---|---|---|
| Pre-production | Jun to Sep 2028 | 2 designers, 1 artist | GDD, paper prototypes, art bible, engine friction forecast |
| Vertical slice | Oct 2028 to Mar 2029 | 4 | One biome, core loop, two characters, co-op for two |
| Production | Apr to Sep 2029 | 8 | All content, all systems, alpha |
| Polish and beta | Oct to Nov 2029 | 8 plus QA | Beta, balance, certification |
| Launch | Dec 2029 | All | Live on three stores |
| Post-launch | Q1 2030 | 4 | Two patches, engine 1.1 inputs |

### 7.4 Dogfooding rules

- Every designer and artist on the game team uses the agent daily. Their friction logs are prioritized above engine team self-reported issues.
- Content with agent provenance is tracked per category. Target 60% overall, with the agent expected to dominate scripting and scene assembly and humans expected to dominate art direction and level design.
- Any engine fork or workaround in the game repo must be approved by the engine lead and fixed upstream within two sprints.

### 7.5 Content inventory

- 4 biomes, each with a procedural room-graph generator and twenty hand-authored room templates.
- 12 enemy archetypes, 4 bosses.
- 4 playable characters with distinct movesets.
- 60 run-time upgrades, 30 meta-upgrades.
- 40 minutes of music, roughly 600 sound effects.
- 6 languages.

---

## 8. Quality, testing, and release engineering

- **CI on every commit:** build all crates, unit tests, schema validation, TS SDK conformance.
- **Nightly:** full target builds, golden-image rendering tests, headless play-test suite on Core Sample and Driftwake, agent eval harness, device farm smoke tests, performance budgets, four-client network soak with bots under the latency profiles, determinism checksum comparison across reference device classes.
- **Weekly:** store-submission dry run on one platform in rotation, security dependency audit, crash report triage, service load test in one region, backup restore check.
- **Release trains:** engine beta releases every two weeks during Phase 8; game builds to internal QA daily.
- **Bug priority:** P0 (data loss, crash on launch, credential exposure), P1 (blocks a workflow), P2 (workaround exists), P3 (cosmetic). P0 and P1 block releases.
- **Telemetry:** opt-in only. Never includes project content, prompts, or credentials. Includes crash stacks, feature usage counts, agent turn success/failure (not content). Gameplay analytics for a shipped game is a separate, consent-gated service (Section 6.10) that the game developer enables; it never flows into engine telemetry.

---

## 9. Risks and mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Scope explosion; "engine" is infinite | High | Fatal | Exit gates, explicit out-of-scope list (Nanite, full sculpting, visual scripting, skeletal 2D, isometric and hex tilemaps, crowd simulation, consoles, editor localization), game drives priorities |
| Navigation, 2D and localization widen 1.0 scope | Medium | Medium | Each is bounded (no crowd simulation, no skeletal 2D, English-only editor); 2D is an acceleration-option drop; the Phase 1 and 2 gates measure them on Core Sample and the 2D template |
| OpenAI OAuth not available to third parties when we need it | Medium | Medium | API key path is always available; abstraction supports other providers |
| OpenAI changes pricing, models, or policy | High | Medium | Provider abstraction, model routing config updated with releases, evals across providers |
| Tauri plus native viewport is flaky on some platform | Medium | High | Phase 0 spike; fallback to egui-native editor |
| CRDT performance on large scenes | Medium | High | Phase 0 benchmark; fallback to operational transform or per-subtree CRDTs |
| Agent quality plateaus below usable | Medium | Fatal | Eval harness from Phase 0, skills to constrain the problem, approval modes, invest in tool ergonomics over prompts |
| Mobile performance | Medium | High | Mobile rendering engineer from day one, device farm, budgets in CI |
| Finding a senior graphics reviewer | Medium | Medium | Part-time or contract in Year 1; contribute to Bevy to build reputation |
| Bevy breaking changes | High | Medium | Pin versions, upgrade on a schedule, upstream fixes |
| Store rejection for AI-generated content policies | Low | Medium | Track platform policies, attach provenance and licensing to all assets |
| Prompt injection via shared projects or templates | Medium | High | Data labeling, sandboxed scripts, no shell access, red-team pass before each release |
| Team burnout building engine and game simultaneously | High | High | Separate teams, honest scheduling, phase gates that allow slips without death marches |
| ACP adapter or Claude account limits stall visual work | Medium | Medium | Handoff packets are self-contained so the director can run Claude Code manually; batch non-urgent visual tasks per sprint |
| Codex outages or task limits stall the primary builder | Medium | Medium | Codex CLI fallback on the director's machine; work queue lives in the repo so any agent session can resume it |
| Two agents drift on conventions and style | High | Medium | Both instruction files generated from one conventions source; CI lint; Astra integrates every handoff and owns the final PR |
| Routing ambiguity causes rework | Medium | Low | Default to the split pattern; director tie-breaks weekly |
| Director becomes the bottleneck for approvals | High | Medium | Approval modes per risk level; agents merge completed PRs after review and passing checks; gates and routing disputes need the director |
| Cross-architecture determinism (x86 versus ARM) breaks rollback cross-play | High | Medium | Driftwake defaults to host-authoritative with prediction; rollback cross-play gated on determinism CI across the four reference device classes; deterministic math and RNG in the TS SDK |
| Mobile networks (carrier NAT, IPv6-only, backgrounding, Wi-Fi to cellular switches) drop sessions | High | High | Relay by default on cellular, dual-stack services, QUIC connection migration, rejoin grace period, network-transition tests on the device farm |
| Store review lag leaves cross-play clients on different versions | High | Medium | Protocol and schema versioning, current-plus-previous compatibility window, matchmaking by compatibility version, forced-update flow |
| Host leaves or lags in host-authoritative co-op | High | Medium | Host selection by device class and connection quality, host migration with periodic state handoff, run state checkpointed at room transitions |
| Live-service outage or cost spike at launch | Medium | High | Relay topology keeps variable cost near egress price; load test at twice projected peak; runbooks and on-call before beta; dedicated servers optional |
| Online features trigger legal and store obligations late (age gates, account deletion, moderation) | High | Medium | Optional accounts, no chat at launch, age gate and in-app deletion designed in Phase 6, counsel review before beta (human-owned) |
| Services become a single point of failure for a game that should work offline | Medium | High | Offline degradation is a tested requirement; single-player and local saves never touch services; self-hosting keeps the services alive if Incant stops hosting them |

---

## 10. Metrics and gates dashboard

Tracked continuously from Phase 1 and reviewed at every phase review.

- Agent eval pass rate per model (target 80% core, 70% content).
- Time-to-playable-prototype for a new user (target under fifteen minutes).
- Agent provenance share in Driftwake (target 60%).
- Frame time per tier per reference device (targets: 16.6 ms desktop, 16.6 ms mobile high, 33 ms mobile low).
- Cold start per platform.
- Build size per platform (targets: under 150 MB mobile base, under 500 MB desktop base).
- Crash-free sessions (target 99.5% in beta).
- P0/P1 open count (target zero at every release).
- Agent cost per successful task (tracked, no hard target; surfaced to users).
- Network: p95 round-trip time to the nearest relay per region (target under 100 ms for 90% of players), packet loss, and data per run on mobile (target under 20 MB per 25-minute run).
- Desync incidents per thousand runs (target zero confirmed), rejoin success after a disconnect (target 95%), host migration success rate, matchmaking time to a full lobby (tracked).
- Service availability per month (target 99.9% for sessions and player, 99.95% for relay), relay cost per session-hour, fixed monthly service cost.

---

## 11. Budget estimate

Rough order of magnitude, for planning only. Reflects the agent-first execution model in Section 4.

| Category | Year 1 | Year 2 | Year 3 | Total |
|---|---|---|---|---|
| Human salaries and contracts (1.5 / 6 / 9 heads) | 0.4M | 1.3M | 2.0M | 3.7M |
| Build agents: Codex usage for Astra, Claude subscription for handoffs | 0.15M | 0.25M | 0.3M | 0.7M |
| Model usage for the in-engine agent eval harness | 0.1M | 0.3M | 0.3M | 0.7M |
| Infrastructure, CI, device farm | 0.2M | 0.4M | 0.5M | 1.1M |
| Live game services: relay, sessions, player, DDoS protection, monitoring, load tests | 0 | 0.05M | 0.2M | 0.25M |
| Licenses, tools, certificates, stores | 0.1M | 0.1M | 0.2M | 0.4M |
| Contract art and audio for Driftwake | 0 | 0.3M | 0.6M | 0.9M |
| Marketing and launch | 0 | 0.1M | 0.8M | 0.9M |
| Contingency (15%) | 0.15M | 0.4M | 0.75M | 1.3M |
| **Total** | **1.1M** | **3.2M** | **5.65M** | **9.95M** |

Revision 3 added the live game services line and the backend and live-operations role, raising the total from 9.35M to 9.95M. Post-launch service costs fall outside the 36-month horizon; the run-rate in Section 6.10 (fixed costs plus about a cent per relayed session-hour) is re-estimated from beta telemetry at the Phase 8 gate and funded from game revenue.

Figures in USD. The build-agent line is the least certain; it depends on Codex and Claude pricing and limits over three years, and should be re-estimated at every phase gate from actual usage. For comparison, a fully human-staffed version of this plan (15, 29, and 36 heads) was estimated at roughly 23M.

---

## 12. Open decisions

Decisions 1 to 9 were to be resolved in Phase 0; `docs/gates/phase0-review.md` records their current disposition. Decisions 10 to 17 concern networking and online services and carry their own deadlines.

1. Solid vs React for the editor UI. Resolved: React (ADR 0009).
2. Loro vs Automerge for the CRDT. Resolved: Loro, both benchmarked (ADR 0008).
3. Managed identity provider vs self-hosted accounts. Open. ADR 0012 separates optional cloud metadata from local provider login; the identity operating choice needs director review before any service is implemented.
4. RON vs JSON for the on-disk format. Resolved: canonical typed JSON with derived JSON Schemas (ADR 0008).
5. Whether browser mode is in 1.0 or 1.1. Open. The definition of done includes it in 1.0; the WASM probe is not the delivered browser editor.
6. Whether to ship the editor itself under an open-source license with a paid cloud tier, or closed with a free tier. Open. This affects hiring, community templates, Bevy upstreaming strategy, and whether Incant hosts services for engine users (decision 16).
7. Final name for the game, chosen from Appendix D after a trademark, domain, and store search. Open; Driftwake remains a placeholder. The engine name is decided: Incant. Still to do for it: trademark search (USPTO, EUIPO), domain registration, and reserving `incant_*` on crates.io and the `@incant` scope on npm.
8. Codex execution mode for Astra: cloud tasks only, CLI only, or both. Open in principle; Phase 0 and Phase 1 work has used the local CLI with hosted CI, and no cloud Codex deployment is claimed.
9. Approval mode per risk level for agent PRs, so the director is not the bottleneck. Largely resolved by the director decisions of 2026-10-08 and 2026-10-09: reversible implementation proceeds autonomously and agents merge reviewed, passing PRs; phase approval, signing, purchases, legal and staffing stay with the director.
10. Driftwake session topology. Recommended: host-authoritative sessions with client prediction over the relay, desktop host preferred, host migration. Alternative: dedicated servers for every session. Decide by Jun 2028, before Phase 7 pre-production, because it shapes level streaming, enemy ownership and the cost model.
11. Transport base. Recommended: QUIC via quinn. Alternative: custom UDP reliability layer over a Noise handshake. Decide at Phase 6 start.
12. Online services. Recommended: the engine's own Rust services, self-hostable, dogfooded by Driftwake. Alternatives: Epic Online Services, Nakama or PlayFab behind the platform-services abstraction. Decide at Phase 6 start.
13. Driftwake player accounts. Recommended: optional linked accounts for cross-store saves and friends. Alternatives: no accounts (store-native saves only, no cross-store sync) or mandatory accounts. Decide with decision 10.
14. In-game communication. Recommended: pings, emotes and preset phrases only at launch. Alternatives: text chat or voice chat, each adding moderation, legal and store-review obligations. Decide by Phase 7.
15. Rollback cross-play scope. Recommended: rollback only within one CPU architecture class unless the determinism CI passes across all four reference device classes; Driftwake does not depend on rollback. Decide at the Phase 6 gate.
16. Hosting provider, launch regions, and whether Incant offers a hosted services tier for engine users (ties to decision 6). Decide before the Phase 8 beta.
17. Global leaderboards. Recommended: none at launch. Alternative: server-validated leaderboards, which require run verification. Decide by Phase 7.

---

## Appendix A: Agent tool surface v1

All tools are generated from the command bus and schema registry. Each has a JSON Schema for inputs and outputs. Destructive tools are marked and subject to approval modes.

Document:
- `doc.query(path, depth, filter)` returns a subtree.
- `doc.search(text, kinds)` finds entities, assets, scripts by name or content.
- `doc.schema(type)` returns the schema for a component or asset type.
- `doc.patch(ops)` applies a transaction of create, update, delete, reparent ops. Returns validation results. Destructive.
- `doc.history(limit)` returns recent transactions with provenance.
- `doc.revert(transaction_id)` undoes a transaction. Destructive.

Assets:
- `asset.list(filter)`, `asset.inspect(id)`, `asset.import(path)`, `asset.generate(kind, prompt, options)`.

Scripts:
- `script.read(id)`, `script.write(id, source)` (destructive), `script.typecheck(id)`, `script.list_errors()`.

Perception:
- `view.screenshot(camera, size)` returns an image.
- `view.frame_debug()` returns draw calls, triangle counts, overdraw.
- `console.read(since, level)`.
- `profiler.read(frames)`.

Play-test:
- `play.run(seconds, inputs_script, cameras)` runs headless, returns screenshots at intervals, logs, final state snapshot, and assertion results.
- `play.state_query(path)` during a live play session.

Network:
- `net.simulate(profile)` applies a latency, jitter, loss and bandwidth profile to play-in-editor and headless runs.
- `net.status()` returns connection mode (direct or relay), RTT, loss, send rate, protocol version and session info.
- `net.desync_report(session)` returns the first diverging tick, the component paths that diverged, and per-client checksums.
- `play.run_multi(clients, latency_profile, inputs_scripts, cameras)` runs a host or server plus N headless clients and returns per-client screenshots, logs, converged-state assertions and the desync report. Long-running.

Analytics:
- `analytics.query(metric, range, filters)` returns aggregated gameplay analytics (counts, rates, distributions) for the project's declared events; never raw per-player records.

Skills and memory:
- `skill.list()`, `skill.read(name)`, `skill.run(name, params)`.
- `memory.read()`, `memory.write(section, text)` for the project memory document.

Meta:
- `budget.remaining()` returns tokens and cost left for the session.
- `user.ask(question, options)` pauses for user input.

## Appendix B: Document schema sketch

```
Project
  id: ulid
  schema_version: u32
  engine_version: semver range
  settings: ProjectSettings
  scenes: [SceneRef]
  assets: [AssetRef]
  scripts: [ScriptRef]
  skills: [SkillRef]
  memory: MemoryDoc
  string_tables: [StringTableRef]
  analytics_events: [EventSchema]

Scene
  id: ulid
  name: string
  entities: [Entity]

Entity
  id: ulid
  name: string
  parent: ulid?
  prefab: { source: ulid, overrides: [Override] }?
  components: { TypeName: ComponentValue }
  provenance: { origin: user|agent|script|import, actor, transaction }

Component (examples)
  Transform { translation, rotation, scale }
  MeshRenderer { mesh: AssetRef, materials: [AssetRef], cast_shadows, lod_group }
  Material { shader: AssetRef | graph: ShaderGraph, params: {...} }
  RigidBody { kind, mass, collider: ColliderShape, layers }
  Script { source: ScriptRef, props: {...} }
  AnimationGraph { graph: AnimGraph, params }
  Replicated { owner, mode: authority|rollback, fields, relevance, priority }
  Predicted { fields, reconcile: snap|smooth }
  UiRoot { layout: UiTree }
  GeometryGraph { nodes, edges, outputs }
  Sprite { atlas: AssetRef, region, flip, sorting_layer, order }
  Tilemap { tileset: AssetRef, layers, cell_size, collision }
  NavMeshSurface { agent_radius, agent_height, tile_size, layers }
  NavAgent { radius, speed, avoidance }
  Text { content: LocalizedString | string, font: AssetRef, size, wrap }
```

## Appendix C: Glossary

- **Command bus:** the single pathway through which every edit flows.
- **Provenance:** metadata recording who or what made an edit.
- **CRDT:** conflict-free replicated data type, a data structure that merges concurrent edits automatically.
- **Headless runner:** the engine running without a window, used for tests and agent play-tests.
- **Skill:** a reusable playbook the agent follows to perform a common multi-step task.
- **Eval harness:** the automated suite that measures agent success on scripted tasks.
- **Tier:** a rendering quality level (mobile, desktop, desktop-high).
- **BYO model:** bring-your-own model, where the user supplies their own provider account.
- **Astra:** the agent running in OpenAI Codex that builds everything non-visual and integrates all work.
- **ACP:** Agent Client Protocol, the open protocol for talking to coding agents such as Claude Code from another tool. Used to hand visual work to Claude 5.5.
- **Handoff packet:** a self-contained folder describing one visual task for Claude, plus the result it returns.
- **Director:** the human (Andrey) who sets priorities, approves gates and breaks routing ties. Agents merge completed PRs under the director’s standing authorization.
- **Relay:** a server that forwards game traffic between players who cannot connect directly, hiding their addresses from each other.
- **Host-authoritative:** one player's client runs the authoritative simulation and the others predict and reconcile. Server-authoritative is the same code running on a dedicated server.
- **Rollback:** every client simulates deterministically from inputs and re-simulates when late inputs arrive. Requires identical results on every client.
- **Prediction and reconciliation:** a client applies its own inputs immediately and corrects to the authority's state when it arrives.
- **Interest management:** sending each client only the replicated state relevant to it.
- **Session ticket:** a short-lived signed token from the sessions service that authorizes one player to join one session through the relay.
- **Compatibility version:** the protocol version and schema hash pair that clients must share to play together.
- **Navmesh:** a walkable-surface mesh generated from level geometry that agents find paths over.
- **String table:** a typed document asset mapping string keys to per-locale text with plural and select rules.
- **Pseudo-localization:** replacing every string with a lengthened, accented variant to find truncation, hard-coded text and shaping bugs before real translation.

## Appendix D: Name shortlist

Registry columns show whether the exact single word is free on crates.io and npm as of 2026-10-08. A taken single word is not disqualifying, since crates and packages will be prefixed (`incant_core`, `@incant/runtime`). Before committing to any name, run a trademark search (USPTO, EUIPO), check domains, and search Steam, the App Store, and Google Play.

### Engine

| Name | Why it fits | crates.io | npm | Notes |
|---|---|---|---|---|
| **Incant** (chosen) | To speak something into being. Verb-like, short, "incant a world." | taken | taken | `incantor` is free on both and works as a studio or CLI name |
| **Tulpa** | A being made real by sustained thought. Unusual and memorable. | taken | taken | Tibetan Buddhist origin; check cultural sensitivity |
| **Thaum** | A unit of magic. Short and clean. | free | free | `thaumic` also free on both |
| **Nacre** | Mother-of-pearl, built up in layers. Elegant, modeling feel. | taken | taken | |
| **Demiurge** | The craftsman who shapes the world. Grand. | taken | taken | Long for a CLI name |
| **Golem** | Brought to life by written words. Strong metaphor for the product. | taken | taken | Crowded name space (crypto network) |
| **Lathe** | Shapes raw stock into form. Tool-maker feel. | taken | taken | Common word, harder to trademark |
| **Marrow** | The core of a thing. Gritty engine feel. | taken | taken | |
| **Sayso** | "Say so and it is so." Playful, consumer-friendly. | free | taken | |
| **Wyrd** | Fate as something woven. Short, Norse. | taken | taken | |

Avoid: Forge (Minecraft Forge, Autodesk Forge), Lumen (Unreal), Cinder (creative coding framework), Quill (editor library), Loom (video), Mythos and Fable (Anthropic model names).

### Launch game (alternatives to Driftwake)

| Name | Feel | crates.io | npm |
|---|---|---|---|
| **Hollowtide** | Drowned ruins, cyclical runs | free | free |
| **Saltcrown** | Sea kingdom, bosses as crowns | free | free |
| **Ebbfall** | Each run is a tide going out | free | free |
| **Keelbound** | Co-op crew on one hull | unchecked | unchecked |
| **Wreckling** | Scavenger tone, lighter | unchecked | unchecked |
