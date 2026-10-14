use clap::{Parser, Subcommand};
use incant_agent_native::GpuPerception;
use incant_render_authoring::RendererAuthoringExt as _;
mod agent_command;
mod assets;
mod engine_version;
mod eval;
mod localization;
mod native_play;
mod native_worker;
mod play;
mod play_assertions;
mod play_audio;
mod play_logs;
mod play_saves;
mod watch;
use incant_agent::{
    Agent, ApprovalMode, Budget, accounts::AccountStore, credentials::CredentialStore,
    provider::OpenAiProvider,
};
use incant_cmd::{Actor, CommandBus};
use incant_core::Engine;
use incant_doc::{Entity, Project, Scene, Transform, new_id};
use incant_play_legacy::PlaySession;
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    error::Error,
    fs,
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
#[derive(Parser)]
#[command(name = "incant", about = "Incant headless engine and project tools")]
struct Args {
    #[command(subcommand)]
    command: Cli,
}
#[derive(Subcommand)]
enum Cli {
    EvalCheck {
        #[arg(default_value = "evals/phase0/tasks.json")]
        suite: PathBuf,
    },
    Eval {
        #[arg(long, default_value = "evals/phase0/tasks.json")]
        suite: PathBuf,
        #[arg(long, default_value = "artifacts/live-eval.json")]
        output: PathBuf,
        #[arg(long)]
        model: String,
        #[arg(long, default_value_t = 128000)]
        max_tokens: u64,
        #[arg(long, default_value_t = 25000)]
        max_output_tokens: u64,
        /// Run one existing case for diagnosis; cannot pass the twenty-task gate.
        #[arg(long)]
        case: Option<String>,
        /// Use only a dedicated evaluation key from the environment, never OS accounts.
        #[arg(long)]
        ci: bool,
    },
    Init {
        path: PathBuf,
        #[arg(long, default_value = "Untitled")]
        name: String,
        #[arg(long, default_value_t = 1)]
        entities: usize,
    },
    /// Import a project-local model, image or WAV/OGG clip through the shared command bus.
    Import {
        project: PathBuf,
        source: PathBuf,
        #[arg(long)]
        cache: Option<PathBuf>,
        #[arg(long, value_parser = ["color", "linear", "normal"])]
        texture_usage: Option<String>,
    },
    /// Reimport registered sources and reload CPU assets; emits JSON lines until interrupted.
    WatchAssets {
        project: PathBuf,
        #[arg(long, default_value_t = 500, value_parser = clap::value_parser!(u64).range(10..=60000))]
        interval_ms: u64,
        #[arg(long, default_value_t = 300, value_parser = clap::value_parser!(u64).range(0..=60000))]
        debounce_ms: u64,
        /// Stop after this many polls (otherwise keep watching). Unsettled/error state fails.
        #[arg(long, value_parser = clap::value_parser!(u64).range(1..))]
        polls: Option<u64>,
    },
    Validate {
        project: PathBuf,
    },
    /// Inspect the declared engine requirement without changing the project.
    EngineCompatibility {
        project: PathBuf,
    },
    /// Review an explicit metadata pin for a legacy unpinned project; writes nothing.
    ReviewEnginePin {
        project: PathBuf,
    },
    /// Report absent translations; exits unsuccessfully unless fallback is explicitly allowed.
    LocalizationCheck {
        project: PathBuf,
        #[arg(long)]
        locale: Option<String>,
        #[arg(long)]
        allow_fallback: bool,
    },
    /// Export one table as a new UTF-8 XLIFF file.
    LocalizationExport {
        project: PathBuf,
        table_id: String,
        locale: String,
        output: PathBuf,
    },
    /// Import matching XLIFF targets in one reversible shared transaction.
    LocalizationImport {
        project: PathBuf,
        input: PathBuf,
    },
    Schema {
        directory: PathBuf,
    },
    Run {
        project: PathBuf,
        #[arg(long, default_value_t = 120)]
        ticks: u64,
    },
    /// Run isolated gameplay, optionally recording actual GPU frames and a report.
    Play(play::Options),
    /// Run a selected project scene on the native runtime with startup-only cooking.
    NativePlay(native_play::Options),
    /// Retain a native CPU Play session behind a bounded read/control-only pipe.
    NativeWorker(native_worker::Options),
    Script {
        project: PathBuf,
        compiled_script: PathBuf,
        #[arg(long, default_value_t = 60)]
        ticks: u64,
    },
    Screenshot {
        project: PathBuf,
        output: PathBuf,
        /// Authored Camera entity ID; omit for the editor preview.
        #[arg(long)]
        camera: Option<String>,
        #[arg(long, default_value_t = 1280)]
        width: u32,
        #[arg(long, default_value_t = 720)]
        height: u32,
    },
    Rpc {
        project: PathBuf,
        #[arg(long)]
        journal: Option<PathBuf>,
    },
    Agent(agent_command::Options),
    Auth {
        #[command(subcommand)]
        action: AuthCli,
    },
}
#[derive(Subcommand)]
enum AuthCli {
    Login {
        #[arg(long)]
        add: bool,
    },
    ApiKey,
    Refresh,
    Models,
    Status,
    #[command(alias = "keychain-check")]
    CredentialCheck,
    Accounts,
    Switch {
        account: String,
    },
    Disconnect,
}
fn read_project(path: &Path) -> Result<Project> {
    Ok(Project::from_text(&fs::read_to_string(path)?)?)
}
fn save_derived(path: &Path, text: &str) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(text.as_bytes())?;
    file.as_file().sync_all()?;
    file.persist(path)?;
    Ok(())
}
fn print(value: impl Serialize) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}
fn provider(model: String) -> Result<OpenAiProvider> {
    Ok(AccountStore::open()?.provider(model)?)
}
fn auth_command(action: AuthCli) -> Result<()> {
    match action {
        AuthCli::Login { add } => {
            let (attempt, subject) = AccountStore::open()?.prepare_login(add, None)?;
            // No account lock held while the person signs in. Never log a URL
            // containing a returning account's ID-token hint.
            attempt.open_browser()?;
            println!("Incant opened your system browser. Complete Continue with ChatGPT there.");
            let account = attempt.finish(subject.as_deref(), Duration::from_secs(300))?;
            AccountStore::open()?.activate(account)?;
            println!(
                "Connected. This login is shared by the editor and CLI and survives rebuilds."
            );
        }
        AuthCli::Refresh => {
            let store = AccountStore::open()?;
            let account = store.selected().ok_or("No active ChatGPT account")?;
            incant_agent::auth::refresh_access_token(account)?;
            print(json!({"session_refreshed":true,"storage":CredentialStore::storage_kind()}))?;
        }
        AuthCli::ApiKey => {
            let key = rpassword::prompt_password("OpenAI API key (hidden): ")?;
            OpenAiProvider::new(key.clone(), "connection-validation".into())?.models()?;
            let mut store = AccountStore::open()?;
            store.invalidate_selection()?;
            CredentialStore::save("api-key", &key)?;
            store.data.api_key_connected = true;
            store.data.active = None;
            store.save()?;
            println!("API key validated and stored in Incant's private credential store.");
        }
        AuthCli::CredentialCheck => {
            let account = format!("smoke-{}", new_id());
            let secret = format!("temporary-probe-{}", new_id());
            CredentialStore::save(&account, &secret)?;
            let loaded = CredentialStore::load(&account);
            let deleted = CredentialStore::delete(&account);
            let matched = loaded?.as_str() == secret;
            deleted?;
            if !matched {
                return Err("credential probe did not round-trip".into());
            }
            print(
                json!({"credential_roundtrip":true,"storage":CredentialStore::storage_kind(),"temporary_credential_deleted":true,"os":std::env::consts::OS}),
            )?;
        }
        AuthCli::Accounts => print(&AccountStore::open()?.data.accounts)?,
        AuthCli::Switch { account } => {
            if !AccountStore::open()?.switch(&account)? {
                let (attempt, subject) =
                    AccountStore::open()?.prepare_login(false, Some(&account))?;
                attempt.open_browser()?;
                let account = attempt.finish(subject.as_deref(), Duration::from_secs(300))?;
                AccountStore::open()?.activate(account)?;
            }
            println!("Active OpenAI account changed.");
        }
        AuthCli::Models => print(provider("model-catalog".into())?.models()?)?,
        AuthCli::Status => {
            let store = AccountStore::open()?;
            let credential_available = if let Some(account) = store.selected() {
                CredentialStore::load_optional(&account.id)?.is_some()
            } else if store.data.api_key_connected {
                CredentialStore::load_optional("api-key")?.is_some()
            } else {
                false
            };
            print(
                json!({"oauth_accounts":store.data.accounts.len(),"active_oauth":store.data.active.is_some(),"api_key_connected":store.data.api_key_connected,"credential_available":credential_available,"storage":CredentialStore::storage_kind()}),
            )?;
        }
        AuthCli::Disconnect => {
            let revoked = AccountStore::open()?.disconnect()?;
            let remote = match revoked {
                incant_agent::accounts::RemoteRevocation::Confirmed => "confirmed",
                incant_agent::accounts::RemoteRevocation::Unconfirmed => "unconfirmed",
                incant_agent::accounts::RemoteRevocation::NotApplicable => "not performed",
            };
            println!("Local credentials removed. Remote revocation: {remote}");
        }
    }
    Ok(())
}
fn rpc(project: PathBuf, journal: Option<PathBuf>) -> Result<()> {
    let root = project
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let journal = journal.unwrap_or_else(|| project.with_extension("journal.jsonl"));
    let mut bus = CommandBus::open_project_file_with_journal(&project, journal)?;
    let mut play = incant_agent_native::Controller::new(native_play::embedded::load);
    for line in io::stdin().lock().lines() {
        let line = line?;
        if line.len() > 16 * 1024 * 1024 {
            return Err("RPC request too large".into());
        }
        let request: Value = serde_json::from_str(&line)?;
        let id = request.get("id").cloned().unwrap_or(Value::Null);
        let response = (|| -> Result<Value> {
            let empty = json!({});
            let params = request.get("params").unwrap_or(&empty);
            match request["method"].as_str().ok_or("missing RPC method")?{
  "project.engine_compatibility"=>Ok(serde_json::to_value(bus.project().engine_compatibility()?)?),
  "project.review_engine_pin"=>engine_version::review(&bus,&project,params.clone()),
  "project.read"=>Ok(json!({"project":bus.project(),"revision":bus.revision(),"project_file":{"available":bus.has_project_file()}})),
  "project.file"=>Ok(serde_json::to_value(bus.project_file_request(serde_json::from_value(params.clone())?,Actor::user("editor"))?)?),
  "project.save"=>{let request: engine_version::SaveRequest=serde_json::from_value(params.clone())?;let receipt=bus.save_project_file(request.expected_revision.unwrap_or_else(|| bus.revision()),Actor::user("editor"))?;Ok(json!({"saved":true,"receipt":receipt}))},
  "schema.list"=>Ok(json!(engine_version::schemas())),
  "command.execute"=>{let commands=serde_json::from_value(params["commands"].clone())?;let revision=params["expected_revision"].as_u64().ok_or("expected_revision required")?;let tx=bus.execute(commands,Actor::user("editor"),params["description"].as_str().unwrap_or("Editor edit"),Some(revision))?;let tid=tx.id.clone();Ok(json!({"transaction_id":tid,"revision":bus.revision(),"project":bus.project()}))},
  "script.read"=>Ok(incant_agent::dispatch(&mut bus,&mut incant_agent::NoViewport,"script_read",params.clone(),Actor::user("editor"))?),
  "script.write"=>Ok(incant_agent::dispatch(&mut bus,&mut incant_agent::NoViewport,"script_write",params.clone(),Actor::user("editor"))?),
  "history.read"=>Ok(json!(bus.history().iter().map(|t|json!({"id":t.id,"description":t.description,"actor":t.actor,"commands":t.commands.len()})).collect::<Vec<_>>())),
  "history.undo"=>{let tx=bus.undo()?;Ok(json!({"transaction_id":tx,"revision":bus.revision(),"project":bus.project()}))},
  "history.redo"=>{let tx=bus.redo()?;Ok(json!({"transaction_id":tx,"revision":bus.revision(),"project":bus.project()}))},
  "play.run"=>{bus.ensure_sources_ready()?;Ok(play.run(bus.project(),bus.revision(),root,serde_json::from_value(params.clone())?,&AtomicBool::new(false))?)},
  "play.state_query"=>{let request: incant_agent::play::StateQuery=serde_json::from_value(params.clone())?;Ok(play.state_query(&request.path)?)},
  "play.start"=>{bus.ensure_sources_ready()?;Ok(play.start(&bus,root,params.clone())?)},
  "play.step"=>Ok(play.step(params.clone())?),
  "play.state"=>Ok(play.state(params.clone())?),
  "play.stop"=>{play.stop(params.clone())?;Ok(json!({"playing":false,"project":bus.project(),"revision":bus.revision()}))},
  _=>Err("unknown RPC method".into()),
 }
        })();
        let response = match response {
            Ok(result) => json!({"id":id,"result":result}),
            Err(error) => json!({"id":id,"error":{"message":error.to_string()}}),
        };
        println!("{}", response);
        io::stdout().flush()?;
    }
    Ok(())
}
fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Error: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
fn run() -> Result<()> {
    match Args::parse().command {
        Cli::Init {
            path,
            name,
            entities,
        } => {
            if entities > 10_000 {
                return Err("maximum initial scene size is 10000 entities".into());
            }
            let mut project = Project::empty(name);
            let mut scene = Scene::new("Main");
            for i in 0..entities {
                let mut entity = Entity::new(format!("Entity {i}"));
                entity.components.insert(
                    "Transform".into(),
                    json!(Transform {
                        translation: [(i % 32) as f64, 0., (i / 32) as f64],
                        ..Default::default()
                    }),
                );
                scene.entities.insert(entity.id.clone(), entity);
            }
            project.scenes.insert(scene.id.clone(), scene);
            let text = project.canonical_text()?;
            if let Some(parent) = path.parent()
                && !parent.as_os_str().is_empty()
            {
                fs::create_dir_all(parent)?;
            }
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)?;
            file.write_all(text.as_bytes())?;
            file.sync_all()?;
            print(json!({"created":project.id,"entities":entities}))?;
        }
        Cli::Validate { project } => assets::validate(&project)?,
        Cli::EngineCompatibility { project } => print(engine_version::inspect(&project, false)?)?,
        Cli::ReviewEnginePin { project } => print(engine_version::inspect(&project, true)?)?,
        Cli::LocalizationCheck {
            project,
            locale,
            allow_fallback,
        } => {
            let project = read_project(&project)?;
            let mut settings = project.settings.localization.clone();
            if let Some(locale) = locale {
                settings.locale = locale;
            }
            let catalog = incant_localization::Catalog::compile(&project.string_tables)?;
            let missing = catalog.missing_strings(&settings)?;
            let complete = missing.is_empty();
            print(json!({"project_id":project.id,"locale":settings.locale,
                "complete":complete,"fallback_allowed":allow_fallback,"missing":missing}))?;
            if !complete && !allow_fallback {
                return Err("localization has missing translations".into());
            }
        }
        Cli::LocalizationExport {
            project,
            table_id,
            locale,
            output,
        } => localization::export(&project, &table_id, &locale, &output)?,
        Cli::LocalizationImport { project, input } => localization::import(&project, &input)?,
        Cli::Import {
            project,
            source,
            cache,
            texture_usage,
        } => assets::import(
            &project,
            &source,
            cache.as_deref(),
            texture_usage.as_deref(),
        )?,
        Cli::WatchAssets {
            project,
            interval_ms,
            debounce_ms,
            polls,
        } => {
            watch::run(
                &project,
                Duration::from_millis(interval_ms),
                Duration::from_millis(debounce_ms),
                polls,
            )?;
        }
        Cli::Schema { directory } => {
            fs::create_dir_all(&directory)?;
            let mut registry = engine_version::schemas();
            registry.insert(
                "GameSave".into(),
                json!(schemars::schema_for!(incant_play_legacy::GameSave)),
            );
            registry.insert(
                "ScriptClock".into(),
                json!(schemars::schema_for!(incant_play_legacy::ScriptClock)),
            );
            registry.insert(
                "TimerRequest".into(),
                json!(schemars::schema_for!(incant_play_legacy::TimerRequest)),
            );
            registry.insert(
                "TimerEvent".into(),
                json!(schemars::schema_for!(incant_play_legacy::TimerEvent)),
            );
            registry.insert(
                "InputEvent".into(),
                json!(schemars::schema_for!(incant_input::InputEvent)),
            );
            registry.insert(
                "InputActions".into(),
                json!(schemars::schema_for!(incant_input::InputActions)),
            );
            registry.insert(
                "InputFrame".into(),
                json!(schemars::schema_for!(incant_input::InputFrame)),
            );
            registry.insert(
                "InputRecording".into(),
                json!(schemars::schema_for!(incant_input::InputRecording)),
            );
            registry.insert(
                "PlayAssertions".into(),
                json!(schemars::schema_for!(play_assertions::PlayAssertions)),
            );
            registry.insert(
                "Command".into(),
                json!(schemars::schema_for!(incant_cmd::Command)),
            );
            registry.insert(
                "ScriptRead".into(),
                json!(schemars::schema_for!(incant_cmd::ScriptRead)),
            );
            registry.extend([
                (
                    "StringTable".into(),
                    json!(schemars::schema_for!(incant_localization::StringTable)),
                ),
                (
                    "LocaleSettings".into(),
                    json!(schemars::schema_for!(incant_localization::LocaleSettings)),
                ),
                (
                    "LocalizeRequest".into(),
                    json!(schemars::schema_for!(incant_localization::LocalizeRequest)),
                ),
                (
                    "LocalizedText".into(),
                    json!(schemars::schema_for!(incant_localization::LocalizedText)),
                ),
                (
                    "LocalizationReportRequest".into(),
                    json!(schemars::schema_for!(
                        incant_localization::LocalizationReportRequest
                    )),
                ),
                (
                    "LocalizationReport".into(),
                    json!(schemars::schema_for!(
                        incant_localization::LocalizationReport
                    )),
                ),
                (
                    "MissingString".into(),
                    json!(schemars::schema_for!(incant_localization::MissingString)),
                ),
                (
                    "CalendarDate".into(),
                    json!(schemars::schema_for!(incant_localization::CalendarDate)),
                ),
                (
                    "DateLength".into(),
                    json!(schemars::schema_for!(incant_localization::DateLength)),
                ),
            ]);
            registry.extend([
                (
                    "GridNavigationQuery".into(),
                    json!(schemars::schema_for!(incant_core::GridNavigationQuery)),
                ),
                (
                    "GridPath".into(),
                    json!(schemars::schema_for!(incant_core::GridPath)),
                ),
                (
                    "OffMeshLink".into(),
                    json!(schemars::schema_for!(incant_core::OffMeshLink)),
                ),
                (
                    "OffMeshTraversal".into(),
                    json!(schemars::schema_for!(incant_core::OffMeshTraversal)),
                ),
                (
                    "SteeringQuery".into(),
                    json!(schemars::schema_for!(incant_core::SteeringQuery)),
                ),
                (
                    "SteeringAgent".into(),
                    json!(schemars::schema_for!(incant_core::SteeringAgent)),
                ),
                (
                    "SteeringObstacle".into(),
                    json!(schemars::schema_for!(incant_core::SteeringObstacle)),
                ),
                (
                    "SteeringVelocity".into(),
                    json!(schemars::schema_for!(incant_core::SteeringVelocity)),
                ),
                (
                    "NavigationQuery".into(),
                    json!(schemars::schema_for!(incant_core::NavigationQuery)),
                ),
                (
                    "NavigationPath".into(),
                    json!(schemars::schema_for!(incant_core::NavigationPath)),
                ),
                (
                    "PhysicsCharacterQuery".into(),
                    json!(schemars::schema_for!(incant_core::CharacterQuery)),
                ),
                (
                    "PhysicsCharacterMovement".into(),
                    json!(schemars::schema_for!(incant_core::CharacterMovement)),
                ),
                (
                    "PhysicsRayQuery".into(),
                    json!(schemars::schema_for!(incant_core::RayQuery)),
                ),
                (
                    "PhysicsRayHit".into(),
                    json!(schemars::schema_for!(incant_core::RayHit)),
                ),
                (
                    "PhysicsTriggerEvent".into(),
                    json!(schemars::schema_for!(incant_core::TriggerEvent)),
                ),
                (
                    "PhysicsRayQuery2D".into(),
                    json!(schemars::schema_for!(incant_runtime::RayQuery2D<String>)),
                ),
                (
                    "PhysicsRayHit2D".into(),
                    json!(schemars::schema_for!(incant_runtime::RayHit2D<String>)),
                ),
                (
                    "PhysicsTriggerEvent2D".into(),
                    json!(schemars::schema_for!(
                        incant_runtime::TriggerEvent2D<String>
                    )),
                ),
            ]);
            for (name, schema) in registry {
                save_derived(
                    &directory.join(format!("{name}.schema.json")),
                    &(serde_json::to_string_pretty(&schema)? + "\n"),
                )?;
            }
            save_derived(
                &directory.join("agent-tools.json"),
                &(serde_json::to_string_pretty(&incant_agent::tools())? + "\n"),
            )?;
        }
        Cli::Run { project, ticks } => {
            if ticks > 1_000_000 {
                return Err("tick count exceeds limit".into());
            }
            let document = read_project(&project)?;
            let assets = assets::load_runtime(&project, &document)?;
            let mut engine =
                Engine::with_navigation_resources(&document, assets.navigation_resources())?;
            let start = Instant::now();
            engine.run_ticks(ticks)?;
            print(
                json!({"state":engine.snapshot(),"assets":assets.snapshot(),"wall_ms":start.elapsed().as_secs_f64()*1000.}),
            )?;
        }
        Cli::NativePlay(options) => {
            let report = native_play::run(options)?;
            let passed = report.passed;
            print(report)?;
            if !passed {
                return Err("native gameplay assertions failed; see completed report".into());
            }
        }
        Cli::NativeWorker(options) => native_worker::run(options)?,
        Cli::Play(options) => {
            let report = play::run(options).map_err(|error| error.to_string())?;
            let passed = report.passed;
            print(report)?;
            if !passed {
                return Err("gameplay assertions failed; see JSON assertion results".into());
            }
        }
        Cli::Script {
            project,
            compiled_script,
            ticks,
        } => {
            if ticks > 10000 {
                return Err("script tick count exceeds limit".into());
            }
            let document = read_project(&project)?;
            let assets = assets::load_runtime(&project, &document)?;
            let mut play = PlaySession::with_navigation_resources(
                &document,
                &fs::read_to_string(compiled_script)?,
                assets.navigation_resources(),
            )
            .map_err(|error| error.to_string())?;
            let mut times = vec![];
            let mut count = 0;
            let mut logs = play_logs::Capture::new(None, None)?;
            for tick in 1..=ticks {
                let start = Instant::now();
                count += play.tick().map_err(|source| {
                    self::play::PlayError::Tick {
                        tick: play.host.clock().tick.saturating_add(1),
                        source,
                    }
                    .to_string()
                })?;
                times.push(start.elapsed().as_secs_f64() * 1000.);
                let entries = play.host.take_logs();
                if !entries.is_empty() {
                    logs.append(tick, play.snapshot().elapsed_seconds, entries)?;
                }
            }
            let logs = logs.finish()?;
            times.sort_by(f64::total_cmp);
            let p95 = times
                .get((times.len() * 95 / 100).min(times.len().saturating_sub(1)))
                .copied()
                .unwrap_or(0.);
            print(
                json!({"ticks":ticks,"commands":count,"state":play.host.state(),"logs":logs,"p95_frame_ms":p95,"max_frame_ms":times.last()}),
            )?;
        }
        Cli::Screenshot {
            project,
            output,
            camera,
            width,
            height,
        } => {
            let renderer = incant_render::Renderer::headless().map_err(|e| e.to_string())?;
            let document = read_project(&project)?;
            let assets = assets::load_runtime(&project, &document)?;
            let scene = renderer
                .prepare_scene(&document, &assets)
                .map_err(|e| e.to_string())?;
            let scene = if let Some(id) = &camera {
                scene.with_camera(id).map_err(|e| e.to_string())?
            } else {
                scene
            };
            let bytes = renderer
                .screenshot_scene_png(&scene, width, height)
                .map_err(|e| e.to_string())?;
            if let Some(parent) = output.parent()
                && !parent.as_os_str().is_empty()
            {
                fs::create_dir_all(parent)?;
            }
            fs::write(&output, bytes)?;
            print(
                json!({"output":output,"adapter":renderer.adapter_name,"width":width,"height":height,"geometry":scene.stats(),"shading":scene.shading(),"camera":camera}),
            )?;
        }
        Cli::Rpc { project, journal } => rpc(project, journal)?,
        Cli::EvalCheck { suite } => eval::check(&suite)?,
        Cli::Eval {
            suite,
            output,
            model,
            max_tokens,
            max_output_tokens,
            case,
            ci,
        } => eval::run(
            &suite,
            &output,
            model,
            max_tokens,
            max_output_tokens,
            ci,
            case.as_deref(),
        )?,
        Cli::Auth { action } => auth_command(action)?,
        Cli::Agent(options) => agent_command::run(options)?,
    }
    Ok(())
}
