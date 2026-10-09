# AI-Native Game Engine: Start-to-Finish Plan

Engine name: **Incant** (decided 2026-10-08; the shortlist it was chosen from is in Appendix D).
Working name for the launch game: **Driftwake** (placeholder).

Plan author: Claude, for Andrey Yurlov. Date: 2026-10-08. Revision 2 (same day): renamed engine, added agent-first execution model and Claude ACP visual handoff.
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
| Physics | Jolt via FFI bindings | Production-proven, deterministic mode for rollback netcode | Rapier (pure Rust, simpler, less performant) |
| Audio | Kira plus a thin mixer layer | Pure Rust, good for games | FMOD via FFI for studios who require it |
| Game UI | Taffy (flexbox layout) plus custom widget tree | Familiar web-style layout, agent writes it easily | None needed |
| Gameplay scripting | TypeScript on a sandboxed QuickJS runtime, hot reload | LLMs write TS best; sandboxable; users already know it | V8 via deno_core on desktop for perf-heavy projects |
| Native extensions | Rust plugin ABI with stable C interface | Escape hatch for perf-critical code | None |
| Project document | RON-like typed text format, schema-validated, CRDT-backed (Automerge or Loro) | Diffable, mergeable, agent-readable | JSON with JSON Schema if RON tooling is weak |
| Editor shell | Web UI (Solid or React, TypeScript) inside Tauri | Fast UI iteration, same UI reused in browser mode | egui native UI |
| Viewport | Native wgpu surface composited into the Tauri window | Zero-copy rendering | Texture streaming over shared memory |
| Agent runtime | Local agent loop in the editor process, provider-agnostic | Keeps user data local, supports BYO keys | Hosted agent for teams who opt in |
| Accounts service | Small Rust (axum) service plus Postgres | Login, project sync, licensing | Managed auth provider (see 3.2) |
| Asset generation | Pluggable providers (image, 3D, audio, animation) | Models change every six months | None |
| CI/CD | GitHub Actions, self-hosted macOS and Windows runners, device farm | Required for six targets | Buildkite |

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
    incant_physics     Jolt bindings, colliders, character controller
    incant_audio       Mixer, spatialization, buses
    incant_ui          Game UI layout and widgets
    incant_script      QuickJS host, TS bindings, hot reload, sandbox
    incant_net         Transport, replication, rollback, server-authoritative
    incant_assets      Import, cook, stream, hash, cache
    incant_agent       Agent loop, tool registry, providers, skills
    incant_headless    Headless runner for play-tests and CI
    incant_export      Build pipeline for all targets
    incant_runtime     The shipped game runtime (no editor)
  editor/
    app/              Tauri shell
    ui/               Web UI (panels, inspectors, chat, history)
    bridge/           IPC between UI and engine process
  services/
    accounts/         Login, provider connections metadata, licensing, sync
    telemetry/        Opt-in crash and usage reporting
  sdk/
    ts/               @incant/runtime TypeScript types and helpers
    templates/        Starter projects
    skills/           Agent skills and recipes
  games/
    driftwake/        The launch game
  docs/
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
| **Andrey** (director) | | | Priorities, phase gate approvals, PR merges, routing tie-breaks, and anything requiring a human: store and developer accounts, signing keys, payments, legal, age ratings. |

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
| Docs and marketing | Reference and tutorial text, API docs | Diagrams, screenshots, site design, video |

If routing is ambiguous, Astra asks the director in the task thread and defaults to the split pattern in the meantime.

### 4.3 Handoff protocol (Astra to Claude over ACP)

1. **Packet.** Astra writes a self-contained handoff packet to `handoffs/<id>/brief.md`: goal, acceptance criteria, relevant file paths, build and run commands, the headless-runner command that captures screenshots, reference images, constraints (performance budgets, target platforms, tiers), and exactly what to return.
2. **Session.** Astra runs `tools/handoff run <id>`. The tool opens an ACP session to Claude Code through the Claude Code ACP adapter. The session inherits the director's logged-in Claude account. No API key or token is written anywhere.
3. **Work.** Claude works in a git worktree on branch `handoff/<id>`, runs the visual regression suite, and writes `handoffs/<id>/result.md` with before and after screenshots, rationale, and open questions.
4. **Integration.** Astra reviews the result, merges the worktree, runs full CI, and opens the PR. Every commit carries a trailer `Built-by: astra` or `Built-by: claude` so provenance covers engine development as well as game content.
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
- Human-only: Apple, Google, and Steam developer accounts, signing keys, payment setup, legal filings, age rating submissions.

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
| **Total** | **1.5** | **5.5** | **8** | |

### 4.6 Operating rules

- Every PR is agent-authored and human-merged. The director approves every phase gate.
- Weekly the director reviews the handoff log, eval results, and friction logs, and resolves any routing disputes.
- Agents never self-approve a routing exception. Ambiguity defaults to the split pattern.
- Both agents dogfood the product. Astra and Claude build Driftwake content through the in-engine agent wherever possible, which is what the provenance metric measures.
- Agent-first execution runs around the clock and parallelizes by crate, so the 36-month schedule in Section 5 is conservative. Phases 1 through 4 may compress. Phases 5 and 9 will not, because they are bound by physical device testing and store review times.

---

## 5. Phases and timeline

Start: November 2026. Durations overlap deliberately. Each phase lists goals, deliverables, and the exit gate.

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
- Physics: Jolt integration, colliders, rigid bodies, character controller, raycasts, triggers, deterministic step mode.
- Audio: mixer with buses, 3D spatialization, streaming for music.
- Animation: skeletal playback, blend trees, state machines, two-bone IK, root motion.
- Game UI: flexbox layout, text rendering with font atlases, input focus, gamepad navigation.
- Scripting: full TS SDK covering entities, components, input, physics queries, audio, UI, timers, coroutines. Type definitions generated from the schema registry.
- Headless runner: run a project for N seconds, capture frames and logs, exit with a status.
- Input: keyboard, mouse, gamepad, touch with gesture recognition.
- Save system: serialize game state to a versioned format.

Exit gate:
- A "Core Sample" test game (third-person character on a terrain with enemies, pickups, UI, music) runs at 60 fps on an M1 MacBook Air, a mid-range Windows laptop, an iPhone 13, and a Pixel 6, written entirely in TypeScript against the SDK with no editor.
- Headless runner used in CI to play Core Sample and assert on game state.

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
- Browser mode: the editor web UI running against the WASM engine, read-only viewport first, editing second.
- Engine account sign-in and project cloud sync (single user, last-write-wins at file level, CRDT merge at document level).

Exit gate:
- Core Sample can be rebuilt from an empty project using only the editor in under four hours by someone who has never seen the code.
- Every editor action is a command on the bus (verified by a test that disables direct document mutation and runs the UI test suite).
- Undo/redo passes a fuzz test of ten thousand random command sequences with document equality checks.

### Phase 3: AI layer (Sep 2027 to Jun 2028, 10 months, overlaps Phase 2)

Goals: the agent is a genuine second user of the editor.

Deliverables:
- Provider layer with OpenAI OAuth and API key, keychain storage, cost tracking, model routing. Anthropic and local OpenAI-compatible endpoints as second providers.
- Agent loop: planning, tool calls, streaming to chat UI, interruption, approval modes (auto, ask for destructive, ask always).
- Tool surface v1 (see Appendix A): document query and patch, schema lookup, asset listing, screenshot, console, profiler, play-test, script read/write, validation, search.
- Context management: project summary, recent history, relevant schemas, and open panels are assembled per turn. Long projects use a maintained project memory file.
- Skills system: a skill is a markdown playbook plus optional TS helpers. Ship fifteen skills: character controllers (third-person, first-person, 2D platformer, top-down), camera rigs, inventory, dialogue, health and damage, spawners, checkpoints, mobile touch controls, settings menu, main menu, save/load, simple AI (patrol, chase), day/night cycle.
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
- Asset generation v2: image-to-3D with automatic cleanup to engine-ready meshes, texture set generation (albedo, normal, roughness), animation generation from text for humanoid rigs.

Exit gate:
- A technical artist builds a complete environment (terrain, twenty props, five materials, lighting, VFX) in two days using only the engine, with at least half the props from the geometry graph or generation.
- Agent eval extended with fifty content tasks, pass rate at or above 70%.

### Phase 5: Platforms and export (Jun 2028 to Jan 2029, 8 months)

Goals: one-click builds that pass store review.

Deliverables:
- Export pipeline for Windows (MSIX and portable), macOS (notarized app), Linux (AppImage), iOS (Xcode project generation plus direct archive), Android (Gradle project plus direct APK/AAB), web (WASM plus a hosting bundle).
- Mobile: touch input layer, safe-area handling, orientation, app lifecycle, battery-aware frame pacing, thermal throttling response, on-device texture compression tiers (ASTC, ETC2).
- Desktop: Steam integration (achievements, cloud saves, overlay, input), window management, display modes.
- Platform services abstraction: achievements, leaderboards, cloud save, IAP (hooks only, not a store).
- Crash reporting and symbolication for all targets.
- Device farm in CI: ten physical devices across iOS and Android, nightly smoke tests.
- Build size and startup time budgets enforced in CI.

Exit gate:
- Core Sample ships to TestFlight, Google Play internal testing, and a private Steam app, from CI, with no manual steps beyond pressing a button.
- Cold start under three seconds on an iPhone 13 and under five seconds on a Pixel 6.

### Phase 6: Networking (Sep 2028 to Mar 2029, 7 months)

Goals: multiplayer in the box.

Deliverables:
- Transport: UDP with reliability layer, WebRTC data channels for browser, relay service for NAT traversal.
- Replication: component-level replication with ownership, interest management, delta compression.
- Two models: server-authoritative with client prediction and reconciliation, and deterministic rollback for small-player-count action games.
- Lobby and matchmaking service (minimal, extensible), with Steam and platform friend lists.
- Dedicated server build target (headless Linux).
- Network simulation tools in the editor: latency, jitter, loss.
- Agent skills for networking: "make this entity replicated," "add a lobby," "diagnose desync."

Exit gate:
- Core Sample runs four-player co-op across desktop and mobile with 150 ms simulated latency and feels acceptable to a playtest group.
- Desync detection catches all seeded determinism bugs in a test suite.

### Phase 7: Driftwake vertical slice (Oct 2028 to Mar 2029, 6 months, overlaps 5 and 6)

Goals: prove the engine can make the game and set the production bar.

Deliverables:
- Game design document, written and maintained in the project, readable by the agent.
- One complete biome, three enemy types, one boss, the full core loop (run, die, upgrade, repeat), two playable characters, co-op for two.
- Art pipeline defined: which assets are generated, which are procedural, which are hand-made.
- Mobile and desktop input schemes finalized.
- Friction log: every engine gap found, with a ticket.

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

Game deliverables:
- Four biomes, twelve enemy types, four bosses, four characters, meta-progression, four-player co-op, cross-play, cloud saves.
- Localization into six languages (agent-assisted, human reviewed).
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
- Post-launch plan: two content patches, engine 1.1 roadmap from beta feedback.

Exit gate (project done):
- Driftwake live on all three stores.
- Engine 1.0 downloadable with signed installers.
- Definition-of-done metrics in Section 1.3 all verified and published.

### 5.1 Timeline summary

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
- Choosing a 2D game instead of 3D co-op would cut the game production by half but would fail to exercise most of the engine. Not recommended.

---

## 6. Workstream detail

Each workstream lists its human owner (the reviewer accountable for it), its first milestone, its key design choices, and its tests. Build execution for every workstream follows Section 4: Astra in Codex builds, and anything matching the visual rule in 4.2 is handed to Claude 5.5 over ACP.

### 6.1 Document model and command bus
- Owner: engine lead.
- First milestone: Phase 0 Spike 2.
- Choices: Loro or Automerge for CRDT (benchmark both in Phase 0 on a ten-thousand-entity scene). RON-like text format with a canonical formatter. Schema in JSON Schema with Rust derive macros generating both the schema and the serializers.
- Tests: round-trip property tests, CRDT merge fuzzing, undo fuzzing, migration tests for every schema version.

### 6.2 Rendering
- Owner: rendering lead.
- First milestone: Phase 1 clustered forward+ on desktop and mobile tiers.
- Choices: three quality tiers (mobile, desktop, desktop-high). GPU-driven culling with meshlets on desktop, CPU culling on mobile. Virtual texturing deferred to post-1.0. Nanite-style virtualized geometry explicitly out of scope.
- Tests: golden-image tests per tier per platform, frame-time budgets in CI, shader compile tests across all backends.

### 6.3 Scripting
- Owner: scripting engineer.
- First milestone: Phase 0 Spike 3.
- Choices: QuickJS for portability and sandboxing; V8 optional on desktop. TS compiled with a bundled SWC. Bindings generated from the schema registry so the SDK never drifts. Hot reload preserves entity state where types match.
- Tests: SDK conformance suite, sandbox escape tests, hot reload state preservation tests, performance tests with ten thousand scripted entities.

### 6.4 Editor
- Owner: editor lead.
- First milestone: Phase 0 Spike 1.
- Choices: Solid for UI performance, or React for hiring ease. Decide in Phase 0. Schema-driven inspector so new components get UI for free. Panels are plugins so the community can add them.
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
- Choices: generate native projects (Xcode, Gradle) rather than hide them, so advanced users can customize. Reproducible builds. Signing in CI with hardware-backed keys.
- Tests: nightly device farm, size and startup budgets, store-submission dry runs quarterly.

### 6.8 Networking
- Owner: core engine engineer with netcode experience.
- First milestone: Phase 6 transport.
- Choices: both server-authoritative and rollback, because the launch game needs rollback-quality feel but mobile battery favors server-authoritative. Deterministic physics mode from Jolt.
- Tests: network simulation suite, desync detection, soak tests with bots.

### 6.9 Accounts and OpenAI integration
- Owner: backend engineer plus AI engineer.
- First milestone: Phase 0 Spike 5.
- Choices: see Section 3. Managed identity provider behind an abstraction. OAuth with PKCE and loopback redirect. OS keychain storage. Direct client-to-OpenAI calls.
- Tests: OAuth flow tests on all OSes, keychain tests, revoke tests, token refresh under clock skew, a test that asserts no credential ever appears in logs or telemetry.

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
- **Nightly:** full target builds, golden-image rendering tests, headless play-test suite on Core Sample and Driftwake, agent eval harness, device farm smoke tests, performance budgets.
- **Weekly:** store-submission dry run on one platform in rotation, security dependency audit, crash report triage.
- **Release trains:** engine beta releases every two weeks during Phase 8; game builds to internal QA daily.
- **Bug priority:** P0 (data loss, crash on launch, credential exposure), P1 (blocks a workflow), P2 (workaround exists), P3 (cosmetic). P0 and P1 block releases.
- **Telemetry:** opt-in only. Never includes project content, prompts, or credentials. Includes crash stacks, feature usage counts, agent turn success/failure (not content).

---

## 9. Risks and mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Scope explosion; "engine" is infinite | High | Fatal | Exit gates, explicit out-of-scope list (Nanite, full sculpting, visual scripting), game drives priorities |
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
| Director becomes the bottleneck for approvals | High | Medium | Approval modes per risk level; only gates, merges, and routing disputes need the director |

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

---

## 11. Budget estimate

Rough order of magnitude, for planning only. Reflects the agent-first execution model in Section 4.

| Category | Year 1 | Year 2 | Year 3 | Total |
|---|---|---|---|---|
| Human salaries and contracts (1.5 / 5.5 / 8 heads) | 0.4M | 1.2M | 1.8M | 3.4M |
| Build agents: Codex usage for Astra, Claude subscription for handoffs | 0.15M | 0.25M | 0.3M | 0.7M |
| Model usage for the in-engine agent eval harness | 0.1M | 0.3M | 0.3M | 0.7M |
| Infrastructure, CI, device farm | 0.2M | 0.4M | 0.5M | 1.1M |
| Licenses, tools, certificates, stores | 0.1M | 0.1M | 0.2M | 0.4M |
| Contract art and audio for Driftwake | 0 | 0.3M | 0.6M | 0.9M |
| Marketing and launch | 0 | 0.1M | 0.8M | 0.9M |
| Contingency (15%) | 0.15M | 0.4M | 0.7M | 1.25M |
| **Total** | **1.1M** | **3.05M** | **5.2M** | **9.35M** |

Figures in USD. The build-agent line is the least certain; it depends on Codex and Claude pricing and limits over three years, and should be re-estimated at every phase gate from actual usage. For comparison, a fully human-staffed version of this plan (15, 29, and 36 heads) was estimated at roughly 23M.

---

## 12. Open decisions (resolve in Phase 0)

1. Solid vs React for the editor UI.
2. Loro vs Automerge for the CRDT.
3. Managed identity provider vs self-hosted accounts.
4. RON vs JSON for the on-disk format.
5. Whether browser mode is in 1.0 or 1.1.
6. Whether to ship the editor itself under an open-source license with a paid cloud tier, or closed with a free tier. This affects hiring, community templates, and Bevy upstreaming strategy.
7. Final name for the game, chosen from Appendix D after a trademark, domain, and store search. The engine name is decided: Incant. Still to do for it: trademark search (USPTO, EUIPO), domain registration, and reserving `incant_*` on crates.io and the `@incant` scope on npm.
8. Codex execution mode for Astra: cloud tasks only, CLI only, or both.
9. Approval mode per risk level for agent PRs, so the director is not the bottleneck.

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
  settings: ProjectSettings
  scenes: [SceneRef]
  assets: [AssetRef]
  scripts: [ScriptRef]
  skills: [SkillRef]
  memory: MemoryDoc

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
  Replicated { owner, mode: server|rollback, fields }
  UiRoot { layout: UiTree }
  GeometryGraph { nodes, edges, outputs }
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
- **Director:** the human (Andrey) who sets priorities, approves gates, merges PRs, and breaks routing ties.

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
