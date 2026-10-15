//! Explicit, externally supervised fresh-process Metal stages. No window/frame.
#![cfg(all(
    target_os = "macos",
    feature = "metal-binary-archives",
    feature = "native-compilation"
))]
#[path = "../src/pipeline_cache/darwin_acl.rs"]
mod darwin_acl;
#[path = "../src/pipeline_cache/metal_private.rs"]
mod metal_private;
use incant_render::{CacheRead, CacheWrite, Renderer, wgpu};
use sha2::{Digest, Sha256};
use std::{
    fs, io,
    path::{Path, PathBuf},
};
use wgpu_hal::metal::binary_archive::{Mode, Session};
const LIMIT: usize = 64 * 1024 * 1024;
const SHADER_A: &str = r#"
@vertex fn vertex(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
  return vec4f(f32(i), 0.0, 0.0, 1.0);
}
@fragment fn fragment() -> @location(0) vec4f { return vec4f(0.25,0.5,0.75,1.0); }
@group(0) @binding(0) var<storage,read_write> value: array<u32>;
@compute @workgroup_size(1) fn compute() { value[0] = 17u; }
"#;

#[test]
#[ignore = "explicit native Metal slot and external process/resource supervisor required"]
fn metal_archive_native_stage() {
    let stage = std::env::var("INCANT_METAL_ARCHIVE_STAGE").expect("explicit stage");
    let root =
        PathBuf::from(std::env::var_os("INCANT_METAL_ARCHIVE_ROOT").expect("private stage root"));
    if stage.starts_with("catalog-") {
        catalog(&stage, &root);
        return;
    }
    let directory = metal_private::Directory::open(&root, false).unwrap();
    let instance = instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
    let (device, _) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    // SAFETY: borrowed backend only; native owner remains the wgpu device.
    let hal = unsafe { device.as_hal::<wgpu::hal::api::Metal>() }.unwrap();
    let identity = format!(
        "{}:{}:{:?}",
        env!("INCANT_METAL_ARCHIVE_SOURCE"),
        hal.binary_archive_identity(),
        adapter.get_info()
    );
    eprintln!(
        "METAL_ARCHIVE_IDENTITY {}",
        serde_json::json!({"stage":stage,"identity":identity,"shader_sha256":format!("{:x}",Sha256::digest(SHADER_A))})
    );
    match stage.as_str() {
        "capture" => {
            assert!(
                !directory.path("fixture.metalarc").unwrap().exists(),
                "create-only capture"
            );
            // SAFETY: an empty archive contains no external native input.
            let archive = unsafe { hal.begin_binary_archive(None, Mode::Capture) }.unwrap();
            pipelines(&device, false, false);
            let counts = archive.counts();
            assert_eq!((counts.render_added, counts.compute_added), (1, 1));
            assert_eq!((counts.fallback_attempts, counts.addition_failures), (0, 0));
            let data = directory
                .export("fixture.metalarc", LIMIT, |path| {
                    // SAFETY: exclusive private temporary; export revalidates output.
                    unsafe { archive.serialize(path) }
                        .map_err(|e| io::Error::other(format!("{e:?}")))
                })
                .unwrap();
            let receipt = serde_json::to_vec(&serde_json::json!({"identity":identity,
                "bytes":data.len(),"sha256":format!("{:x}",Sha256::digest(&data))}))
            .unwrap();
            directory.publish("fixture.json", &receipt).unwrap();
            report(&stage, &archive);
        }
        "reuse" | "miss" => {
            let data = directory.read("fixture.metalarc", LIMIT).unwrap();
            let receipt: serde_json::Value =
                serde_json::from_slice(&directory.read("fixture.json", 4096).unwrap()).unwrap();
            assert_eq!(receipt["identity"], identity);
            assert_eq!(receipt["bytes"], data.len());
            assert_eq!(receipt["sha256"], format!("{:x}", Sha256::digest(data)));
            // The distinct B pipeline must first succeed ordinarily, warming
            // Metal's own app cache; that cannot substitute for archive membership.
            if stage == "miss" {
                pipelines(&device, true, false);
            }
            // SAFETY: only this fixture's bounded, owned, checksummed, identical
            // device/source archive is admitted through the checked URL tree.
            let archive = unsafe {
                hal.begin_binary_archive(
                    Some(&directory.path("fixture.metalarc").unwrap()),
                    Mode::RequireHit,
                )
            }
            .unwrap();
            pipelines(&device, stage == "miss", stage == "miss");
            let counts = archive.counts();
            assert_eq!(
                (
                    counts.render_added,
                    counts.compute_added,
                    counts.fallback_attempts
                ),
                (0, 0, 0)
            );
            assert_eq!((counts.render_attached, counts.compute_attached), (1, 1));
            assert_eq!(
                (counts.require_succeeded, counts.require_failed),
                if stage == "reuse" { (2, 0) } else { (0, 2) }
            );
            directory.validate().unwrap();
            report(&stage, &archive);
        }
        "disabled" => {
            // SAFETY: empty in-memory diagnostic archive, no input URL.
            let archive = unsafe { hal.begin_binary_archive(None, Mode::RequireHit) }.unwrap();
            archive.disable();
            pipelines(&device, false, true);
            let counts = archive.counts();
            assert_eq!(counts.disabled_require_refusals, 2);
            assert_eq!(
                (
                    counts.render_attached,
                    counts.compute_attached,
                    counts.require_succeeded,
                    counts.require_failed,
                    counts.fallback_attempts
                ),
                (0, 0, 0, 0, 0)
            );
            report(&stage, &archive);
        }
        "empty" => {
            pipelines(&device, false, false);
            // SAFETY: a fresh empty in-memory archive, no input bytes.
            let archive = unsafe { hal.begin_binary_archive(None, Mode::RequireHit) }.unwrap();
            pipelines(&device, false, true);
            assert_eq!(archive.counts().require_failed, 2);
            assert_eq!(
                archive.counts().render_added + archive.counts().compute_added,
                0
            );
            report(&stage, &archive);
        }
        _ => panic!("unplanned native stage {stage}"),
    }
}
fn instance() -> wgpu::Instance {
    wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    })
}
fn pipelines(device: &wgpu::Device, distinct: bool, must_fail: bool) {
    let source = if distinct {
        SHADER_A.replace("0.25", "0.875").replace("17u", "29u")
    } else {
        SHADER_A.into()
    };
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("fixed archive conformance fixture"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    // wgpu-core maps HAL Linkage (including native archive refusal) to
    // Internal. Ordinary validation errors must still fail this fixed fixture.
    let render_scope = device.push_error_scope(wgpu::ErrorFilter::Internal);
    let _render = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("fixed archive render"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vertex"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fragment"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8Unorm,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    });
    let render_error = pollster::block_on(render_scope.pop());
    assert_eq!(
        render_error.is_some(),
        must_fail,
        "render error: {render_error:?}"
    );
    let compute_scope = device.push_error_scope(wgpu::ErrorFilter::Internal);
    let _compute = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("fixed archive compute"),
        layout: None,
        module: &shader,
        entry_point: Some("compute"),
        compilation_options: Default::default(),
        cache: None,
    });
    let compute_error = pollster::block_on(compute_scope.pop());
    assert_eq!(
        compute_error.is_some(),
        must_fail,
        "compute error: {compute_error:?}"
    );
    eprintln!(
        "METAL_ARCHIVE_PIPELINES {}",
        serde_json::json!({"distinct":distinct,"require_failure":must_fail,
        "render_error":render_error.map(|e|e.to_string()),"compute_error":compute_error.map(|e|e.to_string())})
    );
}
fn report(stage: &str, archive: &Session) {
    eprintln!(
        "METAL_ARCHIVE_COUNTS {}",
        serde_json::json!({"stage":stage,"counts":format!("{:?}",archive.counts())})
    );
}
fn catalog(stage: &str, root: &Path) {
    let xdg =
        PathBuf::from(std::env::var_os("XDG_CACHE_HOME").expect("explicit private cache home"));
    assert_eq!(xdg, root.join("xdg"));
    if stage == "catalog-invalid" {
        let directory =
            metal_private::Directory::open(&xdg.join("incant/metal-pipelines-v1"), false).unwrap();
        let (raw, sidecar) = only_pair(&xdg);
        let raw = raw.file_name().unwrap().to_str().unwrap();
        let sidecar = sidecar.file_name().unwrap().to_str().unwrap();
        let mut header = directory.read(sidecar, 80).unwrap();
        assert_eq!(&header[..8], b"INCMARC1");
        // Fixed host-authored invalid bytes with an intact admission envelope:
        // distinguish actual native parser rejection from a checksum rejection.
        let invalid = b"Incant fixed invalid Metal archive control";
        header[40..48].copy_from_slice(&(invalid.len() as u64).to_le_bytes());
        header[48..80].copy_from_slice(&Sha256::digest(invalid));
        directory.publish(raw, invalid).unwrap();
        directory.publish(sidecar, &header).unwrap();
    }
    let strict = stage == "catalog-reuse";
    let strict_input = strict.then(|| {
        let (raw, metadata) = only_pair(&xdg);
        use std::os::unix::fs::MetadataExt;
        let stat = fs::metadata(&raw).unwrap();
        (
            fs::read(&raw).unwrap(),
            fs::read(metadata).unwrap(),
            stat.dev(),
            stat.ino(),
        )
    });
    let instance = instance();
    let mut renderer = if strict {
        pollster::block_on(Renderer::new_requiring_metal_archive(&instance)).unwrap()
    } else {
        pollster::block_on(Renderer::new(&instance, None)).unwrap()
    };
    assert_eq!(renderer.adapter.get_info().backend, wgpu::Backend::Metal);
    // SAFETY: scoped device borrow used only to read native device/OS identity.
    let native_identity = unsafe { renderer.device.as_hal::<wgpu::hal::api::Metal>() }
        .unwrap()
        .binary_archive_identity();
    eprintln!(
        "METAL_ARCHIVE_IDENTITY {}",
        serde_json::json!({
            "stage":stage, "source":env!("INCANT_METAL_ARCHIVE_SOURCE"),
            "native":native_identity, "adapter":format!("{:?}",renderer.adapter.get_info()),
            "require_hit":strict,
        })
    );
    let report = &renderer.pipeline_cache_report;
    match stage {
        "catalog-seed" => assert_eq!(report.read, CacheRead::Missing),
        "catalog-reuse" => assert!(matches!(report.read, CacheRead::Supplied { bytes: 1.. })),
        "catalog-invalid" => {
            assert_eq!(report.read, CacheRead::Rejected);
            assert!(report.metal.as_ref().unwrap().load_error.is_some());
        }
        _ => panic!("unplanned catalog stage {stage}"),
    }
    if strict {
        assert_eq!(report.write, CacheWrite::NotAttempted);
    } else {
        assert!(matches!(report.write, CacheWrite::Stored { bytes: 1.. }));
    }
    let before = report.metal.as_ref().unwrap().native.clone();
    if strict {
        strict_counts(&before);
        assert!(before.render_attached > 0 && before.compute_attached > 0);
    } else {
        assert!(before.render_added > 0 && before.compute_added > 0);
    }
    assert!(!before.disabled);
    pollster::block_on(renderer.enable_temporal_accumulation()).unwrap();
    let after = &renderer
        .pipeline_cache_report
        .metal
        .as_ref()
        .unwrap()
        .native;
    if strict {
        strict_counts(after);
        assert!(
            after.render_attached > before.render_attached
                && after.compute_attached > before.compute_attached
        );
        assert!(after.require_succeeded > before.require_succeeded);
        assert_eq!(
            renderer.pipeline_cache_report.write,
            CacheWrite::NotAttempted
        );
    } else {
        assert!(
            after.render_added > before.render_added && after.compute_added > before.compute_added
        );
    }
    assert_eq!(
        (
            after.fallback_attempts,
            after.addition_failures,
            after.serialization_failures
        ),
        (0, 0, 0)
    );
    assert!(!after.disabled);
    let (raw, metadata) = only_pair(&xdg);
    let bytes = (fs::read(&raw).unwrap(), fs::read(&metadata).unwrap());
    let held = fs::File::open(&raw).unwrap();
    use std::os::unix::fs::MetadataExt;
    let identity = (
        held.metadata().unwrap().dev(),
        held.metadata().unwrap().ino(),
    );
    if let Some((input_raw, input_metadata, dev, ino)) = strict_input {
        assert_eq!((&bytes.0, &bytes.1), (&input_raw, &input_metadata));
        assert_eq!(identity, (dev, ino));
    }
    assert!(
        pollster::block_on(renderer.enable_bloom(incant_render::Bloom {
            threshold: 1.,
            intensity: 0.5,
            levels: 0
        }))
        .is_err()
    );
    eprintln!(
        "METAL_ARCHIVE_CATALOG {}",
        serde_json::json!({"stage":stage,"report":format!("{:?}",renderer.pipeline_cache_report),
            "cache_identity":raw.file_stem().unwrap().to_str().unwrap(),
            "source":env!("INCANT_METAL_ARCHIVE_SOURCE"),"require_hit":strict})
    );
    drop(renderer);
    assert_eq!(
        (fs::read(&raw).unwrap(), fs::read(&metadata).unwrap()),
        bytes
    );
    assert_eq!(
        (
            fs::metadata(&raw).unwrap().dev(),
            fs::metadata(&raw).unwrap().ino()
        ),
        identity
    );
}
fn strict_counts(counts: &wgpu_hal::metal::binary_archive::Counts) {
    assert_eq!(
        (
            counts.render_added,
            counts.compute_added,
            counts.fallback_attempts
        ),
        (0, 0, 0)
    );
    assert_eq!(
        (counts.require_failed, counts.disabled_require_refusals),
        (0, 0)
    );
    assert_eq!(
        counts.require_succeeded,
        counts.render_attached + counts.compute_attached
    );
    assert!(counts.require_succeeded > 0 && !counts.disabled);
}
fn only_pair(xdg: &Path) -> (PathBuf, PathBuf) {
    let mut raw = Vec::new();
    let mut metadata = Vec::new();
    for entry in fs::read_dir(xdg.join("incant/metal-pipelines-v1")).unwrap() {
        let path = entry.unwrap().path();
        match path.extension().and_then(|s| s.to_str()) {
            Some("metalarc") => raw.push(path),
            Some("identity") => metadata.push(path),
            _ => panic!("unexpected archive directory entry"),
        }
    }
    assert_eq!((raw.len(), metadata.len()), (1, 1));
    (raw.pop().unwrap(), metadata.pop().unwrap())
}
