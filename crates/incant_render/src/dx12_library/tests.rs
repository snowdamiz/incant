//! Externally supervised Windows-only stages. Each stage uses a fresh process;
//! no process launch, window or screen capture occurs in the test itself.
use crate::{CacheRead, CacheWrite, Renderer, SceneMsaa, pipeline_cache};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write as _,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use wgpu_hal::dx12::pipeline_library::{Config, Mode, RenderPath};

fn counts(c: &wgpu_hal::dx12::pipeline_library::Counts) -> serde_json::Value {
    serde_json::json!({
        "stream_hits": c.stream_hits, "classic_hits": c.classic_hits,
        "compute_hits": c.compute_hits, "miss_or_mismatch": c.miss_or_mismatch,
        "native_load_errors": c.native_load_errors, "fallback_attempts": c.fallback_attempts,
        "stores": c.stores, "store_errors": c.store_errors,
        "strict_refusals": c.strict_refusals, "serialization_errors": c.serialization_errors,
        "disabled": c.disabled, "overflowed": c.overflowed,
        "last_error": c.last_error.as_ref().map(|error| format!("{error:?}")),
    })
}

#[test]
#[ignore = "requires an explicit Windows DX12 device/storage lease and external supervisor"]
fn dx12_library_native_stage() {
    let stage = std::env::var("INCANT_DX12_LIBRARY_STAGE").expect("explicit stage");
    assert!(matches!(
        stage.as_str(),
        "seed"
            | "reuse-stream"
            | "reuse-classic"
            | "missing"
            | "corrupt"
            | "disabled"
            | "empty"
            | "lifetime"
    ));
    let root = PathBuf::from(std::env::var_os("INCANT_DX12_LIBRARY_ROOT").expect("owned root"));
    assert!(root.is_absolute());
    let directory = root.join("cache");
    let output = root.join(&stage);
    fs::create_dir(&output).unwrap(); // No stage overwrite/retry.
    let instance = crate::wgpu::Instance::new(crate::wgpu::InstanceDescriptor {
        backends: crate::wgpu::Backends::DX12,
        ..crate::wgpu::InstanceDescriptor::new_without_display_handle()
    });
    if stage == "lifetime" {
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let (device, _) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let observed = Arc::new(Mutex::new(None));
        let (host, read) = pipeline_cache::dx12::Session::configured(
            &device,
            &adapter,
            pipeline_cache::dx12::Options {
                directory: Some(directory),
                observed: Some(Arc::clone(&observed)),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(matches!(read, CacheRead::Supplied { bytes: 1.. }));
        let library = observed.lock().unwrap().take().unwrap();
        let (retained, life) = library.retain_for_lifetime_control();
        let final_ref = retained.clone();
        // No PSO creation in this fresh device/session: the real library has a
        // nonempty admitted input and only our explicit COM test clones remain.
        drop(host);
        drop(library);
        assert!(life.is_alive());
        drop(retained);
        assert!(life.is_alive());
        drop(final_ref);
        assert!(
            !life.is_alive(),
            "backing must retire after the final COM library ref"
        );
        fs::write(
            output.join("result.txt"),
            "real COM retained refs pin immutable backing; final ref frees it",
        )
        .unwrap();
        return;
    }
    if stage == "empty" {
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let (device, _) =
            pollster::block_on(adapter.request_device(&crate::wgpu::DeviceDescriptor {
                required_features: crate::output::required_features(&adapter).unwrap(),
                ..Default::default()
            }))
            .unwrap();
        let hal = unsafe { device.as_hal::<crate::wgpu::hal::api::Dx12>() }.unwrap();
        // A new trusted in-memory empty library, with strict mode enforced at
        // the actual native compute PSO call. No supplied cache or project data.
        let library = unsafe {
            hal.begin_pipeline_library(
                Vec::new(),
                Config {
                    mode: Mode::RequireHits,
                    ..Default::default()
                },
            )
        }
        .unwrap();
        let scope = device.push_error_scope(crate::wgpu::ErrorFilter::Internal);
        let _candidate =
            crate::lighting::LightingSystem::new(&device, crate::wgpu::Backend::Dx12, None);
        assert!(pollster::block_on(scope.pop()).is_some());
        let counts = library.counts();
        assert_eq!((counts.miss_or_mismatch, counts.strict_refusals), (1, 1));
        assert_eq!(
            (counts.fallback_attempts, counts.stores, counts.compute_hits),
            (0, 0, 0)
        );
        assert_eq!(
            library.serialize(),
            Err(wgpu_hal::dx12::pipeline_library::Error::Disabled)
        );
        fs::write(output.join("result.txt"), format!("{counts:?}")).unwrap();
        return;
    }
    if stage == "missing" {
        assert!(!directory.exists());
    }
    if stage == "corrupt" {
        let path = directory.join("current.library");
        let mut bytes = fs::read(&path).unwrap();
        *bytes.last_mut().unwrap() ^= 1;
        fs::write(&path, bytes).unwrap();
    }
    let observed = Arc::new(Mutex::new(None));
    let strict = stage != "seed";
    let before = strict.then(|| fs::read(directory.join("current.library")).ok());
    let result = pollster::block_on(Renderer::new_configured(
        &instance,
        None,
        false,
        SceneMsaa::One,
        pipeline_cache::dx12::Options {
            config: Config {
                mode: if strict {
                    Mode::RequireHits
                } else {
                    Mode::Capture
                },
                render_path: if stage == "reuse-classic" {
                    RenderPath::Classic
                } else {
                    RenderPath::Automatic
                },
            },
            directory: Some(directory.clone()),
            observed: Some(Arc::clone(&observed)),
        },
    ));
    if matches!(stage.as_str(), "missing" | "corrupt") {
        let error = result
            .err()
            .expect("unsafe/missing strict input must refuse");
        assert!(error.downcast_ref::<crate::Dx12LibraryError>().is_some());
        assert!(observed.lock().unwrap().is_none());
        assert_eq!(
            fs::read(directory.join("current.library")).ok(),
            before.unwrap()
        );
        fs::write(output.join("refusal.txt"), error.to_string()).unwrap();
        return;
    }
    let mut renderer = result.unwrap(); // Unsupported is a failure, never a hit/skip.
    assert_eq!(
        renderer.adapter.get_info().backend,
        crate::wgpu::Backend::Dx12
    );
    let initial = renderer.pipeline_cache_report.clone();
    let counts = &initial.dx12.as_ref().unwrap().native;
    if strict {
        assert!(matches!(initial.read, CacheRead::Supplied { bytes: 1.. }));
        assert_eq!(initial.write, CacheWrite::NotAttempted);
        assert_eq!(
            (
                counts.miss_or_mismatch,
                counts.native_load_errors,
                counts.fallback_attempts,
                counts.stores
            ),
            (0, 0, 0, 0)
        );
        assert!(counts.compute_hits > 0);
        if stage == "reuse-classic" {
            assert!(counts.classic_hits > 0);
            assert_eq!(counts.stream_hits, 0);
        } else {
            assert!(counts.stream_hits > 0);
            assert_eq!(counts.classic_hits, 0);
        }
    } else {
        assert_eq!(initial.read, CacheRead::Missing);
        assert!(matches!(initial.write, CacheWrite::Stored { bytes: 1.. }));
        assert!(counts.fallback_attempts > 0 && counts.fallback_attempts == counts.stores);
        assert_eq!(
            (counts.stream_hits, counts.classic_hits, counts.compute_hits),
            (0, 0, 0)
        );
        assert_eq!(counts.fallback_attempts, counts.miss_or_mismatch);
    }
    assert_eq!(
        (
            counts.store_errors,
            counts.serialization_errors,
            counts.strict_refusals
        ),
        (0, 0, 0)
    );
    assert!(!counts.disabled && !counts.overflowed);
    let input = crate::scene_msaa::fixture::mixed(1.);
    let pixels = crate::scene_msaa::fixture::draw(&renderer, &input, [160, 96], None);
    // Retain bytes before any numerical assertions. External comparison requires
    // exact seed/reuse/classic and ordinary/cooked RGBA, without tolerance.
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output.join("initial.rgba"))
        .unwrap()
        .write_all(&pixels)
        .unwrap();
    assert_eq!(pixels.len(), 160 * 96 * 4);
    assert!(pixels.as_chunks::<4>().0.iter().all(|p| p[3] == 255));
    for channel in 0..3 {
        assert!(
            pixels
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|p| p[channel] > 150
                    && (0..3)
                        .filter(|&c| c != channel)
                        .all(|c| p[channel].saturating_sub(p[c]) > 80))
                .count()
                > 50
        );
    }
    let library = observed.lock().unwrap().clone().unwrap();
    if strict {
        assert_eq!(
            library.serialize(),
            Err(wgpu_hal::dx12::pipeline_library::Error::Disabled)
        );
    }
    if stage == "disabled" {
        library.disable();
    }
    // Seed only the initial finite catalog. Optional bloom has not been stored;
    // the strict later cold call must refuse and retain prior draw resources.
    if strict {
        let stores = library.counts().stores;
        assert!(
            pollster::block_on(renderer.enable_bloom(crate::Bloom {
                threshold: 1.0,
                intensity: 0.25,
                levels: 3,
            }))
            .is_err()
        );
        let after = library.counts();
        assert!(after.strict_refusals > 0);
        assert_eq!((after.fallback_attempts, after.stores), (0, stores));
        let retained = crate::scene_msaa::fixture::draw(&renderer, &input, [160, 96], None);
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output.join("refused.rgba"))
            .unwrap()
            .write_all(&retained)
            .unwrap();
        assert_eq!(retained, pixels);
        assert_eq!(
            fs::read(directory.join("current.library")).ok(),
            before.unwrap()
        );
    }
    renderer.check_device().unwrap();
    let native_identity = unsafe { renderer.device.as_hal::<crate::wgpu::hal::api::Dx12>() }
        .unwrap()
        .pipeline_library_identity()
        .unwrap();
    let report = serde_json::json!({"stage":stage,"backend":"Dx12",
        "source":env!("INCANT_DX12_LIBRARY_SOURCE"), "adapter":format!("{:?}",renderer.adapter.get_info()),
        "native_identity": native_identity,
        "cooked":cfg!(feature="cooked-native-shaders"), "rgba_sha256":format!("{:x}",Sha256::digest(&pixels)),
        "counts_before": self::counts(counts), "counts_after": self::counts(&library.counts()),
        "initial_report":format!("{initial:?}"),"final_native":format!("{:?}",library.counts()),
        "scope":"native library membership and exact raw parity; no window/appearance/performance gate"});
    fs::write(
        output.join("result.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
}
