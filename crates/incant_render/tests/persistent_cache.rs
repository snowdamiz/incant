//! Explicit Linux/Vulkan persistence proof. No frame, readback, or cache-hit claim.
#![cfg(all(unix, feature = "persistent-pipeline-cache"))]
#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

use incant_render::{CacheRead, CacheWrite, Renderer, wgpu};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

const TEST_NAME: &str = "vulkan_catalog_cache_survives_fresh_processes_and_corruption";
const CHILD_STAGE: &str = "INCANT_PIPELINE_CACHE_TEST_STAGE";
const ENVELOPE_HEADER_BYTES: u64 = 80;
const MAX_CACHE_BYTES: u64 = 64 * 1024 * 1024;
const CHILD_TIMEOUT: Duration = Duration::from_secs(120);

#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires Linux Vulkan with PIPELINE_CACHE; explicitly requested runs must not skip"]
fn vulkan_catalog_cache_survives_fresh_processes_and_corruption() {
    lifecycle();
}

// Compile the complete harness on other Unix hosts too, while keeping it out of
// their generic --ignored GPU suites. Any future direct invocation fails closed.
fn lifecycle() {
    assert_eq!(std::env::consts::OS, "linux", "Linux Vulkan proof required");
    if let Ok(stage) = std::env::var(CHILD_STAGE) {
        initialize_catalog(&stage);
        return;
    }

    // Retain failure logs without ever printing the opaque driver cache. Only
    // this test-created directory is removed after every child has been reaped.
    let root = tempfile::Builder::new()
        .prefix("incant-vulkan-cache-")
        .tempdir()
        .unwrap()
        .keep();
    let cache_home = root.join("xdg-cache");
    run_child(&root, &cache_home, "cold", "missing").unwrap();
    let cache = only_cache(&cache_home);
    run_child(&root, &cache_home, "reopened", "supplied").unwrap();
    assert_eq!(only_cache(&cache_home), cache, "adapter identity changed");

    // Change one checksum byte, leaving driver bytes entirely untouched. The
    // envelope must be rejected before unsafe driver-cache admission is reached.
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&cache)
        .unwrap();
    file.seek(SeekFrom::Start(48)).unwrap();
    let mut byte = [0];
    file.read_exact(&mut byte).unwrap();
    byte[0] ^= 1;
    file.seek(SeekFrom::Start(48)).unwrap();
    file.write_all(&byte).unwrap();
    file.sync_all().unwrap();
    drop(file);

    run_child(&root, &cache_home, "corrupt", "rejected").unwrap();
    assert_eq!(only_cache(&cache_home), cache, "recovery changed identity");
    run_child(&root, &cache_home, "recovered", "supplied").unwrap();
    assert_eq!(only_cache(&cache_home), cache);
    fs::remove_dir_all(&root).unwrap();
}

fn initialize_catalog(stage: &str) {
    // Do not use environment-selected backends or silently substitute GL/Metal.
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let renderer = pollster::block_on(Renderer::new(&instance, None))
        .expect("requested real Vulkan Renderer must initialize");
    eprintln!("Adapter: {:?}", renderer.adapter.get_info());
    assert_eq!(renderer.adapter.get_info().backend, wgpu::Backend::Vulkan);
    assert!(
        renderer
            .device
            .features()
            .contains(wgpu::Features::PIPELINE_CACHE),
        "requested Vulkan adapter must support pipeline-cache persistence"
    );
    match (stage, &renderer.pipeline_cache_report.read) {
        ("missing", CacheRead::Missing) | ("rejected", CacheRead::Rejected) => {}
        ("supplied", CacheRead::Supplied { bytes }) => {
            assert!((1..=MAX_CACHE_BYTES as usize).contains(bytes));
        }
        (_, report) => panic!("unexpected {stage} cache-read report: {report:?}"),
    }
    match renderer.pipeline_cache_report.write {
        CacheWrite::Stored { bytes } => {
            assert!((1..=MAX_CACHE_BYTES as usize).contains(&bytes));
        }
        ref report => panic!("startup cache must be persisted: {report:?}"),
    }

    let report = instance
        .generate_report()
        .expect("native allocation report");
    // 12 materials + 8 shadows + diagnostic + 4 output + sprite + text + 3 UI.
    assert_eq!(report.hub.render_pipelines.num_allocated, 30);
    assert_eq!(report.hub.compute_pipelines.num_allocated, 1);
    renderer.check_device().unwrap();
    let after_poll = instance.generate_report().unwrap();
    assert_eq!(after_poll.hub.render_pipelines, report.hub.render_pipelines);
    assert_eq!(
        after_poll.hub.compute_pipelines,
        report.hub.compute_pipelines
    );
    // These are counts and public status only, never serialized cache bytes.
    eprintln!(
        "Vulkan catalog: 30 render + 1 compute; {:?}",
        renderer.pipeline_cache_report
    );
}

fn only_cache(cache_home: &Path) -> PathBuf {
    let entries: Vec<_> = fs::read_dir(cache_home.join("incant/pipelines-v1"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(entries.len(), 1, "one cache and no abandoned staging files");
    let path = &entries[0];
    assert_eq!(path.extension().and_then(|s| s.to_str()), Some("cache"));
    let metadata = path.symlink_metadata().unwrap();
    assert!(metadata.file_type().is_file());
    assert!(
        (ENVELOPE_HEADER_BYTES + 1..=ENVELOPE_HEADER_BYTES + MAX_CACHE_BYTES)
            .contains(&metadata.len())
    );
    path.clone()
}

fn run_child(root: &Path, cache_home: &Path, label: &str, stage: &str) -> io::Result<()> {
    let log_path = root.join(format!("{label}.log"));
    let log = File::create(&log_path)?;
    let child = Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            TEST_NAME,
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD_STAGE, stage)
        .env("XDG_CACHE_HOME", cache_home)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log))
        .spawn()?;
    let mut owned = OwnedChild(Some(child));
    let start = Instant::now();
    loop {
        if let Some(status) = owned.0.as_mut().unwrap().try_wait()? {
            owned.0.take(); // try_wait reaped it; Drop must not signal that PID.
            // Keep the public adapter/status evidence in CI on success and failure.
            // The child never emits the opaque driver bytes; bound even panic logs.
            let mut output = String::new();
            File::open(&log_path)?
                .take(64 * 1024)
                .read_to_string(&mut output)?;
            println!("{label} child ({status}):\n{output}");
            return if status.success() {
                Ok(())
            } else {
                Err(io::Error::other(format!(
                    "{label} child failed ({status}); retained log: {}",
                    log_path.display()
                )))
            };
        }
        if start.elapsed() >= CHILD_TIMEOUT {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!(
                    "{label} child timed out; retained log: {}",
                    log_path.display()
                ),
            )); // OwnedChild kills and reaps even on this error path.
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

struct OwnedChild(Option<Child>);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
