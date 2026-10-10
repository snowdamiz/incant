use clap::{Parser, Subcommand};
mod assets;
mod eval;
mod play;
mod play_logs;
mod watch;
use incant_agent::{
    Agent, ApprovalMode, Budget, accounts::AccountStore, credentials::CredentialStore,
    provider::OpenAiProvider,
};
use incant_cmd::{Actor, CommandBus};
use incant_core::Engine;
use incant_doc::{Entity, Project, Scene, Transform, new_id, schema_registry};
use incant_script::PlaySession;
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
    /// Import a project-local static model or image through the shared command bus.
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
    Agent {
        project: PathBuf,
        prompt: String,
        #[arg(long)]
        model: String,
        #[arg(long, default_value_t = 64000)]
        max_tokens: u64,
        #[arg(long, default_value_t = 25000)]
        max_output_tokens: u64,
        #[arg(long)]
        journal: Option<PathBuf>,
    },
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
fn save(path: &Path, text: &str) -> Result<()> {
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
    let initial = read_project(&project)?;
    let mut bus = if let Some(path) = journal {
        CommandBus::persistent(path, initial)?
    } else {
        CommandBus::new(initial)?
    };
    let mut engine: Option<Engine> = None;
    for line in io::stdin().lock().lines() {
        let line = line?;
        if line.len() > 16 * 1024 * 1024 {
            return Err("RPC request too large".into());
        }
        let request: Value = serde_json::from_str(&line)?;
        let id = request.get("id").cloned().unwrap_or(Value::Null);
        let response = (|| -> Result<Value> {
            let params = &request["params"];
            match request["method"].as_str().ok_or("missing RPC method")?{
  "project.read"=>Ok(json!({"project":bus.project(),"revision":bus.revision()})),
  "project.save"=>{save(&project,&bus.project().canonical_text()?)?;Ok(json!({"saved":true}))},
  "schema.list"=>Ok(json!(schema_registry())),
  "command.execute"=>{let commands=serde_json::from_value(params["commands"].clone())?;let revision=params["expected_revision"].as_u64().ok_or("expected_revision required")?;let tx=bus.execute(commands,Actor::user("editor"),params["description"].as_str().unwrap_or("Editor edit"),Some(revision))?;let tid=tx.id.clone();Ok(json!({"transaction_id":tid,"revision":bus.revision(),"project":bus.project()}))},
  "history.read"=>Ok(json!(bus.history().iter().map(|t|json!({"id":t.id,"description":t.description,"actor":t.actor,"commands":t.commands.len()})).collect::<Vec<_>>())),
  "history.undo"=>{let tx=bus.undo()?;Ok(json!({"transaction_id":tx,"revision":bus.revision(),"project":bus.project()}))},
  "history.redo"=>{let tx=bus.redo()?;Ok(json!({"transaction_id":tx,"revision":bus.revision(),"project":bus.project()}))},
  "play.start"=>{engine=Some(Engine::new(bus.project())?);Ok(json!({"playing":true}))},
  "play.step"=>{let engine=engine.as_mut().ok_or("no play session")?;engine.step()?;Ok(json!(engine.snapshot()))},
  "play.state"=>Ok(json!(engine.as_mut().ok_or("no play session")?.snapshot())),
  "play.stop"=>{engine=None;Ok(json!({"playing":false,"project":bus.project()}))},
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
fn main() -> Result<()> {
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
        Cli::Validate { project } => {
            let project = read_project(&project)?;
            print(
                json!({"valid":true,"project_id":project.id,"schema_version":project.schema_version}),
            )?;
        }
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
            let mut registry = schema_registry();
            registry.insert(
                "Command".into(),
                json!(schemars::schema_for!(incant_cmd::Command)),
            );
            registry.extend([
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
            ]);
            for (name, schema) in registry {
                save(
                    &directory.join(format!("{name}.schema.json")),
                    &(serde_json::to_string_pretty(&schema)? + "\n"),
                )?;
            }
            save(
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
            let mut engine = Engine::new(&document)?;
            let start = Instant::now();
            engine.run_ticks(ticks)?;
            print(
                json!({"state":engine.snapshot(),"assets":assets.snapshot(),"wall_ms":start.elapsed().as_secs_f64()*1000.}),
            )?;
        }
        Cli::Play(options) => print(play::run(options).map_err(|error| error.to_string())?)?,
        Cli::Script {
            project,
            compiled_script,
            ticks,
        } => {
            if ticks > 10000 {
                return Err("script tick count exceeds limit".into());
            }
            let mut play = PlaySession::new(
                &read_project(&project)?,
                &fs::read_to_string(compiled_script)?,
            )?;
            let mut times = vec![];
            let mut count = 0;
            let mut logs = play_logs::Capture::new(None, None)?;
            for tick in 1..=ticks {
                let start = Instant::now();
                count += play.tick()?;
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
        Cli::Agent {
            project,
            prompt,
            model,
            max_tokens,
            max_output_tokens,
            journal,
        } => {
            let mut provider = provider(model)?;
            let initial = read_project(&project)?;
            let journal = journal.unwrap_or_else(|| project.with_extension("journal.jsonl"));
            let mut bus = CommandBus::persistent(&journal, initial)?;
            let mut agent = Agent {
                budget: Budget {
                    max_tokens,
                    used_tokens: 0,
                },
                approval: ApprovalMode::Destructive,
                max_steps: 30,
                max_output_tokens,
            };
            let root = project
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            let viewport = GpuPerception::new(Some(root.to_path_buf()))?;
            let mut host = incant_agent::ProjectHost::new(bus.project(), root, viewport)?;
            let report = agent.run(
                &mut provider,
                &mut bus,
                &mut host,
                &prompt,
                &new_id(),
                &AtomicBool::new(false),
                &mut |name, args| {
                    eprintln!("Approve {name}: {args}\nType yes to apply:");
                    let mut answer = String::new();
                    io::stdin().read_line(&mut answer).is_ok() && answer.trim() == "yes"
                },
                &mut |text| eprint!("{text}"),
            )?;
            // Keep the input file as the journal's genesis; explicit RPC save is separate.
            let output = project.with_extension("agent-result.json");
            save(&output, &bus.project().canonical_text()?)?;
            print(json!({"report":report,"output":output,"journal":journal}))?;
        }
    }
    Ok(())
}

struct GpuPerception {
    renderer: incant_render::Renderer,
    project_root: Option<PathBuf>,
    assets: incant_assets::AssetStore,
}
impl GpuPerception {
    fn new(project_root: Option<PathBuf>) -> Result<Self> {
        Ok(Self {
            renderer: incant_render::Renderer::headless().map_err(|e| e.to_string())?,
            project_root,
            assets: Default::default(),
        })
    }
}
impl incant_agent::Perception for GpuPerception {
    fn screenshot(
        &mut self,
        project: &Project,
        camera: Option<&str>,
        width: u32,
        height: u32,
    ) -> std::result::Result<Value, incant_agent::AgentError> {
        use base64::Engine;
        if let Some(root) = &self.project_root {
            self.assets.sync_project(project, root).map_err(|_| {
                incant_agent::AgentError::Tool("Cooked assets could not be loaded".into())
            })?;
        } else if !project.assets.is_empty() {
            return Err(incant_agent::AgentError::Tool(
                "This viewport has no project asset access".into(),
            ));
        }
        let scene = self
            .renderer
            .prepare_scene(project, &self.assets)
            .map_err(|_| {
                incant_agent::AgentError::Tool("Scene geometry could not be prepared".into())
            })?;
        let scene = if let Some(id) = camera {
            scene
                .with_camera(id)
                .map_err(|error| incant_agent::AgentError::Tool(error.to_string()))?
        } else {
            scene
        };
        let bytes = self
            .renderer
            .screenshot_scene_png(&scene, width, height)
            .map_err(|_| incant_agent::AgentError::Tool("GPU capture failed".into()))?;
        Ok(
            json!({"mime_type":"image/png","data_url":format!("data:image/png;base64,{}",base64::engine::general_purpose::STANDARD.encode(bytes)),"width":width,"height":height,"camera":camera}),
        )
    }
}
