//! Fixed optional development worker configuration; never part of tool schemas.
use crate::{Backend, Error, Result, SourcePackage};
use serde::Deserialize;
#[cfg(any(target_os = "macos", test))]
use serde::Serialize;
#[cfg(any(target_os = "macos", test))]
use std::collections::BTreeMap;
use std::{
    fs::{self, OpenOptions},
    io::Read,
    path::{Path, PathBuf},
    sync::Arc,
};
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub python: PathBuf,
    pub worker: PathBuf,
    pub rustc: PathBuf,
    pub validator: PathBuf,
    pub storage: PathBuf,
}
/// Trusted backend return value, never deserialized from project/tool arguments.
pub struct Built {
    pub diagnostics: String,
    pub receipt: serde_json::Value,
    pub candidate: PathBuf,
}
pub struct Local {
    config: Config,
    config_file: Option<PathBuf>,
    identities: Vec<(PathBuf, String)>,
    storage_node: std::fs::File,
    #[cfg(target_os = "macos")]
    quarantine: std::sync::Mutex<Vec<crate::process::Owned>>,
}
impl Local {
    pub fn new(config: Config) -> Result<Self> {
        if !cfg!(target_os = "macos") {
            return Err(Error::Unavailable("experimental macOS backend only".into()));
        }
        for path in [
            &config.python,
            &config.worker,
            &config.rustc,
            &config.validator,
        ] {
            if !path.is_absolute() || path.canonicalize()? != *path || !path.is_file() {
                return Err(Error::Invalid("exact canonical trusted tool required"));
            }
            trusted_permissions(path, path != &config.worker)?;
        }
        if !config.storage.is_absolute()
            || config.storage.canonicalize()? != config.storage
            || !config.storage.is_dir()
        {
            return Err(Error::Invalid("existing private host storage required"));
        }
        trusted_permissions(&config.storage, false)?;
        if config
            .worker
            .file_name()
            .is_none_or(|name| name != "rust_guest_job.py")
        {
            return Err(Error::Invalid("fixed engine worker entry required"));
        }
        let identities = identities(&config)?;
        let storage_node = crate::node::directory(&config.storage)?;
        Ok(Self {
            config,
            config_file: None,
            identities,
            storage_node,
            #[cfg(target_os = "macos")]
            quarantine: std::sync::Mutex::new(Vec::new()),
        })
    }
    pub fn from_environment() -> Result<Arc<dyn Backend>> {
        if !cfg!(target_os = "macos") {
            return Err(Error::Unavailable("experimental macOS backend only".into()));
        }
        let path = std::env::var_os("INCANT_MODULE_BUILD_CONFIG")
            .ok_or_else(|| Error::Unavailable("no trusted module worker configured".into()))?;
        let config: Config = serde_json::from_slice(&bounded(Path::new(&path), 65536)?)?;
        let mut local = Self::new(config)?;
        let path = PathBuf::from(path).canonicalize()?;
        local.config_file = Some(path.clone());
        local.identities.push(file_identity(path)?);
        Ok(Arc::new(local))
    }
    fn inputs(&self) -> Result<Vec<(PathBuf, String)>> {
        if self.config.storage.canonicalize()? != self.config.storage
            || !crate::node::matches(&self.storage_node, &self.config.storage)?
        {
            return Err(Error::Unavailable("trusted storage replaced".into()));
        }
        let mut rows = identities(&self.config)?;
        if let Some(path) = &self.config_file {
            rows.push(file_identity(path.clone())?);
        }
        Ok(rows)
    }
}
fn file_identity(path: PathBuf) -> Result<(PathBuf, String)> {
    use sha2::{Digest, Sha256};
    trusted_permissions(&path, false)?;
    if path.canonicalize()? != path {
        return Err(Error::Invalid("trusted input path replaced"));
    }
    let digest = format!("{:x}", Sha256::digest(bounded(&path, 64 * 1024 * 1024)?));
    Ok((path, digest))
}
fn trusted_permissions(path: &Path, executable: bool) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let m = fs::symlink_metadata(path)?;
        if m.is_symlink()
            || ![0, unsafe { libc::geteuid() }].contains(&m.uid())
            || m.mode() & 0o022 != 0
            || (executable && m.mode() & 0o111 == 0)
        {
            return Err(Error::Invalid(
                "owner-controlled trusted authority required",
            ));
        }
    }
    #[cfg(not(unix))]
    let _ = (path, executable);
    Ok(())
}
fn identities(config: &Config) -> Result<Vec<(PathBuf, String)>> {
    let tools = config
        .worker
        .parent()
        .ok_or(Error::Invalid("worker parent"))?;
    let repo = tools.parent().ok_or(Error::Invalid("worker engine root"))?;
    let mut paths = vec![
        config.python.clone(),
        config.rustc.clone(),
        config.validator.clone(),
    ];
    paths.extend(
        [
            "rust_guest_job.py",
            "rust_guest.py",
            "rust_guest_build.py",
            "rust_guest_source.py",
            "rust_guest_sandbox.py",
            "native_process.py",
        ]
        .map(|name| tools.join(name)),
    );
    fn sources(dir: &Path, paths: &mut Vec<PathBuf>) -> Result<()> {
        if paths.len() > 512 {
            return Err(Error::Invalid("SDK source inventory"));
        }
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            let info = fs::symlink_metadata(&path)?;
            if info.is_symlink() {
                return Err(Error::Invalid("SDK source link"));
            }
            if info.is_dir() {
                sources(&path, paths)?;
            } else if path.extension().is_some_and(|x| x == "rs") {
                paths.push(path);
            }
        }
        Ok(())
    }
    sources(&repo.join("sdk/rust/src"), &mut paths)?;
    sources(&repo.join("crates/incant_wasm_abi/src"), &mut paths)?;
    paths.push(repo.join("sdk/abi/native-v1.json"));
    paths.sort();
    paths.dedup();
    paths.into_iter().map(file_identity).collect()
}
impl Backend for Local {
    fn inspect(
        &self,
        root: &Path,
        package: &str,
        entry: &str,
        cancel: &dyn Fn() -> bool,
    ) -> Result<SourcePackage> {
        if self.inputs()? != self.identities {
            return Err(Error::Unavailable(
                "trusted worker/tool/SDK source changed".into(),
            ));
        }
        if self.config.storage.starts_with(root)
            || root.starts_with(&self.config.storage)
            || self
                .identities
                .iter()
                .any(|(path, _)| path.starts_with(root))
            || self
                .config_file
                .as_ref()
                .is_some_and(|path| path.starts_with(root))
        {
            return Err(Error::Invalid("project overlaps host authority/storage"));
        }
        let input =
            serde_json::json!({"mode":"inspect","root":root,"package":package,"entry":entry});
        let (_, reply) = self.execute(input, cancel)?;
        Ok(serde_json::from_value(reply)?)
    }
    fn build(&self, source: &SourcePackage, cancel: &dyn Fn() -> bool) -> Result<Built> {
        let (job, reply) =
            self.execute(serde_json::json!({"mode":"build","source":source}), cancel)?;
        let raw = bounded(&job.join("module.wasm"), 1024 * 1024)?;
        use sha2::{Digest, Sha256};
        if raw.get(..8) != Some(b"\0asm\x01\0\0\0")
            || reply["raw_sha256"] != format!("{:x}", Sha256::digest(&raw))
        {
            return Err(Error::Invalid("private raw candidate identity"));
        }
        let mut public = serde_json::Map::new();
        for key in [
            "compiler_version",
            "raw_sha256",
            "raw_bytes",
            "prepared_sha256",
            "policy_sha256",
            "sdk_sources",
            "service_sources",
            "host_tool_manifest_sha256",
            "host_closeout_sha256",
        ] {
            public.insert(
                key.into(),
                reply
                    .get(key)
                    .cloned()
                    .ok_or(Error::Invalid("missing worker provenance"))?,
            );
        }
        Ok(Built {
            diagnostics: String::new(),
            receipt: serde_json::Value::Object(public),
            candidate: job.join("module.wasm"),
        })
    }
}
fn bounded(path: &Path, cap: usize) -> Result<Vec<u8>> {
    let info = fs::symlink_metadata(path)?;
    if !info.is_file() || info.len() > cap as u64 {
        return Err(Error::Invalid("regular bounded protocol file required"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if info.nlink() != 1 {
            return Err(Error::Invalid("protocol hardlink"));
        }
    }
    let mut file = crate::node::read(path)?;
    let before = file.metadata()?;
    if !before.is_file() || before.len() > cap as u64 || !crate::node::matches(&file, path)? {
        return Err(Error::Invalid("protocol file replaced"));
    }
    let mut result = Vec::new();
    Read::by_ref(&mut file)
        .take((cap + 1) as u64)
        .read_to_end(&mut result)?;
    let after = file.metadata()?;
    if result.len() > cap
        || before.len() != after.len()
        || before.modified()? != after.modified()?
        || !crate::node::matches(&file, path)?
    {
        return Err(Error::Invalid("protocol/output byte bound or replacement"));
    }
    Ok(result)
}
#[cfg(any(target_os = "macos", test))]
fn write(path: &Path, value: &impl Serialize) -> Result<()> {
    use std::io::Write;
    if path.exists() {
        return Err(Error::Invalid("private protocol collision"));
    }
    let temporary = path.with_extension("pending");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    file.write_all(&serde_json::to_vec(value)?)?;
    file.sync_all()?;
    drop(file);
    // Private 0700 job, one trusted publisher per protocol path. Readers only
    // observe completed JSON, never a newly created empty response.
    fs::rename(temporary, path)?;
    Ok(())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg(any(target_os = "macos", test))]
struct Launch {
    argv: Vec<String>,
    cwd: PathBuf,
    env: BTreeMap<String, String>,
    stdout: PathBuf,
    stderr: PathBuf,
}
#[cfg(any(target_os = "macos", test))]
impl Launch {
    fn admit(
        &self,
        config: &Config,
        job: &Path,
        index: usize,
        source: Option<&SourcePackage>,
    ) -> Result<()> {
        crate::recipe::admit(&self.argv, config, job, source)?;
        if self.argv.len() > 128 || self.argv.iter().any(|x| x.len() > 4096 || x.contains('\0')) {
            return Err(Error::Invalid("worker argv bound"));
        }
        if !private_path(&self.cwd, job)?
            || self.stdout != job.join(format!("calls/{index:03}/stdout"))
            || self.stderr != job.join(format!("calls/{index:03}/stderr"))
        {
            return Err(Error::Invalid("worker private IO paths"));
        }
        let run = job.join("work/run");
        let expected = BTreeMap::from([
            (
                "PATH".into(),
                config
                    .rustc
                    .parent()
                    .ok_or(Error::Invalid("rustc parent"))?
                    .to_string_lossy()
                    .into_owned(),
            ),
            (
                "HOME".into(),
                run.join("home").to_string_lossy().into_owned(),
            ),
            ("TMPDIR".into(), format!("{}/", run.join("tmp").display())),
            ("LANG".into(), "C".into()),
            ("LC_ALL".into(), "C".into()),
            ("RAYON_NUM_THREADS".into(), "1".into()),
            ("SOURCE_DATE_EPOCH".into(), "0".into()),
        ]);
        if self.env != expected {
            return Err(Error::Invalid("worker environment authority"));
        }
        Ok(())
    }
}
#[cfg(any(target_os = "macos", test))]
fn private_path(path: &Path, root: &Path) -> Result<bool> {
    Ok(path.is_absolute() && path.starts_with(root) && path.canonicalize()? == path)
}
#[cfg(not(target_os = "macos"))]
impl Local {
    fn execute(
        &self,
        _: serde_json::Value,
        _: &dyn Fn() -> bool,
    ) -> Result<(PathBuf, serde_json::Value)> {
        Err(Error::Unavailable("experimental macOS backend only".into()))
    }
}
#[cfg(target_os = "macos")]
impl Local {
    fn execute(
        &self,
        mut input: serde_json::Value,
        cancel: &dyn Fn() -> bool,
    ) -> Result<(PathBuf, serde_json::Value)> {
        use crate::process::Owned;
        use std::{
            process::Command,
            time::{Duration, Instant},
        };
        if !self
            .quarantine
            .lock()
            .map_err(|_| Error::Cleanup("quarantine poisoned".into()))?
            .is_empty()
        {
            return Err(Error::Cleanup(
                "previous owned child still quarantined".into(),
            ));
        }
        if cancel() {
            return Err(Error::Cancelled);
        }
        if self.inputs()? != self.identities {
            return Err(Error::Unavailable(
                "trusted worker/tool/SDK source changed".into(),
            ));
        }
        if inventory(&self.config.storage)? > 512 * 1024 * 1024 {
            return Err(Error::Worker("retained host job storage full".into()));
        }
        let job = self.config.storage.join(ulid::Ulid::new().to_string());
        use std::os::unix::fs::DirBuilderExt;
        fs::DirBuilder::new().mode(0o700).create(&job)?;
        fs::create_dir(job.join("calls"))?;
        input["rustc"] = serde_json::json!(self.config.rustc);
        input["validator"] = serde_json::json!(self.config.validator);
        let source = input
            .get("source")
            .map(|value| serde_json::from_value::<SourcePackage>(value.clone()))
            .transpose()?;
        write(&job.join("request.json"), &input)?;
        let mut command = Command::new(&self.config.python);
        command
            .args(["-I"])
            .arg(&self.config.worker)
            .arg(&job)
            .current_dir(&job)
            .env_clear()
            .env("PATH", self.config.python.parent().unwrap())
            .env("HOME", &job)
            .env("TMPDIR", &job)
            .env("LANG", "C");
        let mut bridge = Owned::spawn(
            command,
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(job.join("stdout"))?,
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(job.join("stderr"))?,
        )?;
        let start = Instant::now();
        let mut index = 0;
        let mut active: Option<(Owned, Instant)> = None;
        let mut observed = std::collections::BTreeSet::new();
        let mut peak_rss = 0u64;
        let operation = (|| -> Result<serde_json::Value> {
            write(
                &job.join("owner.json"),
                &serde_json::json!({"pid":bridge.pid(),"scope":"module-build-host-bridge"}),
            )?;
            loop {
                crate::process::disk_floor(&job)?;
                let mut rows = crate::process::rss(bridge.pid())?;
                if let Some((child, _)) = &active {
                    rows.extend(crate::process::rss(child.pid())?);
                }
                let rss = rows.iter().map(|(_, bytes)| bytes).sum::<u64>();
                peak_rss = peak_rss.max(rss);
                observed.extend(rows.into_iter().map(|(pid, _)| pid));
                if rss > 8 * 1024 * 1024 * 1024 {
                    return Err(Error::Worker("sampled owned RSS cap".into()));
                }
                let diagnostics = (0..=index).try_fold(0u64, |sum, n| -> Result<u64> {
                    let dir = job.join(format!("calls/{n:03}"));
                    let mut bytes = sum;
                    for name in ["stdout", "stderr"] {
                        let path = dir.join(name);
                        if path.exists() {
                            bytes += bounded_size(&path)?;
                        }
                    }
                    Ok(bytes)
                })?;
                if diagnostics > 16 * 1024 * 1024 {
                    return Err(Error::Worker("combined child diagnostic cap".into()));
                }
                if cancel() {
                    return Err(Error::Cancelled);
                }
                if start.elapsed() > Duration::from_secs(180) {
                    return Err(Error::Worker("aggregate deadline".into()));
                }
                if inventory(&self.config.storage)? > 512 * 1024 * 1024
                    || bounded_size(&job.join("stdout"))? > 2 * 1024 * 1024
                    || bounded_size(&job.join("stderr"))? > 65536
                {
                    return Err(Error::Worker(
                        "global retained scratch/bridge output bound".into(),
                    ));
                }
                if let Some((child, started)) = &mut active {
                    let dir = job.join(format!("calls/{index:03}"));
                    if started.elapsed() > Duration::from_secs(90)
                        || bounded_size(&dir.join("stdout"))? > 4 * 1024 * 1024
                        || bounded_size(&dir.join("stderr"))? > 4 * 1024 * 1024
                        || dir.join("cancel").exists()
                    {
                        return Err(Error::Worker("child deadline/output/cancellation".into()));
                    }
                    if child.exited()? {
                        let mut ended = active.take().unwrap().0;
                        let exit = match ended.close() {
                            Ok(exit) => exit,
                            Err(error) => {
                                self.quarantine
                                    .lock()
                                    .map_err(|_| Error::Cleanup("quarantine poisoned".into()))?
                                    .push(ended);
                                return Err(error);
                            }
                        };
                        write(
                            &dir.join("response.json"),
                            &serde_json::json!({"pid":ended.pid(),"returncode":exit,"group_absent":true}),
                        )?;
                        index += 1;
                    }
                } else {
                    let dir = job.join(format!("calls/{index:03}"));
                    let request = dir.join("request.json");
                    if request.exists() {
                        if index >= 16 {
                            return Err(Error::Invalid("worker command count"));
                        }
                        let launch: Launch = serde_json::from_slice(&bounded(&request, 65536)?)?;
                        launch.admit(&self.config, &job, index, source.as_ref())?;
                        let mut command = Command::new(&launch.argv[0]);
                        command
                            .args(&launch.argv[1..])
                            .current_dir(launch.cwd)
                            .env_clear()
                            .envs(launch.env);
                        let child = Owned::spawn(
                            command,
                            OpenOptions::new()
                                .write(true)
                                .create_new(true)
                                .open(launch.stdout)?,
                            OpenOptions::new()
                                .write(true)
                                .create_new(true)
                                .open(launch.stderr)?,
                        )?;
                        let pid = child.pid();
                        active = Some((child, Instant::now()));
                        write(&dir.join("started.json"), &serde_json::json!({"pid":pid}))?;
                    }
                }
                if bridge.exited()? {
                    if active.is_some() {
                        return Err(Error::Worker("bridge ended with child active".into()));
                    }
                    let reply: serde_json::Value = serde_json::from_slice(&bounded(
                        &job.join("reply.json"),
                        2 * 1024 * 1024,
                    )?)?;
                    if reply["status"] != "complete" {
                        return Err(Error::Worker(bounded_diagnostics(
                            reply["untrusted_diagnostics"]
                                .as_str()
                                .unwrap_or("worker failed"),
                        )));
                    }
                    return Ok(reply["data"].clone());
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        })();
        // Explicitly close every owned group before returning data or an error.
        let child_close = active.as_mut().map(|(child, _)| child.close()).transpose();
        let bridge_close = bridge.close();
        if child_close.is_err() || bridge_close.is_err() {
            let mut quarantine = self
                .quarantine
                .lock()
                .map_err(|_| Error::Cleanup("quarantine poisoned".into()))?;
            if child_close.is_err()
                && let Some((child, _)) = active.take()
            {
                quarantine.push(child);
            }
            if bridge_close.is_err() {
                quarantine.push(bridge);
            }
        }
        let quarantined = !self
            .quarantine
            .lock()
            .map_err(|_| Error::Cleanup("quarantine poisoned".into()))?
            .is_empty();
        let close = serde_json::json!({"child_closed":child_close.is_ok()&&!quarantined,"bridge_closed":bridge_close.is_ok(),"bridge_exit":bridge_close.as_ref().ok(),"operation_error":operation.as_ref().err().map(ToString::to_string),"trusted_inputs":self.identities,"observed_pids":observed,"sampled_peak_rss_bytes":peak_rss});
        write(&job.join("closeout.json"), &close).map_err(|e| {
            Error::Cleanup(format!(
                "receipt: {e}; operation: {:?}",
                operation.as_ref().err()
            ))
        })?;
        if child_close.is_err() || bridge_close.is_err() || quarantined {
            return Err(Error::Cleanup(format!(
                "{child_close:?}; {bridge_close:?}; operation: {:?}",
                operation.as_ref().err()
            )));
        }
        let mut data = operation?;
        if bridge_close? != 0 {
            return Err(Error::Worker("bridge native exit".into()));
        }
        if self.inputs()? != self.identities {
            return Err(Error::Unavailable(
                "trusted worker/tool/SDK source changed".into(),
            ));
        }
        if input["mode"] == "build" {
            use sha2::{Digest, Sha256};
            data["host_tool_manifest_sha256"] = serde_json::json!(format!(
                "{:x}",
                Sha256::digest(serde_json::to_vec(&self.identities)?)
            ));
            data["host_closeout_sha256"] = serde_json::json!(format!(
                "{:x}",
                Sha256::digest(bounded(&job.join("closeout.json"), 1024 * 1024)?)
            ));
        }
        Ok((job, data))
    }
}
#[cfg(any(target_os = "macos", test))]
fn bounded_diagnostics(text: &str) -> String {
    let mut end = text.len().min(65536);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_owned()
}
#[cfg(target_os = "macos")]
fn bounded_size(path: &Path) -> Result<u64> {
    Ok(fs::symlink_metadata(path)?.len())
}
#[cfg(target_os = "macos")]
fn inventory(path: &Path) -> Result<u64> {
    fn visit(path: &Path, count: &mut usize) -> Result<u64> {
        *count += 1;
        if *count > 4096 {
            return Err(Error::Invalid("job inventory bound"));
        }
        let m = fs::symlink_metadata(path)?;
        if m.is_symlink() {
            return Err(Error::Invalid("job link"));
        }
        let mut n = m.len();
        if m.is_dir() {
            for x in fs::read_dir(path)? {
                n += visit(&x?.path(), count)?;
            }
        }
        Ok(n)
    }
    visit(path, &mut 0)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_launch_environment_and_private_receipts_are_not_worker_authority() {
        let root = tempfile::tempdir().unwrap();
        let job = root.path().canonicalize().unwrap();
        let config = Config {
            python: "/tools/python".into(),
            worker: "/engine/tools/rust_guest_job.py".into(),
            rustc: "/toolchain/bin/rustc".into(),
            validator: "/tools/validator".into(),
            storage: job.clone(),
        };
        let run = job.join("work/run");
        fs::create_dir_all(&run).unwrap();
        let mut launch = Launch {
            argv: vec!["/toolchain/bin/rustc".into(), "-vV".into()],
            cwd: run.clone(),
            stdout: job.join("calls/000/stdout"),
            stderr: job.join("calls/000/stderr"),
            env: BTreeMap::from([
                ("PATH".into(), "/toolchain/bin".into()),
                ("HOME".into(), run.join("home").display().to_string()),
                ("TMPDIR".into(), format!("{}/", run.join("tmp").display())),
                ("LANG".into(), "C".into()),
                ("LC_ALL".into(), "C".into()),
                ("RAYON_NUM_THREADS".into(), "1".into()),
                ("SOURCE_DATE_EPOCH".into(), "0".into()),
            ]),
        };
        assert!(launch.admit(&config, &job, 0, None).is_ok());
        launch.env.insert("RUSTC_WRAPPER".into(), "/bin/sh".into());
        assert!(launch.admit(&config, &job, 0, None).is_err());
        launch.env.remove("RUSTC_WRAPPER");
        launch.stdout = job.join("project.rs");
        assert!(launch.admit(&config, &job, 0, None).is_err());
        let request = job.join("request.json");
        write(&request, &serde_json::json!({"complete":true})).unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&bounded(&request, 1024).unwrap()).unwrap()
                ["complete"],
            true
        );
        assert!(write(&request, &serde_json::json!({"overwrite":true})).is_err());
        assert!(bounded(&request, 1).is_err());
    }
    #[test]
    fn diagnostics_never_exceed_byte_cap_at_multibyte_boundary() {
        for text in [
            "short".into(),
            "x".repeat(65536),
            format!("{}€", "x".repeat(65535)),
            "😀".repeat(65536),
        ] {
            let output = bounded_diagnostics(&text);
            assert!(output.len() <= 65536);
            assert!(text.starts_with(&output));
        }
    }
    #[cfg(all(target_os = "macos", feature = "host-qualification"))]
    #[test]
    #[ignore = "actual configured worker bridge; separately granted CPU qualification only"]
    fn actual_shared_bridge_build_and_cancellation() {
        use crate::{Actor, BuildRequest, Service, SourceQuery};
        use std::sync::atomic::{AtomicBool, Ordering};
        let backend = Local::from_environment().unwrap();
        let service = Service::new(backend);
        let root = tempfile::tempdir().unwrap();
        let package = root.path().join("logic");
        fs::create_dir(&package).unwrap();
        fs::write(package.join("module.rs"),"#![no_std]\nuse incant_module::{Tick,Result};\n#[panic_handler] fn panic(_: &core::panic::PanicInfo<'_>)->!{core::arch::wasm32::unreachable()}\nincant_module::module!(on_update);\nfn on_update(_: &mut Tick<'_>, _:f64)->Result<()>{Ok(())}\n").unwrap();
        let project = ulid::Ulid::new().to_string();
        service.bind(&project, root.path()).unwrap();
        let cancelled = AtomicBool::new(false);
        let source = service
            .inspect(
                &project,
                SourceQuery {
                    package: "logic".into(),
                    entry: "module.rs".into(),
                    expected_revision: 0,
                },
                &cancelled,
                &mut || Ok(()),
            )
            .unwrap();
        let request = BuildRequest {
            snapshot_id: source.snapshot_id.clone(),
            expected_revision: 0,
            description: "Fixed actual bridge qualification".into(),
        };
        let actor = Actor {
            origin: "user".into(),
            actor: "qualification".into(),
            model: None,
            conversation_id: None,
        };
        let result = service
            .build(
                &project,
                request.clone(),
                actor.clone(),
                &cancelled,
                &mut || Ok(()),
            )
            .unwrap();
        assert_eq!(result.source_digest, source.source_digest);
        assert_eq!(result.provenance["not_installed"], true);
        // Cancel after a real bridge/child has launched, rather than mocking Backend.
        let config: Config = serde_json::from_slice(
            &bounded(
                Path::new(&std::env::var_os("INCANT_MODULE_BUILD_CONFIG").unwrap()),
                65536,
            )
            .unwrap(),
        )
        .unwrap();
        let existing = fs::read_dir(&config.storage)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect::<std::collections::BTreeSet<_>>();
        std::thread::scope(|scope| {
            scope.spawn(|| {
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
                loop {
                    let launched = fs::read_dir(&config.storage)
                        .unwrap()
                        .filter_map(|e| e.ok())
                        .map(|e| e.path())
                        .any(|p| {
                            !existing.contains(&p) && p.join("calls/001/started.json").exists()
                        });
                    if launched {
                        cancelled.store(true, Ordering::Release);
                        break;
                    }
                    assert!(
                        std::time::Instant::now() < deadline,
                        "real delegated compiler was not observed"
                    );
                    std::thread::sleep(std::time::Duration::from_millis(2));
                }
            });
            assert!(matches!(
                service.build(&project, request, actor, &cancelled, &mut || Ok(())),
                Err(Error::Cancelled)
            ));
        });
        assert!(service.available());
        assert_eq!(
            service
                .status(
                    &project,
                    &crate::JobQuery {
                        snapshot_id: source.snapshot_id
                    }
                )
                .unwrap()
                .state,
            "cancelled"
        );
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1); // No project asset/history publication.
    }
}
