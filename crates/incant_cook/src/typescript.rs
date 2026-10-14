//! Authoring-only fixed TypeScript compiler shared by developer Play and trusted
//! native builds. Project data never chooses the executable, flags or environment.
use crate::{CookedProjectBuild, CookedProjectProgram, PreparedScriptScene, ProjectScriptError};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};

mod phases;
#[cfg(test)]
mod tests;
#[cfg(windows)]
mod windows;

const DRIVER: &str = "tools/build_native_script.mjs";
const DEADLINE: Duration = Duration::from_secs(8);
const MAX_LOG_BYTES: u64 = 2 * 1024 * 1024;
const DEADLINE_MESSAGE: &str = "native TypeScript cook exceeded its 8 second deadline";

/// The caller has already checked the document and bounded all attachments.
/// A temporary private artifact is consumed only after a successful fixed cook.
pub fn compile_project_sources(
    prepared: PreparedScriptScene,
    root: &Path,
) -> Result<ProjectCompilation> {
    let (program, source_map, compiler_manifest) =
        compile_sources(prepared, root, PreparedScriptScene::compile)?;
    Ok(ProjectCompilation {
        program,
        source_map,
        compiler_manifest,
    })
}

/// Fresh source compilation into a build-only, destination-bound program. This
/// never accepts a compiled development program or a supplied executable cache.
pub fn compile_project_sources_for_target(
    prepared: PreparedScriptScene,
    root: &Path,
    target: &str,
) -> Result<ProjectBuildCompilation> {
    let (program, source_map, compiler_manifest) = compile_sources(prepared, root, |p, source| {
        p.compile_for_target(source, target)
    })?;
    Ok(ProjectBuildCompilation {
        program,
        source_map,
        compiler_manifest,
    })
}

fn compile_sources<T>(
    prepared: PreparedScriptScene,
    root: &Path,
    compile: impl FnOnce(PreparedScriptScene, &str) -> std::result::Result<T, ProjectScriptError>,
) -> Result<(T, String, serde_json::Value)> {
    let root = root.canonicalize()?;
    let source_epoch = incant_source::check_recovery(&root)?;
    let engine = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    #[cfg(windows)]
    let launch_engine = windows::launch_directory(&engine)?;
    #[cfg(not(windows))]
    let launch_engine = &engine;
    let driver = engine.join(DRIVER);
    if !driver.is_file()
        || !engine
            .join("node_modules/typescript/package.json")
            .is_file()
    {
        return Err(invalid(
            "authored TypeScript requires the development checkout's pinned Node dependencies",
        ));
    }
    let node = node_executable(&root)?;
    let directory = tempfile::Builder::new()
        .prefix("incant-native-project-cook-")
        .tempdir()?;
    let catalog = serde_json::to_vec(prepared.source_catalog())?;
    if catalog.len() > 96 * 1024 {
        return Err(invalid("source catalog exceeds its byte limit"));
    }
    let input_path = directory.path().join("catalog.json");
    fs::write(&input_path, catalog)?;
    let stdout_path = directory.path().join("stdout");
    let stderr_path = directory.path().join("stderr");
    let phase_path = directory.path().join("phases.jsonl");
    File::create(&phase_path)?;
    let output = directory.path().join("cooked");
    let mut command = Command::new(&node);
    command
        .args(["--max-old-space-size=512", "--disable-proto=throw"])
        // Rust canonicalizes Windows paths to \?\ drive/UNC syntax. Pinned Node
        // can fail in its CLI realpathSync before entering a namespaced script
        // (nodejs/node#62446). Resolve only this fixed entry relative to the
        // trusted engine cwd; retain canonical paths for checks and project data.
        .arg(DRIVER)
        .arg("--project")
        .arg(&root)
        .arg(&output)
        .current_dir(launch_engine)
        .env_clear()
        .env("PATH", node.parent().expect("canonical executable parent"))
        .env("TMPDIR", directory.path())
        .env("TEMP", directory.path())
        .env("TMP", directory.path())
        .env("INCANT_NATIVE_COOK_PHASE_FILE", &phase_path)
        // The pinned SWC carrier materializes its verified native payload. Keep
        // this transient cache private instead of inheriting a user's HOME/cache.
        // Its no-symlink check requires resolving macOS's /var -> /private/var.
        .env(
            "SWC_NATIVE_BINDING_CACHE",
            directory.path().canonicalize()?.join("swc"),
        )
        .stdin(Stdio::from(File::open(input_path)?))
        .stdout(Stdio::from(File::create(&stdout_path)?))
        .stderr(Stdio::from(File::create(&stderr_path)?));
    // Windows needs these OS installation locations. Node configuration and
    // credentials never enter the compiler's process environment.
    for key in ["SystemRoot", "SystemDrive", "WINDIR"] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    let mut child = CookChild(command.spawn()?);
    let started = Instant::now();
    let status = wait_for_cook(
        &mut child,
        started,
        DEADLINE,
        &stdout_path,
        &stderr_path,
        &phase_path,
    );
    // Close all writes before reading the sidecar, including a deadline with no
    // first phase. This also reaps the child on every refusal/error path.
    drop(child);
    let timed_out = matches!(&status, Err(TypeScriptCookError::Invalid(DEADLINE_MESSAGE)));
    if timed_out
        || std::env::var_os("INCANT_NATIVE_COOK_DIAGNOSTIC").as_deref()
            == Some(std::ffi::OsStr::new("1"))
    {
        let outcome = if timed_out {
            "deadline"
        } else if status.as_ref().is_ok_and(ExitStatus::success) {
            "exited_success"
        } else {
            "refused"
        };
        eprintln!(
            "INCANT_NATIVE_COOK_PHASES {}",
            phases::observation(&phase_path, started.elapsed(), outcome)
        );
    }
    let status = status?;
    if !status.success() {
        let diagnostic = bounded(&stderr_path, MAX_LOG_BYTES as usize)?;
        let message = String::from_utf8_lossy(&diagnostic);
        return Err(TypeScriptCookError::Diagnostic(
            message.chars().take(16_384).collect(),
        ));
    }
    let bytes = bounded(
        &output.join("behavior.cjs"),
        incant_script::artifact::MAX_SOURCE_BYTES,
    )?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&bounded(&output.join("manifest.json"), 128 * 1024)?)?;
    let map = bounded(&output.join("behavior.cjs.map"), 4 * 1024 * 1024)?;
    if manifest["format"] != "incant-native-typescript-project"
        || manifest["version"] != 1
        || &manifest["sources"] != prepared.source_catalog()
        || manifest["bundleSha256"] != format!("{:x}", Sha256::digest(&bytes))
        || manifest["mapSha256"] != format!("{:x}", Sha256::digest(&map))
    {
        return Err(invalid("fresh native compiler output/catalog mismatch"));
    }
    // Compile fresh text only. Never deserialize a supplied executable cache.
    let source = std::str::from_utf8(&bytes)?;
    let compiled = (
        compile(prepared, source)?,
        String::from_utf8(map)?,
        manifest,
    );
    if incant_source::check_recovery(&root)? != source_epoch {
        return Err(TypeScriptCookError::SourcesChanged);
    }
    Ok(compiled)
}

fn wait_for_cook(
    child: &mut CookChild,
    started: Instant,
    deadline: Duration,
    stdout_path: &Path,
    stderr_path: &Path,
    phase_path: &Path,
) -> Result<ExitStatus> {
    loop {
        if let Some(status) = child.0.try_wait()? {
            return Ok(status);
        }
        if started.elapsed() > deadline {
            return Err(invalid(DEADLINE_MESSAGE));
        }
        if fs::metadata(stdout_path)?.len() > MAX_LOG_BYTES
            || fs::metadata(stderr_path)?.len() > MAX_LOG_BYTES
            || fs::metadata(phase_path)?.len() > phases::MAX_BYTES
        {
            return Err(invalid(
                "native TypeScript cook exceeded its diagnostic byte limit",
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Both outputs were produced in this invocation from one immutable source graph.
/// Source maps are authoring symbols and must not be embedded in the player.
pub struct ProjectCompilation {
    pub program: CookedProjectProgram,
    pub source_map: String,
    pub compiler_manifest: serde_json::Value,
}

pub struct ProjectBuildCompilation {
    pub program: CookedProjectBuild,
    pub source_map: String,
    pub compiler_manifest: serde_json::Value,
}

#[derive(Debug, thiserror::Error)]
pub enum TypeScriptCookError {
    #[error(transparent)]
    Source(#[from] incant_source::SourceError),
    #[error("script sources changed during compilation; retry from the current project")]
    SourcesChanged,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),
    #[error(transparent)]
    Text(#[from] std::string::FromUtf8Error),
    #[error(transparent)]
    Project(#[from] ProjectScriptError),
    #[error("native TypeScript cook: {0}")]
    Invalid(&'static str),
    #[error("native TypeScript cook failed: {0}")]
    Diagnostic(String),
}
type Result<T> = std::result::Result<T, TypeScriptCookError>;
fn invalid(message: &'static str) -> TypeScriptCookError {
    TypeScriptCookError::Invalid(message)
}
fn bounded(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(invalid("compiler output exceeds its byte limit"));
    }
    Ok(bytes)
}

/// Always kill and reap on deadline/error before temporary inputs are removed.
struct CookChild(Child);
impl CookChild {
    fn stop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
impl Drop for CookChild {
    fn drop(&mut self) {
        self.stop();
    }
}
fn node_executable(project: &Path) -> Result<PathBuf> {
    let executable = if cfg!(windows) { "node.exe" } else { "node" };
    let paths = std::env::var_os("PATH")
        .ok_or_else(|| invalid("Node is not installed in the host's PATH"))?;
    for directory in std::env::split_paths(&paths) {
        if !directory.is_absolute() {
            continue;
        }
        let path = directory.join(executable);
        if path.is_file() {
            let path = path.canonicalize()?;
            if !path.starts_with(project) {
                return Ok(path);
            }
        }
    }
    Err(invalid(
        "a host-installed Node executable outside the project is required",
    ))
}
