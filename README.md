# Incant

Incant is an in-progress, text-native game engine and editor. This repository is
implementing [PLAN.md](PLAN.md). **Current stage: Phase 0.** It is not an Engine 1.0
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

The editor/UI build is being returned through the Claude handoff; until it is
integrated, use the headless commands below. Build-time Claude login is needed only
for visual development, not for using the engine.

## Headless tools

```sh
tools/cargo run -p incant_headless -- init artifacts/demo.incant.json --name Demo
tools/cargo run -p incant_headless -- validate artifacts/demo.incant.json
tools/cargo run -p incant_headless -- run artifacts/demo.incant.json --ticks 120
tools/cargo run -p incant_headless -- screenshot artifacts/demo.incant.json artifacts/demo.png
node tools/build_script.mjs sdk/templates/kinematic/move.ts artifacts/move.js
tools/cargo run -p incant_headless --release -- script artifacts/demo.incant.json artifacts/move.js --ticks 120
```

`init` refuses to overwrite an existing file. `rpc` serves newline-delimited JSON
on stdin/stdout. `project.read`, `schema.list`, `command.execute`, `history.read`,
`history.undo`, `history.redo`, `project.save`, and disposable `play.*` operations are
available. `command.execute` requires an expected document revision. Use `--journal`
for recovery across process restarts. The journal and project checkpoint belong
together; do not reuse a journal for another project.

```sh
tools/cargo run -p incant_headless -- rpc artifacts/demo.incant.json --journal artifacts/demo.journal.jsonl
```

Gameplay types are generated from Rust-derived JSON Schemas. Regenerate both after
changing the registry or commands:

```sh
tools/cargo run -p incant_headless -- schema schemas
node tools/generate_sdk.mjs
node_modules/.bin/tsc -p sdk/ts/tsconfig.json
```

## OpenAI connection and live evaluation

The CLI makes direct requests to OpenAI. It never reads Codex/Claude credentials.
Authentication is not yet verified end to end. OpenAI authorization availability
and public-client registration must work for the connecting account.

```sh
tools/cargo run -p incant_headless -- auth login
# Open the printed URL in your system browser and finish consent there.
# If OAuth is unavailable, use the hidden local prompt:
tools/cargo run -p incant_headless -- auth api-key
tools/cargo run -p incant_headless -- auth models
```

Do not paste a key or token into chat, source files or command-line arguments. The
API-key prompt is hidden and saves only to the OS keychain. `auth status` reports
connection metadata. `auth accounts`, `auth login --add`, `auth switch <account-id>`
and `auth disconnect` manage profiles. `auth keychain-check` writes/reads/deletes an
isolated temporary probe without touching existing provider credentials.

Choose an exact available model ID from `auth models`:

```sh
tools/cargo run -p incant_headless --release -- agent artifacts/demo.incant.json 'Rename the first entity to Player' --model MODEL_ID
tools/cargo run -p incant_headless --release -- eval-check
tools/cargo run -p incant_headless --release -- eval --model MODEL_ID
```

The interactive agent asks before each patch and saves a separate result file plus
its journal. The evaluation automatically edits twenty disposable fixtures and
makes billable/provider-plan requests with a per-case token cap. It checks exact
final state, successful query/patch/screenshot calls, provenance and undo. Mock
tests and `eval-check` never count toward the fourteen-of-twenty live gate.

## Verification and platform probes

```sh
npm run test:tools
python3 tools/generate_conventions.py --check
tools/cargo fmt --all --check
tools/cargo clippy --workspace --all-targets --locked -- -D warnings
tools/cargo test --workspace --release --locked
tools/cargo run -p incant_cmd --release --example collaboration_spike
tools/cargo run -p incant_doc --release --example crdt_benchmark
tools/cargo run -p incant_platform_smoke --release
```

The platform probe exercises actual document parsing and Bevy simulation. Build
hosts live in [platforms](platforms), with commands in [tools/platforms](tools/platforms).
WASM requires `wasm-bindgen-cli` 0.2.126. iOS simulator builds require Xcode; device
builds are unsigned until the director provides signing/provisioning. Android uses
Gradle 8.11.1, JDK 17, Android SDK 35, NDK and cargo-ndk 4.1.2. Platform packaging is
a hello-world proof, not an exported game or a mobile editor.

GitHub Actions configurations are prepared for local checks, nightly six-target
probes, and protected live evaluation. They have not run because no remote is
configured. Nightly debug artifacts do not satisfy signed-release requirements.
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
