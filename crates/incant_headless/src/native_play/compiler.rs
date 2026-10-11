//! Developer-only fixed compiler invocation. Agent arguments cannot choose a
//! program, flags, config, output path or environment. Gameplay gets no process API.
use super::{Result, bounded, invalid, native};
use incant_cook::{CookedProjectProgram, PreparedScriptScene};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

const DEADLINE: Duration = Duration::from_secs(8);
const MAX_LOG_BYTES: u64 = 2 * 1024 * 1024;

/// The caller has already checked the document and bounded all attachments.
/// A temporary private artifact is consumed only after a successful fixed cook.
pub(super) fn compile(prepared: PreparedScriptScene, root: &Path) -> Result<CookedProjectProgram> {
    let root = root.canonicalize()?;
    let engine = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    let driver = engine.join("tools/build_native_script.mjs");
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
    let catalog = serde_json::to_vec(prepared.source_catalog()).map_err(native)?;
    if catalog.len() > 96 * 1024 {
        return Err(invalid("source catalog exceeds its byte limit"));
    }
    let input_path = directory.path().join("catalog.json");
    fs::write(&input_path, catalog)?;
    let stdout_path = directory.path().join("stdout");
    let stderr_path = directory.path().join("stderr");
    let output = directory.path().join("cooked");
    let mut command = Command::new(&node);
    command
        .args(["--max-old-space-size=512", "--disable-proto=throw"])
        .arg(&driver)
        .arg("--project")
        .arg(&root)
        .arg(&output)
        .current_dir(&engine)
        .env_clear()
        .env("PATH", node.parent().expect("canonical executable parent"))
        .env("TMPDIR", directory.path())
        .env("TEMP", directory.path())
        .env("TMP", directory.path())
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
    let status = loop {
        if let Some(status) = child.0.try_wait()? {
            break status;
        }
        if started.elapsed() > DEADLINE {
            return Err(invalid(
                "native TypeScript cook exceeded its 8 second deadline",
            ));
        }
        if fs::metadata(&stdout_path)?.len() > MAX_LOG_BYTES
            || fs::metadata(&stderr_path)?.len() > MAX_LOG_BYTES
        {
            return Err(invalid(
                "native TypeScript cook exceeded its diagnostic byte limit",
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    if !status.success() {
        let diagnostic = bounded(&stderr_path, MAX_LOG_BYTES as usize)?;
        let message = String::from_utf8_lossy(&diagnostic);
        return Err(native(CompileDiagnostic(
            message.chars().take(16_384).collect(),
        )));
    }
    let bytes = bounded(
        &output.join("behavior.cjs"),
        incant_script::artifact::MAX_SOURCE_BYTES,
    )?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&bounded(&output.join("manifest.json"), 128 * 1024)?)
            .map_err(native)?;
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
    let source = std::str::from_utf8(&bytes).map_err(native)?;
    prepared.compile(source).map_err(native)
}

#[derive(Debug, thiserror::Error)]
#[error("native TypeScript cook failed: {0}")]
struct CompileDiagnostic(String);

/// Always kill and reap on deadline/error before temporary inputs are removed.
struct CookChild(Child);
impl Drop for CookChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
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
