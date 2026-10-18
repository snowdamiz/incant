//! Explicit hosted diagnostic: only this owned disposable cache is corrupted.
//! Each real Node cook still uses the production command and eight-second bound.
use super::*;
use crate::typescript::{
    CookChild, DEADLINE, DEADLINE_MESSAGE, MAX_LOG_BYTES, TypeScriptCookError, bounded,
    command::CompilerPaths, node_executable, phases, wait_for_cook, windows::launch_directory,
};
use sha2::{Digest, Sha256};
use std::{io::Read, process::Command, time::Instant};

const SOURCE: &str = "import type { NativeBehavior } from '@incant/runtime/native';\nexport default { update() { const speed: number = 3; void speed; } } satisfies NativeBehavior;\n";

fn cook(base: &Path, project: &Path, cache: &Cache, name: &str) -> Vec<Vec<u8>> {
    let directory = base.join(name);
    std::fs::create_dir(&directory).unwrap();
    let engine = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let node = node_executable(project).unwrap();
    let output = directory.join("cooked");
    let phase_file = directory.join("phases.jsonl");
    let stdout = directory.join("stdout");
    let stderr = directory.join("stderr");
    let input = directory.join("input");
    std::fs::write(
        &input,
        r#"[{"id":"00000000000000000000000001","path":"main.ts"}]"#,
    )
    .unwrap();
    File::create(&phase_file).unwrap();
    let launch = launch_directory(&engine).unwrap();
    let mut command = Command::new(&node);
    // These would break dependency resolution if the production helper leaked
    // ambient settings. They are configured on this child, never global env.
    for key in [
        "NODE_OPTIONS",
        "NODE_PATH",
        "SWC_BINARY_PATH",
        "SWC_NATIVE_BINDING_CACHE",
        "LOCALAPPDATA",
    ] {
        command.env(key, "invalid ambient override must be discarded");
    }
    CompilerPaths {
        node: &node,
        engine: &launch,
        project,
        output: &output,
        temporary: &directory,
        phases: &phase_file,
        cache: cache.path(),
    }
    .configure(&mut command);
    command
        .stdin(File::open(&input).unwrap())
        .stdout(File::create(&stdout).unwrap())
        .stderr(File::create(&stderr).unwrap());
    let mut child = CookChild(command.spawn().unwrap());
    let started = Instant::now();
    let result = wait_for_cook(&mut child, started, DEADLINE, &stdout, &stderr, &phase_file);
    drop(child); // Kill/reap before inspecting or corrupting any cached file.
    let outcome = if matches!(&result, Err(TypeScriptCookError::Invalid(DEADLINE_MESSAGE))) {
        "deadline"
    } else if result.as_ref().is_ok_and(|s| s.success()) {
        "exited_success"
    } else {
        "refused"
    };
    let observation = phases::observation(&phase_file, started.elapsed(), outcome);
    eprintln!(
        "INCANT_SWC_CACHE_PHASES {}",
        serde_json::json!({"case":name,
        "observation":observation})
    );
    let status = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(
        status.success(),
        "{name}: {}",
        String::from_utf8_lossy(&bounded(&stderr, MAX_LOG_BYTES as usize).unwrap())
    );
    assert_eq!(observation["state"], "valid_prefix");
    assert_eq!(observation["phases"].as_array().unwrap().len(), 10);
    let files = [
        bounded(
            &output.join("behavior.cjs"),
            incant_script::artifact::MAX_SOURCE_BYTES,
        )
        .unwrap(),
        bounded(&output.join("behavior.cjs.map"), 4 * 1024 * 1024).unwrap(),
        bounded(&output.join("manifest.json"), 128 * 1024).unwrap(),
    ];
    let manifest: serde_json::Value = serde_json::from_slice(&files[2]).unwrap();
    assert_eq!(manifest["format"], "incant-native-typescript-project");
    assert_eq!(
        manifest["bundleSha256"],
        format!("{:x}", Sha256::digest(&files[0]))
    );
    assert_eq!(
        manifest["mapSha256"],
        format!("{:x}", Sha256::digest(&files[1]))
    );
    files.into()
}
fn image(cache: &Path) -> PathBuf {
    let entries: Vec<_> = std::fs::read_dir(cache)
        .unwrap()
        .take(2)
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(entries.len(), 1, "one user namespace only in owned cache");
    let namespace = &entries[0];
    assert!(namespace.file_type().unwrap().is_dir());
    assert!(
        namespace
            .file_name()
            .to_string_lossy()
            .starts_with("swc-native-")
    );
    let version = namespace.path().join("v1");
    let mut images = Vec::new();
    let entries: Vec<_> = std::fs::read_dir(&version)
        .unwrap()
        .take(9)
        .collect::<Result<_, _>>()
        .unwrap();
    assert!(entries.len() <= 8, "bounded disposable cache inventory");
    for entry in entries {
        if entry.path().extension().is_some_and(|v| v == "node") {
            assert!(entry.file_type().unwrap().is_file());
            let name = entry.file_name();
            let key = Path::new(&name).file_stem().unwrap().to_string_lossy();
            assert!(key.len() == 128 && key.bytes().all(|b| b.is_ascii_hexdigit()));
            images.push(entry.path());
        }
    }
    assert_eq!(
        images.len(),
        1,
        "native carrier must materialize one actual image"
    );
    images.remove(0)
}
fn digest(path: &Path) -> String {
    let mut file = File::open(path).unwrap();
    assert!((1..=512 * 1024 * 1024).contains(&file.metadata().unwrap().len()));
    let length = file.metadata().unwrap().len();
    let mut bytes_read = 0u64;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buffer).unwrap();
        if n == 0 {
            break;
        }
        bytes_read += n as u64;
        assert!(
            bytes_read <= length,
            "native cache file grew during verification"
        );
        hash.update(&buffer[..n]);
    }
    assert_eq!(bytes_read, length);
    format!("{:x}", hash.finalize())
}

fn image_identity(path: &Path) -> (u32, u32, u32) {
    let file = File::open(path).unwrap();
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    assert_ne!(
        unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) },
        0
    );
    (
        info.dwVolumeSerialNumber,
        info.nFileIndexHigh,
        info.nFileIndexLow,
    )
}

#[test]
#[ignore = "explicit hosted Windows cold/warm/corrupt carrier diagnostic"]
fn windows_dependency_cache_cold_warm_corrupt_preserves_fresh_compiler_outputs() {
    let base = tempfile::tempdir().unwrap();
    let host = base.path().join("private-host");
    let project = base.path().join("project");
    std::fs::create_dir(&host).unwrap();
    std::fs::create_dir(&project).unwrap();
    std::fs::write(project.join("main.ts"), SOURCE).unwrap();
    let project = project.canonicalize().unwrap();
    // Private injection only; production dependency_cache always queries the OS.
    let cache = resolve(&host, &project).unwrap();
    assert!(!cache.path().exists());
    let cold = cook(base.path(), &project, &cache, "cold");
    let raw = image(cache.path());
    let first_hash = digest(&raw);
    let before = std::fs::metadata(&raw).unwrap();
    let cold_identity = image_identity(&raw);
    let warm = cook(base.path(), &project, &cache, "warm");
    assert_eq!(cold, warm, "exact bundle, map and manifest parity");
    assert_eq!(image(cache.path()), raw);
    assert_eq!(image_identity(&raw), cold_identity);
    assert_eq!(digest(&raw), first_hash);
    assert_eq!(
        std::fs::metadata(&raw).unwrap().modified().unwrap(),
        before.modified().unwrap(),
        "warm image must be reused"
    );
    // All child processes have exited; corrupt only the discovered owned image.
    assert!(raw.starts_with(host.canonicalize().unwrap()));
    std::fs::write(&raw, b"deliberately truncated diagnostic image").unwrap();
    let repaired = cook(base.path(), &project, &cache, "corrupt-repair");
    assert_eq!(cold, repaired);
    assert_eq!(
        digest(&raw),
        first_hash,
        "carrier must repair with exact verified installed bytes"
    );
    std::fs::write(project.join("main.ts"), SOURCE.replace("= 3;", "= 7;")).unwrap();
    let changed = cook(base.path(), &project, &cache, "changed-source");
    assert_ne!(
        cold[0], changed[0],
        "shared native dependency must not cache project output"
    );
    assert_ne!(cold[2], changed[2]);
    assert_eq!(digest(&raw), first_hash);
    assert_eq!(std::fs::read_dir(&project).unwrap().count(), 1);
    eprintln!(
        "INCANT_SWC_CACHE_RESULT {}",
        serde_json::json!({"version":1,"cases":4,
        "cold_warm_repair_artifact_parity":true,"changed_source_recompiled":true,
        "native_image_sha256":first_hash,"deadline_seconds":8,"normal_user_cache_modified":false})
    );
    drop(cache);
    base.close().unwrap();
}
