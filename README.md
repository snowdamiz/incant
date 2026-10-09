# Incant

Incant is an in-progress, text-native game engine and editor. This repository is
implementing [PLAN.md](PLAN.md). **Current stage: Phase 1, with open Phase 0 items deferred.** It is not an Engine 1.0
release, and Driftwake has not been built or shipped. No phase gate is approved.
See [the evidence ledger](docs/spikes/phase-0.md) and [STATUS.md](STATUS.md).

## Local setup

Install Rust with rustup, Node 22, Python 3 and the native platform build tools.
The pinned Rust toolchain includes rustfmt, clippy and LLVM tools. On macOS, use
Xcode and its command-line tools. Tauri also requires its native system prerequisites
on Windows/Linux. `tools/cargo` finds the rustup toolchain when Cargo is not on PATH.

```sh
npm ci
npm run build --workspace editor/ui
tools/cargo build --workspace --locked
```

The editor UI and native bridge are integrated. Build-time Claude login is needed
only for visual development, not for using the engine.

`python3 tools/editor-dev.py` builds the frontend, native
transport and editor with bundled assets. On macOS it produces the unsigned
development bundle `artifacts/Incant.app`; open that bundle to run the native
window. `--release` builds an optimized version. This is a local development
package, not a signed distribution installer.

To import assets in the native editor, launch it with a saved project path. On macOS:

```sh
open -n artifacts/Incant.app --args /absolute/path/game.incant.json
```

Open **Assets → Import**, enter one or more project-relative glTF/GLB or image
paths, and import the batch. Files must already be inside the project folder.
Texture details allow Color, Linear or Normal map interpretation and reimport.
Each changed batch is one undoable history entry. Cooking runs in the background;
editing the document during cooking causes a conflict that can be retried.
The default unsaved startup project explains why imports are unavailable.
Imported assets are registered and cooked; placing models in the viewport remains
open. [Editor and agent import evidence](docs/spikes/editor-agent-asset-imports.md).

## Headless tools

```sh
tools/cargo run -p incant_headless -- init artifacts/demo.incant.json --name Demo
tools/cargo run -p incant_headless -- validate artifacts/demo.incant.json
tools/cargo run -p incant_headless -- run artifacts/demo.incant.json --ticks 120
tools/cargo run -p incant_headless -- screenshot artifacts/demo.incant.json artifacts/demo.png
node tools/build_script.mjs sdk/templates/kinematic/move.ts artifacts/move.js
tools/cargo run -p incant_headless --release -- script artifacts/demo.incant.json artifacts/move.js --ticks 120
tools/cargo run -p incant_headless --release -- play artifacts/demo.incant.json --seconds 2 --compiled-script artifacts/move.js --output artifacts/play-demo --capture-every 30 --log-output artifacts/play-demo/game.jsonl
```

Scripts emit structured output with `api.log(message, level?)`, where the default
level is `info`; `debug`, `warn` and `error` are also supported. Both `script` and
`play` include successful-tick logs in their JSON result. `play --log-output FILE`
also writes a new JSONL file as ticks complete, including when no GPU captures are
requested. Existing log files and reserved frame/report filenames are rejected.
If a later tick fails, prior complete log records remain. Output is bounded to
10,000 records and 8 MiB of JSONL data; exceeding either limit fails the run.

`play` runs an isolated simulation without changing the saved document or its
journal. It accepts `--seconds` (rounded up to a fixed tick) or `--ticks`, with a
10,000-tick limit. Without `--output` it needs no GPU. With a new output directory
it records the initial/final state and every `--capture-every` ticks as actual PNGs,
plus an atomic `report.json` containing final runtime/script state and capture times.
Capture is bounded to 128 frames and 256 MiB of raw pixels, with dimensions
from 16×16 to 1920×1080. A failed run exits
nonzero and may leave partial PNGs, but never a completed report. Current captures
use the shared renderer, including imported materials and HDR lighting followed
by the preview display transform. `screenshot --camera ENTITY_ID` and
`play --camera ENTITY_ID --output DIR` select an authored perspective Camera.
Playback follows that camera through each simulated tick. Omit `--camera` for
the fixed editor preview. The agent `view_screenshot` tool accepts the same
optional `camera` ID. Missing/non-camera IDs fail instead of silently falling
back. Camera edits use the shared command bus and normal Undo/Redo. Asset sources are unnecessary when
the cooked cache is present; the command reads the saved checkpoint, not unsaved
editor edits. Game logs and assertion-script support remain open.

`init` refuses to overwrite an existing file. `rpc` serves newline-delimited JSON
on stdin/stdout. `project.read`, `schema.list`, `command.execute`, `history.read`,
`history.undo`, `history.redo`, `project.save`, and disposable `play.*` operations are
available. `command.execute` requires an expected document revision. Use `--journal`
for recovery across process restarts. The journal and project checkpoint belong
together; do not reuse a journal for another project.

```sh
tools/cargo run -p incant_headless -- rpc artifacts/demo.incant.json --journal artifacts/demo.journal.jsonl
```

Import an existing project-local source, then watch its files and dependencies:

```sh
tools/cargo run -p incant_headless --release -- import /path/game.json models/prop.glb
tools/cargo run -p incant_headless --release -- watch-assets /path/game.json
```

`watch-assets` emits JSON lines, commits stable source changes through the shared
undoable history and reloads CPU assets. It owns the project's journal until
interrupted, so close other writers first. `--interval-ms` defaults to 500 and
`--debounce-ms` to 300. `--polls N` bounds a run and exits unsuccessfully when
sources remain unsettled or errors remain. GPU and editor hot reload are still
open; see [source watching](docs/spikes/asset-source-watch.md).

The native host injects the compiled transport before the UI loads. After editing
`editor/bridge/native.ts`, run `node tools/build_bridge.mjs`; CI checks that its
generated JavaScript is current.

Gameplay types are generated from Rust-derived JSON Schemas. Regenerate both after
changing the registry or commands:

```sh
tools/cargo run -p incant_headless -- schema schemas
node tools/generate_sdk.mjs
node_modules/.bin/tsc -p sdk/ts/tsconfig.json
```

## OpenAI connection and live evaluation

The CLI makes direct requests to OpenAI. It never reads Codex/Claude credentials.
Browser sign-in, session renewal, persistence across development rebuilds and
live scene editing are verified on macOS. At the director’s request, Windows/Linux
use CI builds, protocol tests and real-backend synthetic credential persistence in
place of manual sign-in checks. API-key fallback is covered by automated protocol
tests; no additional live key is needed for the verified OAuth workflow. Live
revocation verification remains separate.

```sh
tools/cargo run -p incant_headless -- auth login
# Finish consent in the system browser that opens automatically.
# If OAuth is unavailable, use the hidden local prompt:
tools/cargo run -p incant_headless -- auth api-key
tools/cargo run -p incant_headless -- auth models
```

Do not paste a key or token into chat, source files or command-line arguments. The
API-key prompt is hidden. On macOS, credentials live in owner-only files in
`~/Library/Application Support/Incant/credentials/`, shared by the editor, CLI and
development builds. This avoids Keychain password prompts; file permissions
protect the records, without additional encryption. Windows/Linux use their OS
credential stores. `auth status` reports connection metadata. `auth accounts`,
`auth login --add`, `auth switch <account-id>` and `auth disconnect` manage profiles.
`auth refresh` renews the active session. `auth credential-check` writes, reads and
deletes an isolated synthetic probe without touching provider credentials.

Choose an exact available model ID from `auth models`:

```sh
tools/cargo run -p incant_headless --release -- agent artifacts/demo.incant.json 'Rename the first entity to Player' --model MODEL_ID
tools/cargo run -p incant_headless --release -- eval-check
tools/cargo run -p incant_headless --release -- eval --model MODEL_ID
```

The interactive agent asks before document patches and asset imports under its
default approval policy, then saves a separate result file plus its journal.
It can list and inspect registered assets, and import existing sources inside the
opened project's folder through the same atomic import service as the editor.
The engine host supplies that folder; the model cannot choose another root,
arbitrary output paths or shell commands. The evaluation automatically edits twenty disposable fixtures and
makes billable/provider-plan requests with a per-case token cap. It checks exact
final state, successful query/patch/screenshot calls, provenance and undo. Mock
tests and `eval-check` never count toward the fourteen-of-twenty live gate.
`--max-output-tokens` defaults to 25,000 per response, including reasoning, and is
clamped to the remaining session budget. Incomplete turns never execute partial
tool calls. Their reported usage is charged; missing usage reserves the request's
conservative bound. `eval --case 12-ten-step` runs only that existing case for
diagnosis and cannot pass the full-suite gate.

## Verification and platform probes

```sh
npm run test:tools
python3 tools/generate_conventions.py --check
tools/cargo fmt --all --check
tools/cargo clippy --workspace --all-targets --locked -- -D warnings
tools/cargo test --workspace --release --locked
tools/cargo run -p incant_cmd --release --example collaboration_spike
tools/cargo run -p incant_doc --release --example crdt_bench
tools/cargo run -p incant_platform_smoke --release
```

The platform probe exercises actual document parsing and Bevy simulation. Build
hosts live in [platforms](platforms), with commands in [tools/platforms](tools/platforms).
WASM requires `wasm-bindgen-cli` 0.2.126. iOS simulator builds require Xcode; device
builds are unsigned until the director provides signing/provisioning. Android uses
Gradle 8.11.1, JDK 17, Android SDK 35, NDK and cargo-ndk 4.1.2. Platform packaging is
a hello-world proof, not an exported game or a mobile editor.

GitHub Actions configurations are prepared for local checks, nightly six-target
probes, and protected live evaluation. All six probe targets have passed hosted builds, with desktop, browser and iOS
simulator execution. Android APK Activity launch and native instrumentation also
passed in a disposable Android 35 x86_64 hosted emulator. Three-desktop synthetic credential
persistence across changed builds passes. The private remote is
`https://github.com/snowdamiz/incant`. Nightly debug artifacts do not satisfy signed-release requirements.
The [security boundaries](docs/SECURITY.md) and [architecture proposals](docs/adr)
describe current limitations.

## Visual development

Read [AGENTS.md](AGENTS.md) and [CLAUDE.md](CLAUDE.md). They are generated from
`docs/conventions/` and enforce the plan's Claude Opus 5.5 visual-work routing.
A self-contained handoff packet is required for visual work.

```sh
node_modules/.bin/claude auth login
python3 tools/handoff/main.py run 0001-editor-foundation
```

The handoff client isolates work in `.worktrees/<packet>`, explicitly selects the
required model, scopes protocol filesystem access and resumes the latest session
for that exact worktree after interruption. Review the result packet, run tests and
integrate with provenance; do not self-approve a phase gate or merge a PR.

When the director explicitly authorizes unattended tool use, the runner accepts
`--permission-mode bypassPermissions` for that ACP session. It verifies that the
adapter offers the mode and fails explicitly otherwise. This does not change
global Claude settings or broaden the task's authorized scope. Worktree-scoped
ACP filesystem handlers are not an OS sandbox for Claude's shell commands.
